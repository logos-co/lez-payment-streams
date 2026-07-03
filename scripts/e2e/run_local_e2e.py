#!/usr/bin/env python3
"""Step 17 local dual-host Store + eligibility E2E orchestrator."""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
import uuid
from pathlib import Path
from typing import Any, Callable


# Static sharding config - simpler for E2E demo without autosharding complexity
# Autosharding requires content topic format: /app/{version}/content-type/encoding
# where {version} is numeric. The content topic hash determines the shard.
CONTENT_TOPIC = "/lez-payment-streams/1/e2e-eligibility/proto"
# Content topic hashes to shard 1 with 8 shards (cluster 0)
PUBSUB_TOPIC = "/waku/2/rs/0/1"
N8_REFERENCE_QUERY = {
    "requestId": "req-1",
    "includeData": True,
    "pubsubTopic": PUBSUB_TOPIC,
    "contentTopics": [CONTENT_TOPIC],
    "timeStart": 10,
    "paginationForward": True,
    "paginationLimit": 100,
    "messageHashes": [],
}
STORE_QUERY_TIMEOUT_S = 120
PUBLISH_WAIT_S = 15
PEER_MESH_WAIT_S = 45
STORE_QUERY_RETRIES = 4
DAEMON_START_WAIT_S = 6
# Provider verify rejects streams with zero unaccrued allocation; accrual runs between
# prepare and verify, so proof must be minted immediately before storeQuery.
def manifest_allocation_lo(manifest: dict, default: int = 200) -> int:
    if manifest.get("allocation") is not None:
        return int(manifest["allocation"])
    legacy = manifest.get("stream_allocation")
    if legacy is not None:
        return int(legacy)
    return default


def set_manifest_allocation(manifest: dict, value: int) -> None:
    manifest["allocation"] = int(value)
    manifest.pop("stream_allocation", None)


def min_unaccrued_lo_for_proof(manifest: dict) -> int:
    alloc = manifest_allocation_lo(manifest)
    return max(64, min(alloc // 4, 50_000))


def store_reuse_baseline_vault() -> bool:
    return os.environ.get("E2E_REUSE_BASELINE_VAULT", "0").strip() == "1"


def vault_config_is_empty_from_probe(read_ok: bool) -> bool:
    """True when the vault config account is missing or has no data."""
    return not read_ok


def scan_first_empty_vault_id(
    is_empty_for_id: Callable[[int], bool], start: int = 0, limit: int = 100_000
) -> int:
    for vault_id in range(start, start + limit):
        if is_empty_for_id(vault_id):
            return vault_id
    raise E2EError(f"no empty vault id found in scan range [{start}, {start + limit})")


def testnet_e2e_create_via() -> str:
    explicit = os.environ.get("E2E_CREATE_VIA", "").strip().lower()
    if explicit:
        return explicit
    return "chainaction"


def stream_fundable_wait_s() -> int:
    raw = os.environ.get("E2E_STREAM_FUNDABLE_WAIT_S", "30").strip()
    try:
        return max(1, int(raw))
    except ValueError:
        return 30


def stream_fundable_poll_s() -> float:
    raw = os.environ.get("E2E_STREAM_FUNDABLE_POLL_S", "1").strip()
    try:
        return max(0.25, float(raw))
    except ValueError:
        return 1.0


def e2e_subprocess_timeout_s() -> int:
    raw = os.environ.get("E2E_SUBPROC_TIMEOUT_S", "600").strip()
    try:
        return max(30, int(raw))
    except ValueError:
        return 600


def wallet_tx_poll_budget_s(wallet_config_path: Path) -> dict[str, Any]:
    """Upper bound when TxPoller never sees the tx (exponential backoff, capped at seq_poll_timeout)."""
    poll_cap_s = 12.0
    max_attempts = 5
    max_retries = 5
    if wallet_config_path.is_file():
        try:
            data = json.loads(wallet_config_path.read_text())
            max_attempts = max(1, int(data.get("seq_tx_poll_max_blocks", max_attempts)))
            max_retries = int(data.get("seq_poll_max_retries", max_retries))
            raw = data.get("seq_poll_timeout", "12s")
            if isinstance(raw, str) and raw.endswith("s"):
                poll_cap_s = float(raw[:-1])
            elif isinstance(raw, (int, float)):
                poll_cap_s = float(raw)
        except (json.JSONDecodeError, TypeError, ValueError):
            pass
    min_delay_s = 0.25
    delay = min_delay_s
    worst_s = 0.0
    for _ in range(max(0, max_attempts - 1)):
        worst_s += delay
        delay = min(delay * 2.0, poll_cap_s)
    return {
        "seq_poll_timeout_s": poll_cap_s,
        "seq_tx_poll_max_blocks": max_attempts,
        "seq_poll_max_retries": max_retries,
        "tx_poll_worst_case_s": round(worst_s, 1),
        "tx_poll_backoff": "exp_min_250ms_cap_seq_poll_timeout",
        "wallet_config": str(wallet_config_path),
    }


def apply_e2e_wallet_poll_overrides(wallet_home: Path) -> None:
    """Tune copied seed wallet for fast localnet confirm (does not touch repo .scaffold/wallet)."""
    cfg_path = wallet_home / "wallet_config.json"
    if not cfg_path.is_file():
        return
    cap = os.environ.get("E2E_WALLET_POLL_MAX_DELAY", "8s").strip()
    attempts_raw = os.environ.get("E2E_WALLET_POLL_MAX_ATTEMPTS", "22").strip()
    try:
        max_attempts = max(5, int(attempts_raw))
    except ValueError:
        max_attempts = 22
    data = json.loads(cfg_path.read_text())
    data["seq_poll_timeout"] = cap
    data["seq_tx_poll_max_blocks"] = max_attempts
    cfg_path.write_text(json.dumps(data, indent=4) + "\n")


class RunTimer:
    def __init__(self, artifact: Path) -> None:
        self.artifact = artifact
        self.t0 = time.monotonic()

    def mark(self, label: str) -> None:
        log_artifact(
            self.artifact,
            "timing_mark",
            True,
            label=label,
            elapsed_s=round(time.monotonic() - self.t0, 2),
        )


class Narrator:
    """Console narrative printer with three verbosity levels.

    quiet:   JSON-lines to artifact file only, no console output.
    normal:  Phase headers, status markers, on-chain values.
    verbose: Full narrative with inline concept explanations.
    """

    def __init__(self, verbosity: str = "verbose") -> None:
        self.verbosity = verbosity
        self._chain_label = "LocalNet" if os.environ.get("CHAIN", "local").strip().lower() == "local" else "TestNet"

    def _ts(self) -> str:
        return time.strftime("%H:%M:%S")

    def _emit(self, level: str, text: str) -> None:
        if self.verbosity == "quiet":
            return
        if level == "verbose" and self.verbosity != "verbose":
            return
        print(f"[{self._ts()}] {text}", file=sys.stderr)

    def header(self, scenario: str) -> None:
        if self.verbosity == "quiet":
            return
        line = "=" * 44
        print(f"\n[{self._ts()}] {line}", file=sys.stderr)
        print(f"[{self._ts()}] Payment Streams E2E: Developer Journey ({self._chain_label})", file=sys.stderr)
        print(f"[{self._ts()}] {scenario}", file=sys.stderr)
        print(f"[{self._ts()}] {line}", file=sys.stderr)

    def phase(self, name: str) -> None:
        self._emit("always", "")
        self._emit("always", f"PHASE: {name}")

    def step(self, text: str) -> None:
        self._emit("always", f"  → {text}")

    def ok(self, text: str) -> None:
        self._emit("always", f"  ✓ {text}")

    def fail(self, text: str) -> None:
        self._emit("always", f"  ✗ {text}")

    def value(self, text: str) -> None:
        self._emit("always", f"    {text}")

    def concept(self, text: str) -> None:
        self._emit("verbose", f"    {text}")

    def hint(self, text: str) -> None:
        self._emit("always", f"  ! Hint: {text}")

    def complete(self, artifact: Path) -> None:
        if self.verbosity == "quiet":
            return
        line = "=" * 44
        print(f"\n[{self._ts()}] {line}", file=sys.stderr)
        print(f"[{self._ts()}] E2E COMPLETE: All phases succeeded", file=sys.stderr)
        print(f"[{self._ts()}] Artifact: {artifact}", file=sys.stderr)
        print(f"[{self._ts()}] {line}", file=sys.stderr)

    def complete_fail(self, artifact: Path, failures: int) -> None:
        if self.verbosity == "quiet":
            return
        line = "=" * 44
        print(f"\n[{self._ts()}] {line}", file=sys.stderr)
        print(f"[{self._ts()}] E2E FAILED: {failures} phase(s) failed", file=sys.stderr)
        print(f"[{self._ts()}] Artifact: {artifact}", file=sys.stderr)
        print(f"[{self._ts()}] {line}", file=sys.stderr)


narrator = Narrator(os.environ.get("E2E_VERBOSITY", "verbose"))


_PER_RUN_STREAM_MANIFEST_KEYS = ("stream_id", "stream_config_account_id")


def strip_snapshot_stream_fields(manifest: dict, manifest_path: Path) -> None:
    """Step 24c: ignore any stream fields left on disk; stream id is chosen per run from chain."""
    for key in _PER_RUN_STREAM_MANIFEST_KEYS:
        manifest.pop(key, None)
    if not manifest_path.is_file():
        return
    try:
        data = json.loads(manifest_path.read_text())
    except json.JSONDecodeError:
        return
    dirty = any(k in data for k in _PER_RUN_STREAM_MANIFEST_KEYS)
    if not dirty:
        return
    for key in _PER_RUN_STREAM_MANIFEST_KEYS:
        data.pop(key, None)
    manifest_path.write_text(json.dumps(data, indent=2) + "\n")


def reset_payment_streams_module_persist(persist_user: Path, persist_provider: Path) -> None:
    """Drop module inventory/negotiation so listMyStreams reflects this run's stream only."""
    for root in (persist_user, persist_provider):
        if not root.exists():
            continue
        for state_file in root.glob("**/payment_streams_state.json"):
            state_file.unlink(missing_ok=True)


def list_my_streams_inner(cfg: Path, vault_id: int) -> dict[str, Any]:
    r = logoscore_cmd(cfg, "call", "payment_streams_module", "listMyStreams", str(vault_id))
    parsed = call_result(r)
    inner_raw = parsed.get("result")
    if isinstance(inner_raw, str):
        try:
            return json.loads(inner_raw)
        except json.JSONDecodeError:
            return {"status": "error", "parse_error": True, "raw": inner_raw[:500]}
    return inner_raw if isinstance(inner_raw, dict) else {"status": "error", "unexpected_result": True}


def check_stream_fundable(
    cfg: Path, vault_id: int, stream_id: int, manifest: dict
) -> dict[str, Any]:
    """Return fundability diagnosis (orchestrator gate before stream_proof prepare)."""
    min_unaccrued = min_unaccrued_lo_for_proof(manifest)
    inner = list_my_streams_inner(cfg, vault_id)
    if inner.get("status") != "ok":
        return {
            "fundable": False,
            "reason": "list_my_streams_failed",
            "min_unaccrued_lo": min_unaccrued,
            "stream_id": stream_id,
            "list_response": inner,
        }
    streams = inner.get("streams") or []
    listed_ids = [int(row.get("stream_id", -1)) for row in streams if isinstance(row, dict)]
    row = next(
        (r for r in streams if isinstance(r, dict) and int(r.get("stream_id", -1)) == stream_id),
        None,
    )
    if row is None:
        return {
            "fundable": False,
            "reason": "stream_not_in_inventory",
            "min_unaccrued_lo": min_unaccrued,
            "stream_id": stream_id,
            "listed_stream_ids": listed_ids,
            "inventory_count": len(streams),
        }
    unaccrued = int(row.get("unaccrued_lo", 0))
    if unaccrued < min_unaccrued:
        accrued = int(row.get("accrued_lo", 0))
        alloc = manifest_allocation_lo(manifest, default=80)
        sub_reason = "unaccrued_below_min"
        if unaccrued == 0 and accrued > 0:
            sub_reason = "stream_fully_accrued_or_depleted"
        elif unaccrued == 0 and int(row.get("stream_state", -1)) == 2:
            sub_reason = "stream_closed_on_chain"
        return {
            "fundable": False,
            "reason": sub_reason,
            "min_unaccrued_lo": min_unaccrued,
            "stream_id": stream_id,
            "unaccrued_lo": unaccrued,
            "accrued_lo": accrued,
            "fixture_allocation": alloc,
            "stream_row": row,
            "listed_stream_ids": listed_ids,
        }
    return {
        "fundable": True,
        "reason": "ok",
        "min_unaccrued_lo": min_unaccrued,
        "stream_id": stream_id,
        "unaccrued_lo": unaccrued,
        "stream_row": row,
    }


def default_topup_increase_lo(manifest: dict) -> int:
    alloc = manifest_allocation_lo(manifest)
    return max(100, alloc // 2)


class E2EError(Exception):
    pass


def log_artifact(artifact: Path, phase: str, ok: bool, **fields: Any) -> None:
    row = {"phase": phase, "ok": ok, **fields}
    with artifact.open("a") as f:
        f.write(json.dumps(row, separators=(",", ":")) + "\n")


def emit_module_phase(artifact: Path, phase: str, ok: bool, extra: dict[str, Any]) -> None:
    row = {"phase": phase, "ok": ok, "extra": extra}
    with artifact.open("a") as f:
        f.write(json.dumps(row, separators=(",", ":")) + "\n")


def emit_claim_with_demo_alias(artifact: Path, ok: bool, extra: dict[str, Any]) -> None:
    emit_module_phase(artifact, "claim", ok, extra)
    emit_module_phase(artifact, "demo_claim", ok, extra)


def run(cmd: list[str], *, cwd: Path | None = None, env: dict | None = None, timeout: int = 600) -> subprocess.CompletedProcess:
    merged = os.environ.copy()
    if env:
        merged.update(env)
    return subprocess.run(
        cmd,
        cwd=cwd,
        env=merged,
        text=True,
        capture_output=True,
        timeout=timeout,
        check=False,
    )


def logoscore_cmd(cfg_dir: Path, *args: str, timeout: int = 120) -> subprocess.CompletedProcess:
    cmd = ["logoscore", "--config-dir", str(cfg_dir), *args]
    return run(cmd, timeout=timeout)


def testnet_chain_action_timeout_s() -> int:
    if os.environ.get("CHAIN", "local").strip().lower() != "testnet":
        return 120
    raw = os.environ.get("LOGOSCORE_CHAIN_ACTION_TIMEOUT", "360").strip()
    try:
        return max(120, int(raw))
    except ValueError:
        return 360


def last_json_line(text: str) -> dict | None:
    for line in reversed(text.splitlines()):
        line = line.strip()
        if line.startswith("{"):
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                continue
    return None


def call_result(res: subprocess.CompletedProcess) -> dict:
    if res.returncode != 0 and not res.stdout and not res.stderr:
        raise E2EError(f"logoscore failed rc={res.returncode}: {res.stderr}")
    parsed = last_json_line(res.stdout) or last_json_line(res.stderr)
    if not parsed:
        raise E2EError(f"no JSON in logoscore output stdout={res.stdout!r} stderr={res.stderr!r}")
    return parsed


def module_call_value(parsed: dict, context: str) -> Any:
    if parsed.get("status") != "ok":
        raise E2EError(f"{context}: {json.dumps(parsed)}")
    result = parsed.get("result")
    if isinstance(result, str):
        try:
            result = json.loads(result)
        except json.JSONDecodeError:
            return result
    if isinstance(result, dict) and "success" in result:
        if not result.get("success"):
            raise E2EError(f"{context}: {result.get('error') or result}")
        return result.get("value")
    return result


def ensure_ok(parsed: dict, context: str) -> dict:
    val = module_call_value(parsed, context)
    if isinstance(val, dict):
        return val
    return {"raw": val}


def sequencer_block_height(sequencer_url: str) -> int | None:
    repo = Path(os.environ.get("REPO", Path.cwd()))
    helper = repo / "scripts" / "testnet_rpc.py"
    if helper.is_file():
        proc = run(
            ["python3", str(helper), "block-height"],
            env={**os.environ, "TESTNET_SEQUENCER": sequencer_url},
            timeout=30,
        )
        if proc.returncode == 0 and proc.stdout.strip().isdigit():
            return int(proc.stdout.strip())
    proc = run(
        [
            "curl",
            "-sf",
            "-X",
            "POST",
            sequencer_url,
            "-H",
            "Content-Type: application/json",
            "-d",
            '{"jsonrpc":"2.0","id":1,"method":"get_last_block","params":{}}',
        ],
        timeout=30,
    )
    if proc.returncode != 0 or not proc.stdout.strip():
        proc = run(
            [
                "curl",
                "-sf",
                "-X",
                "POST",
                sequencer_url,
                "-H",
                "Content-Type: application/json",
                "-d",
                '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}',
            ],
            timeout=30,
        )
        if proc.returncode != 0 or not proc.stdout.strip():
            return None
    try:
        data = json.loads(proc.stdout)
        result = data.get("result")
        if isinstance(result, dict) and "last_block" in result:
            return int(result["last_block"])
        if isinstance(result, int):
            return result
        if isinstance(result, str) and result.isdigit():
            return int(result)
    except json.JSONDecodeError:
        return None
    return None


def sync_wallet(cfg: Path, sequencer_url: str) -> None:
    height = sequencer_block_height(sequencer_url)
    if height is None:
        return
    logoscore_cmd(cfg, "call", "logos_execution_zone", "sync_to_block", str(height))
    time.sleep(2)


def allocation_available(cfg: Path, vault_id: int, stream_id: int, manifest: dict) -> bool:
    return check_stream_fundable(cfg, vault_id, stream_id, manifest)["fundable"]


def wait_for_stream_fundable(
    cfg_user: Path,
    vault_id: int,
    stream_id: int,
    manifest: dict,
    seq_url: str,
    artifact: Path,
) -> None:
    wait_s = stream_fundable_wait_s()
    poll_s = stream_fundable_poll_s()
    max_attempts = max(1, int(wait_s / poll_s))
    last_check: dict[str, Any] = {}
    t0 = time.monotonic()
    for attempt in range(max_attempts):
        sync_wallet(cfg_user, seq_url)
        logoscore_cmd(cfg_user, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))
        last_check = check_stream_fundable(cfg_user, vault_id, stream_id, manifest)
        last_check["attempt"] = attempt + 1
        last_check["elapsed_s"] = round(time.monotonic() - t0, 2)
        if last_check["fundable"]:
            log_artifact(
                artifact,
                "wait_stream_fundable",
                True,
                stream_id=stream_id,
                attempts=attempt + 1,
                elapsed_s=last_check["elapsed_s"],
                unaccrued_lo=last_check.get("unaccrued_lo"),
                min_unaccrued_lo=last_check.get("min_unaccrued_lo"),
            )
            return
        time.sleep(poll_s)

    block = sequencer_block_height(seq_url)
    chain_hint: dict[str, Any] = {}
    pda = manifest.get("stream_config_account_id")
    if pda:
        try:
            r = logoscore_cmd(
                cfg_user,
                "call",
                "payment_streams_module",
                "readStreamConfigDecoded",
                str(pda),
            )
            parsed = call_result(r)
            chain_hint["readStreamConfigDecoded"] = parsed.get("result")
        except E2EError as exc:
            chain_hint["readStreamConfigDecoded_error"] = str(exc)

    log_artifact(
        artifact,
        "wait_stream_fundable",
        False,
        stream_id=stream_id,
        wait_s=wait_s,
        poll_s=poll_s,
        attempts=max_attempts,
        elapsed_s=round(time.monotonic() - t0, 2),
        last_check=last_check,
        sequencer_block=block,
        chain_hint=chain_hint,
    )
    reason = last_check.get("reason", "unknown")
    raise E2EError(
        f"stream not fundable after create_demo_stream: {reason} "
        f"(waited {wait_s}s, stream_id={stream_id}, detail={json.dumps(last_check, default=str)[:1200]})"
    )


def stream_listed(cfg: Path, vault_id: int, stream_id: int) -> bool:
    r = logoscore_cmd(cfg, "call", "payment_streams_module", "listMyStreams", str(vault_id))
    parsed = call_result(r)
    inner_raw = parsed.get("result")
    if isinstance(inner_raw, str):
        inner = json.loads(inner_raw)
    else:
        inner = inner_raw if isinstance(inner_raw, dict) else {}
    if inner.get("status") != "ok":
        return False
    for row in inner.get("streams", []):
        if int(row.get("stream_id", -1)) == stream_id:
            return True
    return False


def stop_daemon(cfg: Path) -> None:
    run(["logoscore", "--config-dir", str(cfg), "stop"], timeout=30)


def start_daemon(cfg: Path, modules: Path, persist: Path) -> None:
    stop_daemon(cfg)
    time.sleep(1)
    daemon_env = os.environ.copy()
    depleted = os.environ.get("PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF", "").strip().lower()
    if depleted in ("1", "true", "yes"):
        daemon_env["PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF"] = "1"
    persist.mkdir(parents=True, exist_ok=True)
    stderr_path = persist / "logoscore-daemon.stderr"
    stderr_file = stderr_path.open("a")
    proc = subprocess.Popen(
        [
            "logoscore",
            "--config-dir",
            str(cfg),
            "-D",
            "-m",
            str(modules),
            "--persistence-path",
            str(persist),
        ],
        stdout=stderr_file,
        stderr=stderr_file,
        env=daemon_env,
        cwd=str(Path(os.environ.get("REPO", "."))),
    )
    stderr_file.close()
    time.sleep(DAEMON_START_WAIT_S)
    if proc.poll() is not None:
        raise E2EError(f"logoscore daemon exited early for {cfg}")


def load_modules(cfg: Path) -> None:
    for name in ("logos_execution_zone", "payment_streams_module", "delivery_module"):
        r = logoscore_cmd(cfg, "load-module", name)
        parsed = call_result(r)
        if parsed.get("status") != "ok":
            raise E2EError(f"load-module {name}: {parsed}")
    time.sleep(2)


def open_wallet(cfg: Path, wallet_config: Path, wallet_storage: Path) -> None:
    r = logoscore_cmd(
        cfg,
        "call",
        "logos_execution_zone",
        "open",
        str(wallet_config),
        str(wallet_storage),
    )
    parsed = call_result(r)
    if parsed.get("status") != "ok":
        raise E2EError(f"wallet open: {parsed}")


NODE_STARTED_TIMEOUT_S = 120


def local_waku_json_base() -> dict:
    return {
        "clusterId": 0,
        "entryNodes": [],
        "nat": "extip:127.0.0.1",
        "discv5Discovery": False,
        "dnsDiscovery": False,
        "logLevel": "INFO",
        "tcpPort": 60000,
        "listenAddress": "127.0.0.1",
        # Autosharding required for subscribe/send APIs
        "numShardsInNetwork": 8,
    }


def loopback_multiaddr(peer_id: str, tcp_port: int) -> str:
    return f"/ip4/127.0.0.1/tcp/{tcp_port}/p2p/{peer_id}"


def wait_for_delivery_event(
    cfg: Path,
    event_name: str,
    log_path: Path,
    *,
    timeout_s: int = NODE_STARTED_TIMEOUT_S,
) -> dict:
    log_path.parent.mkdir(parents=True, exist_ok=True)
    if log_path.is_file():
        log_path.unlink()
    watch = subprocess.Popen(
        ["logoscore", "--config-dir", str(cfg), "watch", "delivery_module"],
        stdout=log_path.open("w"),
        stderr=subprocess.STDOUT,
        text=True,
        start_new_session=True,
    )
    deadline = time.time() + timeout_s
    try:
        while time.time() < deadline:
            if log_path.is_file():
                for line in log_path.read_text().splitlines():
                    line = line.strip()
                    if not line or event_name not in line:
                        continue
                    try:
                        obj = json.loads(line)
                    except json.JSONDecodeError:
                        continue
                    if obj.get("event") != event_name:
                        continue
                    if event_name == "nodeStarted":
                        data = obj.get("data") if isinstance(obj.get("data"), dict) else {}
                        ok = data.get("arg0", True)
                        if ok is False:
                            msg = data.get("arg1", "")
                            raise E2EError(f"nodeStarted failed: {msg}")
                    return obj
            time.sleep(0.5)
        tail = log_path.read_text()[-2000:] if log_path.is_file() else ""
        raise E2EError(f"timeout waiting for {event_name}; log tail={tail!r}")
    finally:
        watch.terminate()
        try:
            watch.wait(timeout=5)
        except subprocess.TimeoutExpired:
            watch.kill()


def delivery_create_start(cfg: Path, create_json: dict, *, persist: Path, label: str) -> None:
    payload = json.dumps(create_json, separators=(",", ":"))
    r = logoscore_cmd(cfg, "call", "delivery_module", "createNode", payload)
    ensure_ok(call_result(r), f"{label} createNode")
    event_log = persist / f"delivery-watch-{label}.log"
    watch = subprocess.Popen(
        ["logoscore", "--config-dir", str(cfg), "watch", "delivery_module"],
        stdout=event_log.open("w"),
        stderr=subprocess.STDOUT,
        text=True,
        start_new_session=True,
    )
    time.sleep(0.5)
    try:
        r = logoscore_cmd(cfg, "call", "delivery_module", "start")
        ensure_ok(call_result(r), f"{label} start")
        deadline = time.time() + NODE_STARTED_TIMEOUT_S
        while time.time() < deadline:
            if event_log.is_file():
                text = event_log.read_text()
                if "nodeStarted" in text:
                    for line in text.splitlines():
                        line = line.strip()
                        if "nodeStarted" not in line:
                            continue
                        try:
                            obj = json.loads(line)
                        except json.JSONDecodeError:
                            continue
                        if obj.get("event") != "nodeStarted":
                            continue
                        data = obj.get("data") if isinstance(obj.get("data"), dict) else {}
                        if data.get("arg0") is False:
                            raise E2EError(f"{label} nodeStarted failed: {data.get('arg1', '')}")
                        return
            time.sleep(0.5)
        tail = event_log.read_text()[-2000:] if event_log.is_file() else ""
        raise E2EError(f"{label} nodeStarted timeout; log={tail!r}")
    finally:
        watch.terminate()
        try:
            watch.wait(timeout=5)
        except subprocess.TimeoutExpired:
            watch.kill()


def set_eligibility_verifier(cfg: Path, module_name: str) -> None:
    r = logoscore_cmd(cfg, "call", "delivery_module", "setEligibilityVerifier", module_name)
    parsed = call_result(r)
    module_call_value(parsed, "setEligibilityVerifier")


def set_eligibility_provider(cfg: Path, module_name: str) -> None:
    r = logoscore_cmd(cfg, "call", "delivery_module", "setEligibilityProvider", module_name)
    parsed = call_result(r)
    if not module_name:
        # clearing may return success without value wrapper
        if parsed.get("status") == "ok":
            return
    module_call_value(parsed, "setEligibilityProvider")


def get_node_info(cfg: Path, item: str) -> str:
    r = logoscore_cmd(cfg, "call", "delivery_module", "getNodeInfo", item)
    parsed = call_result(r)
    val = module_call_value(parsed, f"getNodeInfo {item}")
    return str(val).strip()


def pick_multiaddr(addrs_csv: str, peer_id: str, tcp_port: int) -> str:
    parts = [p.strip() for p in addrs_csv.split(",") if p.strip()]
    for part in parts:
        if "/p2p/" in part and ("127.0.0.1" in part or "/ip4/127." in part):
            return part
    for part in parts:
        if f"/tcp/{tcp_port}" in part:
            if "/ip4/" in part and not part.endswith(f"/p2p/{peer_id}"):
                return f"{part}/p2p/{peer_id}"
            return part
    return f"/ip4/127.0.0.1/tcp/{tcp_port}/p2p/{peer_id}"


def seed_provider_session_from_user(
    persist_user: Path,
    persist_provider: Path,
    manifest_path: Path,
    repo: Path,
) -> None:
    user_state = find_ps_state_file(persist_user)
    try:
        provider_state = find_ps_state_file(persist_provider)
    except E2EError:
        # The provider's payment_streams_module has not written a state file yet
        # (no stateful call before the seed step). Seed into the module's
        # instance-specific persist dir so the upcoming reload picks it up;
        # seed_provider_acceptance.py creates the file if missing.
        provider_state = provider_ps_state_path(persist_provider)
    seed = run(
        [
            sys.executable,
            str(repo / "scripts/e2e/seed_provider_acceptance.py"),
            "--user-state",
            str(user_state),
            "--provider-state",
            str(provider_state),
            "--manifest",
            str(manifest_path),
        ],
        timeout=30,
    )
    if seed.returncode != 0:
        raise E2EError(seed.stderr or seed.stdout)


def reload_provider_payment_streams_module(cfg_provider: Path) -> None:
    logoscore_cmd(cfg_provider, "unload-module", "payment_streams_module")
    logoscore_cmd(cfg_provider, "load-module", "payment_streams_module")


def reload_payment_streams_wallet(cfg: Path, seq_url: str) -> None:
    sync_wallet(cfg, seq_url)
    logoscore_cmd(cfg, "unload-module", "payment_streams_module")
    logoscore_cmd(cfg, "load-module", "payment_streams_module")
    wc = os.environ.get("WALLET_CONFIG", "")
    ws = os.environ.get("WALLET_STORAGE", "")
    if wc and ws:
        logoscore_cmd(cfg, "call", "logos_execution_zone", "open", wc, ws)
    sync_wallet(cfg, seq_url)


def vault_next_stream_id(cfg: Path, manifest: dict) -> int:
    inner = vault_status_json(cfg, manifest)
    vault_cfg = inner.get("vault_config") if isinstance(inner.get("vault_config"), dict) else {}
    if "next_stream_id" in vault_cfg:
        return int(vault_cfg["next_stream_id"])
    return 0


def vault_status_json(cfg: Path, manifest: dict) -> dict[str, Any]:
    body = json.dumps(
        {
            "owner": manifest["owner_account_id"],
            "vault_id": int(manifest.get("vault_id", 0)),
        }
    )
    r = logoscore_cmd(cfg, "call", "payment_streams_module", "chainAction", "getVaultStatus", body)
    parsed = call_result(r)
    inner_raw = parsed.get("result")
    if isinstance(inner_raw, str):
        try:
            return json.loads(inner_raw)
        except json.JSONDecodeError as exc:
            raise E2EError(f"getVaultStatus returned non-JSON: {inner_raw[:200]}") from exc
    return inner_raw if isinstance(inner_raw, dict) else {}


def vault_unallocated_lo(cfg: Path, manifest: dict) -> int:
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    holding_id = str(manifest.get("vault_holding_account_id", "")).strip()
    if not holding_id:
        raise E2EError("manifest missing vault_holding_account_id for unallocated check")
    holding_acc = sequencer_json_rpc(seq_url, "getAccount", [holding_id])
    holding_bal = int((holding_acc or {}).get("balance", 0) or 0)
    inner = vault_status_json(cfg, manifest)
    vault_cfg = inner.get("vault_config") if isinstance(inner.get("vault_config"), dict) else {}
    total_lo = int(vault_cfg.get("total_allocated_lo", 0) or 0)
    return max(0, holding_bal - total_lo)


def release_logoscore_wallet(cfg: Path) -> None:
    logoscore_cmd(cfg, "call", "logos_execution_zone", "close")


def reopen_logoscore_wallet(cfg: Path, seq_url: str) -> None:
    wc = os.environ.get("WALLET_CONFIG", "")
    ws = os.environ.get("WALLET_STORAGE", "")
    if wc and ws:
        logoscore_cmd(cfg, "call", "logos_execution_zone", "open", wc, ws)
    sync_wallet(cfg, seq_url)


def seed_vault_deposit_onchain(
    repo: Path,
    manifest: dict,
    deposit_amount: int,
    cfg: Path | None = None,
) -> None:
    guest = Path(os.environ["PAYMENT_STREAMS_GUEST_BIN"])
    owner = manifest.get("owner_account_id", "")
    state_file = repo / ".lez_payment_streams-state"
    if state_file.is_file():
        for line in state_file.read_text().splitlines():
            if line.startswith("SIGNER_ID="):
                owner = line.split("=", 1)[1].strip().strip("'\"")
                break
    wallet_home = Path(os.environ.get("LEE_WALLET_HOME_DIR", repo / ".scaffold" / "wallet"))
    apply_e2e_wallet_poll_overrides(wallet_home)
    env = os.environ.copy()
    env["LEE_WALLET_HOME_DIR"] = str(wallet_home)
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    vault_id = int(manifest.get("vault_id", 0))
    if cfg is not None:
        release_logoscore_wallet(cfg)
    try:
        proc = run(
            [
                "cargo",
                "run",
                "-q",
                "--manifest-path",
                "examples/Cargo.toml",
                "--bin",
                "seed_localnet_fixture",
                "--",
                "deposit-onchain",
                "--program-bin",
                str(guest),
                "--owner",
                owner,
                "--vault-id",
                str(vault_id),
                "--deposit-amount",
                str(deposit_amount),
                "--sequencer-url",
                seq_url,
            ],
            cwd=repo,
            env=env,
            timeout=300,
        )
        if proc.returncode != 0:
            raise E2EError(f"seed vault deposit failed: {proc.stderr or proc.stdout}")
    finally:
        if cfg is not None:
            reopen_logoscore_wallet(cfg, seq_url)
            reload_payment_streams_wallet(cfg, seq_url)


def e2e_tx_onchain_wait_s(wallet_config_path: Path | None = None) -> int:
    explicit = os.environ.get("E2E_TX_ONCHAIN_WAIT_S", "").strip()
    if explicit:
        try:
            return max(30, int(explicit))
        except ValueError:
            pass
    worst = 90.0
    wc = wallet_config_path
    if wc is None:
        raw = os.environ.get("WALLET_CONFIG", "").strip()
        if raw:
            wc = Path(raw)
    if wc is not None and wc.is_file():
        worst = max(worst, float(wallet_tx_poll_budget_s(wc).get("tx_poll_worst_case_s", worst)))
    return int(worst) + 20


def chain_action_result_inner(parsed: dict[str, Any]) -> dict[str, Any]:
    inner_raw = parsed.get("result")
    if isinstance(inner_raw, str):
        try:
            return json.loads(inner_raw)
        except json.JSONDecodeError:
            return {}
    return inner_raw if isinstance(inner_raw, dict) else {}


def chain_action_tx_hash(parsed: dict[str, Any]) -> str | None:
    inner = chain_action_result_inner(parsed)
    h = inner.get("tx_hash") or inner.get("txHash")
    if isinstance(h, str) and h.strip():
        return h.strip()
    wallet = inner.get("wallet")
    if isinstance(wallet, dict):
        wh = wallet.get("tx_hash") or wallet.get("txHash")
        if isinstance(wh, str) and wh.strip():
            return wh.strip()
    return None


def chain_action_success(parsed: dict[str, Any]) -> bool:
    if parsed.get("status") != "ok":
        return False
    inner = chain_action_result_inner(parsed)
    if inner.get("success") is False:
        return False
    if inner.get("status") == "error":
        return False
    return True


def await_chain_action_inclusion(
    seq_url: str,
    parsed: dict[str, Any],
    artifact: Path,
    *,
    label: str,
) -> None:
    inner = chain_action_result_inner(parsed)
    wallet_obj = inner.get("wallet") if isinstance(inner.get("wallet"), dict) else {}
    if wallet_obj.get("success") is True and os.environ.get("E2E_STRICT_SEQUENCER_TX_WAIT", "").strip() != "1":
        log_artifact(
            artifact,
            "wait_tx_on_chain",
            True,
            label=label,
            skipped=True,
            reason="wallet_submit_success",
            tx_hash=chain_action_tx_hash(parsed),
        )
        return
    tx_hash = chain_action_tx_hash(parsed)
    if not tx_hash:
        return
    wait_for_sequencer_tx(seq_url, tx_hash, artifact, label=label)


def log_vault_liquidity(
    cfg: Path,
    manifest: dict,
    artifact: Path,
    *,
    phase: str,
) -> None:
    try:
        inner = vault_status_json(cfg, manifest)
        vault_cfg = inner.get("vault_config") if isinstance(inner.get("vault_config"), dict) else {}
        total_lo = int(vault_cfg.get("total_allocated_lo", 0) or 0)
        unalloc = vault_unallocated_lo(cfg, manifest)
        log_artifact(
            artifact,
            phase,
            True,
            total_allocated_lo=total_lo,
            unallocated_lo=unalloc,
            next_stream_id=vault_cfg.get("next_stream_id"),
            vault_holding_balance_hex=inner.get("vault_holding_balance_hex"),
        )
    except E2EError as exc:
        log_artifact(artifact, phase, False, error=str(exc))


def chain_timestamp_to_fold_seconds(ts: int) -> int:
    if ts >= 1_000_000_000_000:
        return ts // 1000
    return ts


def module_json_call(cfg: Path, method: str, *args: str) -> dict[str, Any]:
    r = logoscore_cmd(cfg, "call", "payment_streams_module", method, *args)
    parsed = call_result(r)
    inner_raw = parsed.get("result")
    if isinstance(inner_raw, str):
        try:
            return json.loads(inner_raw)
        except json.JSONDecodeError:
            return {"status": "error", "raw": inner_raw[:500]}
    return inner_raw if isinstance(inner_raw, dict) else {"status": "error"}


def log_chain_baseline_before_create(
    cfg: Path,
    manifest: dict,
    vault_id: int,
    planned_stream_id: int,
    artifact: Path,
) -> None:
    clock = module_json_call(cfg, "readClock10Decoded")
    clock_ts = int((clock.get("decoded") or {}).get("timestamp", 0) or 0)
    stream_probe: dict[str, Any] = {"planned_stream_id": planned_stream_id}
    pda = manifest.get("stream_config_account_id")
    if pda:
        stream_probe["readStreamConfigDecoded"] = module_json_call(cfg, "readStreamConfigDecoded", str(pda))
    log_artifact(
        artifact,
        "baseline_before_create",
        True,
        vault_id=vault_id,
        next_stream_id=planned_stream_id,
        clock10=clock,
        clock_fold_seconds=chain_timestamp_to_fold_seconds(clock_ts),
        stream_slot_probe=stream_probe,
    )


def log_chain_checkpoint_after_create(
    cfg: Path,
    manifest: dict,
    stream_id: int,
    artifact: Path,
) -> None:
    clock = module_json_call(cfg, "readClock10Decoded")
    clock_ts = int((clock.get("decoded") or {}).get("timestamp", 0) or 0)
    clock_s = chain_timestamp_to_fold_seconds(clock_ts)
    stream_json: dict[str, Any] = {}
    pda = manifest.get("stream_config_account_id")
    if pda:
        stream_json = module_json_call(cfg, "readStreamConfigDecoded", str(pda))
    dec = stream_json.get("decoded") if isinstance(stream_json.get("decoded"), dict) else {}
    accrued_as_of = int(dec.get("accrued_as_of", 0) or 0)
    checkpoint_s = chain_timestamp_to_fold_seconds(accrued_as_of)
    log_artifact(
        artifact,
        "checkpoint_after_create",
        stream_json.get("status") == "ok",
        stream_id=stream_id,
        clock_fold_seconds=clock_s,
        accrued_as_of_raw=accrued_as_of,
        accrued_as_of_fold_seconds=checkpoint_s,
        fold_gap_seconds=max(0, clock_s - checkpoint_s),
        stream_config=stream_json,
    )


def bump_allocation_on_chain(
    cfg: Path,
    manifest: dict,
    vault_id: int,
    stream_id: int,
    seq_url: str,
    increase_lo: int = 1,
    *,
    repo: Path | None = None,
) -> None:
    if continuation_e2e_run() and repo is not None:
        try:
            seed_top_up_stream_onchain(repo, manifest, vault_id, stream_id, increase_lo, cfg)
            sync_wallet(cfg, seq_url)
            logoscore_cmd(cfg, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))
            return
        except E2EError:
            sync_wallet(cfg, seq_url)
    topup = {
        "signer": manifest["owner_account_id"],
        "vault_id": vault_id,
        "stream_id": stream_id,
        "increase_lo": increase_lo,
        "increase_hi": 0,
    }
    logoscore_cmd(
        cfg,
        "call",
        "payment_streams_module",
        "chainAction",
        "topUpStream",
        json.dumps(topup),
        timeout=testnet_chain_action_timeout_s(),
    )
    sync_wallet(cfg, seq_url)
    logoscore_cmd(cfg, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))


def manifest_stream_id(manifest: dict) -> int:
    sid = manifest.get("stream_id")
    if sid is None:
        raise E2EError("manifest missing stream_id (run create_demo_stream first)")
    return int(sid)


def continuation_e2e_run() -> bool:
    return os.environ.get("SKIP_SEED", "").strip() == "1" or os.environ.get("RESTORE_LOCALNET", "1").strip() == "0"


def local_e2e_create_via() -> str:
    explicit = os.environ.get("E2E_CREATE_VIA", "").strip().lower()
    if explicit:
        return explicit
    # Continuation legs use seed + logoscore wallet home (nonces match module chainActions).
    # chainAction-only create often returns tx_hash while getTransaction stays null on leg 2.
    if continuation_e2e_run():
        return "seed"
    return "seed"


def sequencer_json_rpc(seq_url: str, method: str, params: list[Any]) -> Any:
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    import urllib.error
    import urllib.request

    req = urllib.request.Request(
        seq_url.rstrip("/"),
        data=body,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=15) as resp:
        payload = json.loads(resp.read().decode())
    if "error" in payload:
        raise E2EError(f"sequencer RPC {method}: {payload['error']}")
    return payload.get("result")


def wait_for_sequencer_tx(
    seq_url: str,
    tx_hash: str,
    artifact: Path,
    *,
    label: str = "create_stream",
) -> None:
    h = tx_hash.strip().lower().removeprefix("0x")
    if len(h) != 64:
        raise E2EError(f"invalid tx_hash for wait: {tx_hash!r}")
    wait_s = e2e_tx_onchain_wait_s()
    poll_s = float(os.environ.get("E2E_TX_ONCHAIN_POLL_S", "0.5"))
    t0 = time.monotonic()
    delay = max(0.25, poll_s)
    attempt = 0
    while time.monotonic() - t0 <= wait_s:
        attempt += 1
        try:
            found = sequencer_json_rpc(seq_url, "getTransaction", [h])
        except E2EError:
            found = None
        if found is not None:
            log_artifact(
                artifact,
                "wait_tx_on_chain",
                True,
                label=label,
                tx_hash=h,
                attempts=attempt,
                elapsed_s=round(time.monotonic() - t0, 2),
            )
            return
        time.sleep(delay)
        delay = min(delay * 1.5, 5.0)
    log_artifact(
        artifact,
        "wait_tx_on_chain",
        False,
        label=label,
        tx_hash=h,
        attempts=attempt,
        elapsed_s=round(time.monotonic() - t0, 2),
    )
    raise E2EError(f"transaction {h} never appeared on sequencer (waited {wait_s}s)")


def ensure_sequencer_advancing(repo: Path, seq_url: str, artifact: Path) -> None:
    window = float(os.environ.get("E2E_SEQ_ADVANCE_WINDOW_S", "12"))
    b0 = int(sequencer_json_rpc(seq_url, "getLastBlockId", []))
    time.sleep(window)
    b1 = int(sequencer_json_rpc(seq_url, "getLastBlockId", []))
    delta = b1 - b0
    if delta > 0:
        log_artifact(artifact, "sequencer_blocks", True, block_delta=delta, window_s=window)
        return
    log_artifact(
        artifact,
        "sequencer_blocks",
        False,
        block_delta=0,
        window_s=window,
        action="localnet_stop_start",
    )
    # Use unified lifecycle.sh for scaffold check
    lifecycle = repo / "scripts" / "lifecycle.sh"
    if lifecycle.is_file():
        run(["bash", str(lifecycle), "scaffold", "check"], cwd=repo, timeout=120)
    run(["lgs", "localnet", "stop"], cwd=repo, timeout=60)
    time.sleep(2)
    run(["lgs", "localnet", "start"], cwd=repo, timeout=120)
    time.sleep(3)
    b2 = int(sequencer_json_rpc(seq_url, "getLastBlockId", []))
    time.sleep(window)
    b3 = int(sequencer_json_rpc(seq_url, "getLastBlockId", []))
    if b3 <= b2:
        raise E2EError(
            f"sequencer not advancing blocks after localnet restart (delta={b3 - b2} over {window}s)"
        )
    log_artifact(
        artifact,
        "sequencer_blocks",
        True,
        block_delta=b3 - b2,
        window_s=window,
        after_restart=True,
    )


def stream_config_on_chain(cfg: Path, pda: str, expected_stream_id: int) -> dict[str, Any]:
    stream_json = module_json_call(cfg, "readStreamConfigDecoded", pda)
    if stream_json.get("status") != "ok":
        return stream_json
    dec = stream_json.get("decoded")
    if not isinstance(dec, dict):
        return {"status": "error", "message": "missing decoded stream config"}
    if int(dec.get("stream_id", -1)) != expected_stream_id:
        return {
            "status": "error",
            "message": f"stream_id mismatch (expected {expected_stream_id})",
            "decoded": dec,
        }
    return stream_json


def wait_for_stream_config_on_chain(
    cfg: Path,
    manifest: dict,
    stream_id: int,
    seq_url: str,
    artifact: Path,
) -> None:
    pda = manifest.get("stream_config_account_id")
    if not pda:
        raise E2EError("manifest missing stream_config_account_id (refresh_manifest_pdas?)")
    wait_s = int(os.environ.get("E2E_STREAM_ONCHAIN_WAIT_S", "90"))
    poll_s = float(os.environ.get("E2E_STREAM_ONCHAIN_POLL_S", "0.5"))
    t0 = time.monotonic()
    delay = max(0.25, poll_s)
    last_json: dict[str, Any] = {"status": "error", "message": "not polled"}
    attempt = 0
    while time.monotonic() - t0 <= wait_s:
        attempt += 1
        sync_wallet(cfg, seq_url)
        reload_payment_streams_wallet(cfg, seq_url)
        last_json = stream_config_on_chain(cfg, str(pda), stream_id)
        if last_json.get("status") == "ok":
            log_artifact(
                artifact,
                "wait_stream_on_chain",
                True,
                stream_id=stream_id,
                attempts=attempt,
                elapsed_s=round(time.monotonic() - t0, 2),
            )
            return
        time.sleep(delay)
        delay = min(delay * 1.5, 5.0)
    log_artifact(
        artifact,
        "wait_stream_on_chain",
        False,
        stream_id=stream_id,
        attempts=attempt,
        elapsed_s=round(time.monotonic() - t0, 2),
        last=last_json,
    )
    raise E2EError(
        f"stream {stream_id} PDA not initialized after create (waited {wait_s}s): {last_json!r}"
    )


def ensure_continuation_vault_funded(
    cfg: Path,
    manifest: dict,
    seq_url: str,
    artifact: Path,
    needed_unallocated_lo: int,
) -> None:
    unalloc = vault_unallocated_lo(cfg, manifest)
    if unalloc >= needed_unallocated_lo:
        log_artifact(
            artifact,
            "continuation_vault_funding",
            True,
            skipped=True,
            unallocated_lo=unalloc,
            needed_unallocated_lo=needed_unallocated_lo,
        )
        return
    repo = Path(os.environ.get("REPO", Path.cwd()))
    # Use unified fixture.sh for owner topup (continuation)
    fixture = repo / "scripts" / "fixture.sh"
    if fixture.is_file():
        # Note: fixture.sh topup not yet implemented; fallback to legacy if needed
        topup_script = repo / "scripts" / "e2e" / "continuation-owner-topup.sh"
        if topup_script.is_file():
            run(["bash", str(topup_script)], cwd=repo, timeout=600)
        sync_wallet(cfg, seq_url)
        logoscore_cmd(
            cfg,
            "call",
            "payment_streams_module",
            "rediscoverStreams",
            str(int(manifest.get("vault_id", 0))),
        )
        unalloc = vault_unallocated_lo(cfg, manifest)
        if unalloc >= needed_unallocated_lo:
            log_artifact(
                artifact,
                "continuation_vault_funding",
                True,
                skipped=True,
                after_pinata=True,
                unallocated_lo=unalloc,
                needed_unallocated_lo=needed_unallocated_lo,
            )
            return
    shortfall = needed_unallocated_lo - unalloc + int(os.environ.get("E2E_VAULT_UNALLOC_BUFFER_LO", "50"))
    deposit_via = os.environ.get("E2E_CONTINUATION_DEPOSIT_VIA", "seed").strip().lower()
    if deposit_via == "chainaction":
        deposit_body = {
            "signer": manifest["owner_account_id"],
            "vault_id": int(manifest.get("vault_id", 0)),
            "amount_lo": shortfall,
            "amount_hi": 0,
        }
        r = logoscore_cmd(
            cfg,
            "call",
            "payment_streams_module",
            "chainAction",
            "deposit",
            json.dumps(deposit_body),
            timeout=e2e_subprocess_timeout_s(),
        )
        parsed = call_result(r)
        ok = chain_action_success(parsed)
        tx_hash = chain_action_tx_hash(parsed)
        log_artifact(
            artifact,
            "continuation_vault_funding",
            ok,
            unallocated_lo=unalloc,
            deposit_lo=shortfall,
            tx_hash=tx_hash,
            via="chainAction_deposit",
        )
        if not ok:
            raise E2EError(f"continuation vault deposit failed: {parsed}")
        if tx_hash:
            await_chain_action_inclusion(seq_url, parsed, artifact, label="continuation_deposit")
    else:
        seed_vault_deposit_onchain(repo, manifest, shortfall, cfg)
        log_artifact(
            artifact,
            "continuation_vault_funding",
            True,
            unallocated_lo=unalloc,
            deposit_lo=shortfall,
            via="seed_deposit_onchain",
        )
    sync_wallet(cfg, seq_url)
    logoscore_cmd(
        cfg,
        "call",
        "payment_streams_module",
        "rediscoverStreams",
        str(int(manifest.get("vault_id", 0))),
    )


def continuation_allocation_lo(manifest: dict, cfg: Path | None = None) -> int:
    default = manifest_allocation_lo(manifest)
    if not continuation_e2e_run():
        return default
    cap = int(os.environ.get("E2E_CONTINUATION_ALLOCATION_LO", str(default)))
    target = min(default, cap)
    if cfg is None:
        return target
    vault_id = int(manifest.get("vault_id", 0))
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    sync_wallet(cfg, seq_url)
    logoscore_cmd(cfg, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))
    unalloc = vault_unallocated_lo(cfg, manifest)
    buffer_lo = int(os.environ.get("E2E_VAULT_UNALLOC_BUFFER_LO", "50"))
    afford = max(0, unalloc - buffer_lo)
    chosen = min(target, afford)
    min_create = int(
        os.environ.get(
            "E2E_MIN_ALLOCATION_LO",
            os.environ.get("E2E_MIN_STREAM_ALLOCATION_LO", "450"),
        )
    )
    if chosen >= min_create:
        return chosen
    if afford >= min_create:
        return afford
    raise E2EError(
        f"continuation run vault unallocated {unalloc} lo (afford {afford}) "
        f"below minimum allocation {min_create}; "
        f"run make full-reset-localnet or deposit to vault before back-to-back leg 2"
    )


def seed_top_up_stream_onchain(
    repo: Path,
    manifest: dict,
    vault_id: int,
    stream_id: int,
    increase_lo: int,
    cfg: Path | None = None,
) -> None:
    guest = Path(os.environ["PAYMENT_STREAMS_GUEST_BIN"])
    owner = manifest.get("owner_account_id", "")
    state_file = repo / ".lez_payment_streams-state"
    if state_file.is_file():
        for line in state_file.read_text().splitlines():
            if line.startswith("SIGNER_ID="):
                owner = line.split("=", 1)[1].strip().strip("'\"")
                break
    wallet_home = Path(os.environ.get("LEE_WALLET_HOME_DIR", repo / ".scaffold" / "wallet"))
    apply_e2e_wallet_poll_overrides(wallet_home)
    env = os.environ.copy()
    env["LEE_WALLET_HOME_DIR"] = str(wallet_home)
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    if cfg is not None:
        release_logoscore_wallet(cfg)
    try:
        proc = run(
            [
                "cargo",
                "run",
                "-q",
                "--manifest-path",
                "examples/Cargo.toml",
                "--bin",
                "seed_localnet_fixture",
                "--",
                "top-up-stream-onchain",
                "--program-bin",
                str(guest),
                "--owner",
                owner,
                "--vault-id",
                str(vault_id),
                "--stream-id",
                str(stream_id),
                "--increase-lo",
                str(increase_lo),
                "--sequencer-url",
                seq_url,
            ],
            cwd=repo,
            env=env,
            timeout=e2e_subprocess_timeout_s(),
        )
        if proc.returncode != 0:
            raise E2EError(f"seed top-up-stream-onchain failed: {proc.stderr or proc.stdout}")
    finally:
        if cfg is not None:
            reopen_logoscore_wallet(cfg, seq_url)
            reload_payment_streams_wallet(cfg, seq_url)


def read_vault_next_stream_id_cli(repo: Path, manifest: dict, wallet_home: Path) -> int:
    guest = Path(os.environ["PAYMENT_STREAMS_GUEST_BIN"])
    owner = manifest.get("owner_account_id", "")
    state_file = repo / ".lez_payment_streams-state"
    if state_file.is_file():
        for line in state_file.read_text().splitlines():
            if line.startswith("SIGNER_ID="):
                owner = line.split("=", 1)[1].strip().strip("'\"")
                break
    env = os.environ.copy()
    env["LEE_WALLET_HOME_DIR"] = str(wallet_home)
    proc = run(
        [
            "cargo",
            "run",
            "-q",
            "--manifest-path",
            "examples/Cargo.toml",
            "--bin",
            "seed_localnet_fixture",
            "--",
            "read-vault-next-stream-id",
            "--program-bin",
            str(guest),
            "--owner",
            owner,
        ],
        cwd=repo,
        env=env,
        timeout=120,
    )
    if proc.returncode != 0:
        raise E2EError(f"read-vault-next-stream-id failed: {proc.stderr or proc.stdout}")
    return int(proc.stdout.strip())


def precreate_stream_before_daemons(
    repo: Path,
    manifest_path: Path,
    manifest: dict,
    artifact: Path,
    wallet_config: Path,
    cfg_user: Path,
) -> int:
    """Continuation runs: create on-chain after user wallet sync (same wallet as close/claim)."""
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    vault_id = int(manifest.get("vault_id", 0))
    strip_snapshot_stream_fields(manifest, manifest_path)
    reload_payment_streams_wallet(cfg_user, seq_url)
    create_id = vault_next_stream_id(cfg_user, manifest)
    target_alloc = manifest_allocation_lo(manifest)
    ensure_continuation_vault_funded(cfg_user, manifest, seq_url, artifact, target_alloc + 50)
    log_artifact(
        artifact,
        "plan_demo_stream",
        True,
        vault_id=vault_id,
        next_stream_id=create_id,
        source="chain_getVaultStatus",
        precreate_after_wallet_sync=True,
    )
    alloc = continuation_allocation_lo(manifest, cfg_user)
    log_artifact(
        artifact,
        "continuation_vault_unallocated",
        True,
        unallocated_lo=vault_unallocated_lo(cfg_user, manifest),
        chosen_allocation_lo=alloc,
    )
    rate = int(manifest.get("stream_rate", 1))
    t0_total = time.monotonic()
    wallet_home = Path(os.environ.get("LEE_WALLET_HOME_DIR", repo / ".scaffold" / "wallet"))
    apply_e2e_wallet_poll_overrides(wallet_home)
    # Use unified fixture.sh for stream creation
    fixture = repo / "scripts" / "fixture.sh"
    script = repo / "scripts" / "create-localnet-stream-fixture.sh"
    env = os.environ.copy()
    env["FIXTURE_MANIFEST"] = str(manifest_path)
    env["REPO"] = str(repo)
    env["STREAM_ID"] = str(create_id)
    env["SEQUENCER_URL"] = seq_url
    env["SEED_ALLOCATION"] = str(alloc)
    env["CREATE_FORCE"] = "1"
    env["E2E_PER_RUN_STREAM"] = "1"
    env["LEE_WALLET_HOME_DIR"] = str(wallet_home)
    release_logoscore_wallet(cfg_user)
    try:
        if fixture.is_file():
            proc = run(["bash", str(fixture), "stream", "create", "0"], cwd=repo, env=env, timeout=e2e_subprocess_timeout_s())
        else:
            # Fallback to legacy script during transition
            proc = run(["bash", str(script)], cwd=repo, env=env, timeout=e2e_subprocess_timeout_s())
    finally:
        reopen_logoscore_wallet(cfg_user, seq_url)
        reload_payment_streams_wallet(cfg_user, seq_url)
    ok = proc.returncode == 0
    log_artifact(
        artifact,
        "create_demo_stream",
        ok,
        stream_id=create_id,
        via="seed_create_stream_onchain",
        precreate_after_wallet_sync=True,
        allocation_lo=alloc,
        elapsed_s=round(time.monotonic() - t0_total, 2),
        stderr=(proc.stderr or "")[-600:],
    )
    if not ok:
        raise E2EError(f"precreate stream failed: {proc.stderr or proc.stdout}")
    set_manifest_allocation(manifest, alloc)
    refresh_manifest_pdas(repo, manifest_path, create_id, manifest)
    wait_for_stream_config_on_chain(cfg_user, manifest, create_id, seq_url, artifact)
    manifest.clear()
    manifest.update(json.loads(manifest_path.read_text()))
    os.environ["E2E_PRECREATED_STREAM_ID"] = str(create_id)
    return create_id


def refresh_manifest_pdas(repo: Path, manifest_path: Path, stream_id: int, manifest: dict) -> None:
    guest = Path(os.environ["PAYMENT_STREAMS_GUEST_BIN"])
    proc = run(
        [
            "cargo",
            "run",
            "-q",
            "--manifest-path",
            "examples/Cargo.toml",
            "--bin",
            "seed_localnet_fixture",
            "--",
            "write-manifest",
            "--program-bin",
            str(guest),
            "--owner",
            manifest["owner_account_id"],
            "--provider",
            manifest["provider_account_id"],
            "--stream-id",
            str(stream_id),
            "--vault-id",
            str(int(manifest.get("vault_id", 0))),
            "--sequencer-url",
            manifest.get("sequencer_url", "http://127.0.0.1:3040"),
            "--output",
            str(manifest_path),
        ],
        cwd=repo,
        timeout=300,
    )
    if proc.returncode != 0:
        raise E2EError(f"write-manifest failed: {proc.stderr or proc.stdout}")
    manifest.clear()
    manifest.update(json.loads(manifest_path.read_text()))


def seed_deposit_amount_lo(manifest: dict) -> int:
    raw = os.environ.get("SEED_DEPOSIT_AMOUNT", "").strip()
    if raw:
        return int(raw)
    demo = manifest.get("demo_deposit_amount")
    if demo is not None:
        return int(demo)
    return manifest_allocation_lo(manifest) + 100


def refresh_manifest_vault_baseline(
    repo: Path, manifest_path: Path, manifest: dict, vault_id: int
) -> None:
    guest = Path(os.environ["PAYMENT_STREAMS_GUEST_BIN"])
    deposit = seed_deposit_amount_lo(manifest)
    proc = run(
        [
            "cargo",
            "run",
            "-q",
            "--manifest-path",
            "examples/Cargo.toml",
            "--bin",
            "seed_localnet_fixture",
            "--",
            "write-vault-manifest",
            "--program-bin",
            str(guest),
            "--owner",
            manifest["owner_account_id"],
            "--provider",
            manifest["provider_account_id"],
            "--vault-id",
            str(vault_id),
            "--deposit-amount",
            str(deposit),
            "--stream-rate",
            str(int(manifest.get("stream_rate", 1))),
            "--allocation",
            str(manifest_allocation_lo(manifest)),
            "--sequencer-url",
            manifest.get("sequencer_url", "http://127.0.0.1:3040"),
            "--output",
            str(manifest_path),
        ],
        cwd=repo,
        timeout=300,
    )
    if proc.returncode != 0:
        raise E2EError(f"write-vault-manifest failed: {proc.stderr or proc.stdout}")
    manifest.clear()
    manifest.update(json.loads(manifest_path.read_text()))


def resolve_store_vault_id_subprocess(repo: Path) -> int:
    env = os.environ.copy()
    proc = run(
        ["bash", str(repo / "scripts" / "fixture.sh"), "vault", "resolve-id"],
        cwd=repo,
        env=env,
        timeout=120,
    )
    if proc.returncode != 0:
        raise E2EError(f"vault resolve-id failed: {proc.stderr or proc.stdout}")
    raw = (proc.stdout or "").strip().splitlines()[-1].strip()
    return int(raw)


def ensure_fresh_vault_for_store_run(
    repo: Path,
    manifest_path: Path,
    manifest: dict,
    artifact: Path,
) -> int:
    chain = os.environ.get("CHAIN", "local").strip().lower()
    strip_snapshot_stream_fields(manifest, manifest_path)
    vault_id = resolve_store_vault_id_subprocess(repo)
    deposit_lo = seed_deposit_amount_lo(manifest)
    log_artifact(
        artifact,
        "plan_demo_vault",
        True,
        vault_id=vault_id,
        deposit_lo=deposit_lo,
        chain=chain,
        source="resolve_store_vault_id",
    )
    if chain == "testnet":
        ensure_script = repo / "scripts" / "e2e" / "ensure-testnet-vault.sh"
        wc = os.environ.get("WALLET_CONFIG", "")
        ws = os.environ.get("WALLET_STORAGE", "")
        seq = manifest.get("sequencer_url", "https://testnet.lez.logos.co/")
        prog_hex = manifest.get("program_id_hex", "")
        guest = os.environ["PAYMENT_STREAMS_GUEST_BIN"]
        submit = os.environ.get("LEZ_TESTNET_SUBMIT", "")
        cmd = [
            "bash",
            str(ensure_script),
            "--manifest",
            str(manifest_path),
            "--vault-id",
            str(vault_id),
            "--deposit-amount",
            str(deposit_lo),
            "--wallet-config",
            wc,
            "--wallet-storage",
            ws,
            "--sequencer-url",
            seq,
            "--program-id-hex",
            prog_hex,
            "--program-bin",
            guest,
        ]
        if submit:
            cmd.extend(["--submit-helper", submit])
        proc = run(cmd, cwd=repo, timeout=e2e_subprocess_timeout_s())
        ok = proc.returncode == 0
        log_artifact(
            artifact,
            "vault_ensure",
            ok,
            vault_id=vault_id,
            via="ensure-testnet-vault.sh",
            deposit_lo=deposit_lo,
            stderr=(proc.stderr or "")[-800:],
        )
        if not ok:
            raise E2EError(f"ensure-testnet-vault failed: {proc.stderr or proc.stdout}")
        manifest.clear()
        manifest.update(json.loads(manifest_path.read_text()))
    else:
        env = os.environ.copy()
        env["FIXTURE_MANIFEST"] = str(manifest_path)
        env["VAULT_ID"] = str(vault_id)
        env["SEED_DEPOSIT_AMOUNT"] = str(deposit_lo)
        env["SEED_ALLOCATION"] = str(manifest_allocation_lo(manifest))
        proc = run(
            ["bash", str(repo / "scripts" / "fixture.sh"), "vault", "ensure", str(vault_id)],
            cwd=repo,
            env=env,
            timeout=e2e_subprocess_timeout_s(),
        )
        ok = proc.returncode == 0
        log_artifact(
            artifact,
            "vault_ensure",
            ok,
            vault_id=vault_id,
            via="fixture.sh_vault_ensure",
            deposit_lo=deposit_lo,
            stderr=(proc.stderr or "")[-800:],
        )
        if not ok:
            raise E2EError(f"fixture vault ensure failed: {proc.stderr or proc.stdout}")
        refresh_manifest_vault_baseline(repo, manifest_path, manifest, vault_id)
    manifest["vault_id"] = vault_id
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    return vault_id


def create_demo_stream_for_run(
    cfg_user: Path,
    cfg_provider: Path,
    repo: Path,
    manifest_path: Path,
    manifest: dict,
    persist_user: Path,
    artifact: Path,
    wallet_config: Path | None = None,
) -> None:
    create_t0 = time.monotonic()
    subproc_timeout = e2e_subprocess_timeout_s()
    chain = os.environ.get("CHAIN", "local").strip().lower()
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    vault_id = int(manifest.get("vault_id", 0))
    sync_wallet(cfg_user, seq_url)
    sync_wallet(cfg_provider, seq_url)
    precreated_raw = os.environ.get("E2E_PRECREATED_STREAM_ID", "").strip()
    if precreated_raw:
        manifest.clear()
        manifest.update(json.loads(manifest_path.read_text()))
        create_id = int(precreated_raw)
    else:
        strip_snapshot_stream_fields(manifest, manifest_path)
        if store_reuse_baseline_vault() or continuation_e2e_run():
            create_id = vault_next_stream_id(cfg_user, manifest)
        else:
            create_id = 0
    precreated = precreated_raw
    if precreated and int(precreated) == create_id and manifest.get("stream_config_account_id"):
        log_artifact(
            artifact,
            "create_demo_stream",
            True,
            stream_id=create_id,
            via="precreate_before_daemons",
            skipped_onchain=True,
        )
        for _ in range(3):
            sync_wallet(cfg_user, seq_url)
            sync_wallet(cfg_provider, seq_url)
            time.sleep(1)
        reload_payment_streams_wallet(cfg_user, seq_url)
        reload_payment_streams_wallet(cfg_provider, seq_url)
        logoscore_cmd(cfg_user, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))
        log_chain_baseline_before_create(cfg_user, manifest, vault_id, create_id, artifact)
        log_chain_checkpoint_after_create(cfg_user, manifest, create_id, artifact)
        wait_for_stream_fundable(cfg_user, vault_id, create_id, manifest, seq_url, artifact)
        return
    log_artifact(
        artifact,
        "plan_demo_stream",
        True,
        vault_id=vault_id,
        next_stream_id=create_id,
        source="chain_getVaultStatus",
    )
    log_chain_baseline_before_create(cfg_user, manifest, vault_id, create_id, artifact)

    if chain == "local" and continuation_e2e_run():
        reload_payment_streams_wallet(cfg_user, seq_url)

    rate = int(manifest.get("stream_rate", 1))
    alloc = continuation_allocation_lo(manifest, cfg_user)

    if chain == "local":
        create_via = local_e2e_create_via()
        if create_via == "chainaction":
            create_body = {
                "signer": manifest["owner_account_id"],
                "vault_id": vault_id,
                "stream_id": create_id,
                "provider": manifest["provider_account_id"],
                "rate": rate,
                "allocation_lo": alloc,
                "allocation_hi": 0,
            }
            r = logoscore_cmd(
                cfg_user,
                "call",
                "payment_streams_module",
                "chainAction",
                "createStream",
                json.dumps(create_body),
                timeout=subproc_timeout,
            )
            parsed = call_result(r)
            ok_create = parsed.get("status") == "ok"
            tx_hash = None
            if ok_create and isinstance(parsed.get("result"), str):
                inner = json.loads(parsed["result"])
                ok_create = inner.get("success", False)
                tx_hash = inner.get("tx_hash")
            log_artifact(
                artifact,
                "create_demo_stream",
                ok_create,
                stream_id=create_id,
                via="chainAction_createStream",
                tx_hash=tx_hash,
                elapsed_s=round(time.monotonic() - create_t0, 2),
            )
            if not ok_create:
                raise E2EError(f"create_demo_stream failed: {parsed}")
            if tx_hash:
                wait_for_sequencer_tx(seq_url, tx_hash, artifact, label="chainAction_createStream")
            refresh_manifest_pdas(repo, manifest_path, create_id, manifest)
            wait_for_stream_config_on_chain(cfg_user, manifest, create_id, seq_url, artifact)
        else:
            wallet_home = Path(os.environ.get("LEE_WALLET_HOME_DIR", repo / ".scaffold" / "wallet"))
            tmp_wallet = persist_user / "demo-create-wallet"
            shared_wallet = continuation_e2e_run() or os.environ.get("E2E_SEED_SHARED_WALLET", "").strip() == "1"
            env = os.environ.copy()
            env["FIXTURE_MANIFEST"] = str(manifest_path)
            env["REPO"] = str(repo)
            env["STREAM_ID"] = str(create_id)
            env["SEQUENCER_URL"] = seq_url
            env["SEED_ALLOCATION"] = str(alloc)
            env["CREATE_FORCE"] = "1"
            env["E2E_PER_RUN_STREAM"] = "1"
            fixture = repo / "scripts" / "fixture.sh"
            script = repo / "scripts" / "create-localnet-stream-fixture.sh"
            proc = None
            for attempt in range(3):
                attempt_t0 = time.monotonic()
                sync_wallet(cfg_user, seq_url)
                if shared_wallet:
                    apply_e2e_wallet_poll_overrides(wallet_home)
                    env["LEE_WALLET_HOME_DIR"] = str(wallet_home)
                    if attempt == 0:
                        log_artifact(
                            artifact,
                            "create_stream_poll_budget",
                            True,
                            shared_wallet=True,
                            **wallet_tx_poll_budget_s(wallet_home / "wallet_config.json"),
                        )
                else:
                    if tmp_wallet.exists():
                        shutil.rmtree(tmp_wallet)
                    shutil.copytree(wallet_home, tmp_wallet)
                    apply_e2e_wallet_poll_overrides(tmp_wallet)
                    if attempt == 0:
                        log_artifact(
                            artifact,
                            "create_stream_poll_budget",
                            True,
                            **wallet_tx_poll_budget_s(tmp_wallet / "wallet_config.json"),
                        )
                    env["LEE_WALLET_HOME_DIR"] = str(tmp_wallet)
                if attempt > 0:
                    time.sleep(5 * attempt)
                if fixture.is_file():
                    proc = run(["bash", str(fixture), "stream", "create", str(vault_id)], cwd=repo, env=env, timeout=subproc_timeout)
                else:
                    proc = run(["bash", str(script)], cwd=repo, env=env, timeout=subproc_timeout)
                attempt_elapsed = round(time.monotonic() - attempt_t0, 2)
                if proc.returncode == 0:
                    log_artifact(
                        artifact,
                        "create_stream_seed_attempt",
                        True,
                        attempt=attempt + 1,
                        elapsed_s=attempt_elapsed,
                    )
                    break
                err = (proc.stderr or "") + (proc.stdout or "")
                log_artifact(
                    artifact,
                    "create_stream_seed_attempt",
                    False,
                    attempt=attempt + 1,
                    elapsed_s=attempt_elapsed,
                    stderr_tail=err[-400:],
                )
                if "confirm transaction" not in err and "Transaction not found" not in err:
                    break
            assert proc is not None
            ok_create = proc.returncode == 0
            log_artifact(
                artifact,
                "create_demo_stream",
                ok_create,
                stream_id=create_id,
                via="seed_create_stream_onchain",
                stderr=(proc.stderr or "")[-800:],
                stdout=(proc.stdout or "")[-400:],
                elapsed_s=round(time.monotonic() - create_t0, 2),
                subproc_timeout_s=subproc_timeout,
            )
            if not ok_create:
                create_body = {
                    "signer": manifest["owner_account_id"],
                    "vault_id": vault_id,
                    "stream_id": create_id,
                    "provider": manifest["provider_account_id"],
                    "rate": rate,
                    "allocation_lo": alloc,
                    "allocation_hi": 0,
                }
                r = logoscore_cmd(
                    cfg_user,
                    "call",
                    "payment_streams_module",
                    "chainAction",
                    "createStream",
                    json.dumps(create_body),
                    timeout=subproc_timeout,
                )
                parsed = call_result(r)
                ok_create = parsed.get("status") == "ok"
                tx_hash = None
                if ok_create and isinstance(parsed.get("result"), str):
                    inner = json.loads(parsed["result"])
                    ok_create = inner.get("success", False)
                    tx_hash = inner.get("tx_hash")
                log_artifact(
                    artifact,
                    "create_demo_stream_fallback",
                    ok_create,
                    stream_id=create_id,
                    via="chainAction_createStream",
                    tx_hash=tx_hash,
                    elapsed_s=round(time.monotonic() - create_t0, 2),
                )
                if ok_create:
                    refresh_manifest_pdas(repo, manifest_path, create_id, manifest)
                    wait_for_stream_config_on_chain(cfg_user, manifest, create_id, seq_url, artifact)
                else:
                    raise E2EError(
                        f"create_demo_stream failed: {proc.stderr or proc.stdout}; fallback: {parsed}"
                    )
            else:
                manifest.clear()
                manifest.update(json.loads(manifest_path.read_text()))
                wait_for_stream_config_on_chain(cfg_user, manifest, create_id, seq_url, artifact)
    elif chain == "testnet":
        create_via = testnet_e2e_create_via()
        if create_via == "chainaction":
            create_body = {
                "signer": manifest["owner_account_id"],
                "vault_id": vault_id,
                "stream_id": create_id,
                "provider": manifest["provider_account_id"],
                "rate": rate,
                "allocation_lo": alloc,
                "allocation_hi": 0,
            }
            r = logoscore_cmd(
                cfg_user,
                "call",
                "payment_streams_module",
                "chainAction",
                "createStream",
                json.dumps(create_body),
                timeout=testnet_chain_action_timeout_s(),
            )
            parsed = call_result(r)
            ok_create = parsed.get("status") == "ok"
            tx_hash = None
            if ok_create and isinstance(parsed.get("result"), str):
                inner = json.loads(parsed["result"])
                ok_create = inner.get("success", False)
                tx_hash = inner.get("tx_hash")
            log_artifact(
                artifact,
                "create_demo_stream",
                ok_create,
                stream_id=create_id,
                tx_hash=tx_hash,
                via="chainAction_createStream",
            )
            if not ok_create:
                raise E2EError(f"create_demo_stream failed: {parsed}")
            if tx_hash:
                wait_for_sequencer_tx(seq_url, tx_hash, artifact, label="chainAction_createStream")
            refresh_manifest_pdas(repo, manifest_path, create_id, manifest)
            wait_for_stream_config_on_chain(cfg_user, manifest, create_id, seq_url, artifact)
        else:
            env = os.environ.copy()
            env["FIXTURE_MANIFEST"] = str(manifest_path)
            env["REPO"] = str(repo)
            env["STREAM_ID"] = str(create_id)
            env["SEQUENCER_URL"] = seq_url
            env["SEED_ALLOCATION"] = str(alloc)
            env["SEED_STREAM_RATE"] = str(rate)
            env["CREATE_FORCE"] = "1"
            env["E2E_PER_RUN_STREAM"] = "1"
            fixture = repo / "scripts" / "fixture.sh"
            script = repo / "scripts" / "create-testnet-stream-fixture.sh"
            if fixture.is_file():
                proc = run(["bash", str(fixture), "stream", "create", str(vault_id)], cwd=repo, env=env, timeout=subproc_timeout)
            else:
                proc = run(["bash", str(script)], cwd=repo, env=env, timeout=subproc_timeout)
            ok_create = proc.returncode == 0
            log_artifact(
                artifact,
                "create_demo_stream",
                ok_create,
                stream_id=create_id,
                via="bootstrap_create_stream_only",
                stderr=(proc.stderr or "")[-800:],
                stdout=(proc.stdout or "")[-400:],
                elapsed_s=round(time.monotonic() - create_t0, 2),
                subproc_timeout_s=subproc_timeout,
            )
            if not ok_create:
                raise E2EError(f"create_demo_stream failed: {proc.stderr or proc.stdout}")
            manifest.clear()
            manifest.update(json.loads(manifest_path.read_text()))
            wait_for_stream_config_on_chain(cfg_user, manifest, create_id, seq_url, artifact)
    else:
        create_body = {
            "signer": manifest["owner_account_id"],
            "vault_id": vault_id,
            "stream_id": create_id,
            "provider": manifest["provider_account_id"],
            "rate": rate,
            "allocation_lo": alloc,
            "allocation_hi": 0,
        }
        r = logoscore_cmd(
            cfg_user,
            "call",
            "payment_streams_module",
            "chainAction",
            "createStream",
            json.dumps(create_body),
            timeout=testnet_chain_action_timeout_s(),
        )
        parsed = call_result(r)
        ok_create = parsed.get("status") == "ok"
        tx_hash = None
        if ok_create and isinstance(parsed.get("result"), str):
            inner = json.loads(parsed["result"])
            ok_create = inner.get("success", False)
            tx_hash = inner.get("tx_hash")
        log_artifact(
            artifact,
            "create_demo_stream",
            ok_create,
            stream_id=create_id,
            tx_hash=tx_hash,
            via="chainAction_createStream",
        )
        if not ok_create:
            raise E2EError(f"create_demo_stream failed: {parsed}")
        refresh_manifest_pdas(repo, manifest_path, create_id, manifest)
        wait_for_stream_config_on_chain(cfg_user, manifest, create_id, seq_url, artifact)

    for _ in range(3):
        sync_wallet(cfg_user, seq_url)
        sync_wallet(cfg_provider, seq_url)
        time.sleep(1)
    if chain == "local":
        reload_payment_streams_wallet(cfg_user, seq_url)
        reload_payment_streams_wallet(cfg_provider, seq_url)
    logoscore_cmd(cfg_user, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))

    stream_id = manifest_stream_id(manifest)
    if stream_id != create_id:
        raise E2EError(
            f"manifest stream_id {stream_id} != planned create id {create_id} (per-run create mismatch)"
        )
    log_chain_checkpoint_after_create(cfg_user, manifest, stream_id, artifact)
    wait_for_stream_fundable(cfg_user, vault_id, stream_id, manifest, seq_url, artifact)


def stream_accrued_lo(cfg: Path, vault_id: int, stream_id: int) -> int:
    r = logoscore_cmd(cfg, "call", "payment_streams_module", "listMyStreams", str(vault_id))
    parsed = call_result(r)
    inner_raw = parsed.get("result")
    inner = json.loads(inner_raw) if isinstance(inner_raw, str) else inner_raw
    if not isinstance(inner, dict):
        return 0
    for row in inner.get("streams") or []:
        if int(row.get("stream_id", -1)) == stream_id:
            return int(row.get("accrued_lo") or 0)
    return 0


def stream_closed_on_chain(cfg: Path, manifest: dict) -> bool:
    pda = manifest.get("stream_config_account_id")
    if not pda:
        return False
    stream_json = module_json_call(cfg, "readStreamConfigDecoded", str(pda))
    dec = stream_json.get("decoded") if isinstance(stream_json.get("decoded"), dict) else {}
    return int(dec.get("stream_state", -1)) == 2


def account_balance_seq(seq_url: str, account_id: str) -> int:
    try:
        acc = sequencer_json_rpc(seq_url, "getAccount", [account_id])
    except E2EError:
        return 0
    if not isinstance(acc, dict):
        return 0
    return int(acc.get("balance", 0) or 0)


def vault_status_balances(cfg: Path, manifest: dict) -> tuple[int, int]:
    inner = vault_status_json(cfg, manifest)
    h = str(inner.get("vault_holding_balance_hex") or "")
    if h[:2].lower() == "0x":
        h = h[2:]
    bal = int.from_bytes(bytes.fromhex(h), "little") if h else 0
    vc = inner.get("vault_config") if isinstance(inner.get("vault_config"), dict) else {}
    total = int(vc.get("total_allocated_lo", 0) or 0)
    return bal, total


def stream_status_fields(cfg: Path, manifest: dict, stream_id: int) -> tuple[int, int, int]:
    body = json.dumps(
        {
            "owner": manifest["owner_account_id"],
            "vault_id": int(manifest.get("vault_id", 0)),
            "stream_id": stream_id,
        }
    )
    r = logoscore_cmd(cfg, "call", "payment_streams_module", "chainAction", "getStreamStatus", body)
    parsed = call_result(r)
    inner_raw = parsed.get("result")
    inner = json.loads(inner_raw) if isinstance(inner_raw, str) else inner_raw
    if not isinstance(inner, dict) or inner.get("status") != "ok":
        return 0, 0, -1
    return (
        int(inner.get("accrued_lo") or 0),
        int(inner.get("unaccrued_lo") or 0),
        int(inner.get("stream_state", -1) if inner.get("stream_state") is not None else -1),
    )


def run_auth_transfer_ensure(
    repo: Path,
    cfg_user: Path,
    cfg_provider: Path,
    manifest: dict,
    artifact: Path,
    wallet_home: Path,
) -> None:
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    ensure_script = repo / "scripts" / "auth-transfer-ensure.sh"
    if not ensure_script.is_file():
        raise E2EError(f"missing auth-transfer ensure script: {ensure_script}")
    release_logoscore_wallet(cfg_user)
    release_logoscore_wallet(cfg_provider)
    try:
        env = os.environ.copy()
        proc = run(
            [
                str(ensure_script),
                "--owner",
                str(manifest["owner_account_id"]),
                "--provider",
                str(manifest["provider_account_id"]),
                "--artifact",
                str(artifact),
                "--wallet-home",
                str(wallet_home),
            ],
            cwd=repo,
            env=env,
            timeout=600,
        )
        if proc.returncode != 0:
            raise E2EError(f"auth-transfer-ensure failed: {proc.stderr or proc.stdout}")
    finally:
        reopen_logoscore_wallet(cfg_user, seq_url)
        reopen_logoscore_wallet(cfg_provider, seq_url)
        reload_payment_streams_wallet(cfg_user, seq_url)
        reload_payment_streams_wallet(cfg_provider, seq_url)


def seed_close_stream_onchain(
    repo: Path,
    manifest: dict,
    vault_id: int,
    stream_id: int,
) -> None:
    guest = Path(os.environ["PAYMENT_STREAMS_GUEST_BIN"])
    owner = manifest.get("owner_account_id", "")
    is_testnet = os.environ.get("CHAIN", "local").strip().lower() == "testnet"
    # The localnet SIGNER_ID state file does not apply to testnet, whose close authority is
    # the manifest vault owner; using a stale signer makes the close revert and hang.
    if not is_testnet:
        state_file = repo / ".lez_payment_streams-state"
        if state_file.is_file():
            for line in state_file.read_text().splitlines():
                if line.startswith("SIGNER_ID="):
                    owner = line.split("=", 1)[1].strip().strip("'\"")
                    break
    wallet_home = Path(os.environ.get("LEE_WALLET_HOME_DIR", repo / ".scaffold" / "wallet"))
    # Testnet confirmation is slow (~90-120s/block); keep the wallet's own long poll
    # config instead of the fast localnet overrides, and give the subprocess more time.
    if not is_testnet:
        apply_e2e_wallet_poll_overrides(wallet_home)
    env = os.environ.copy()
    env["LEE_WALLET_HOME_DIR"] = str(wallet_home)
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    close_timeout = 900 if is_testnet else 300
    proc = run(
        [
            "cargo",
            "run",
            "-q",
            "--manifest-path",
            "examples/Cargo.toml",
            "--bin",
            "seed_localnet_fixture",
            "--",
            "close-stream-onchain",
            "--program-bin",
            str(guest),
            "--owner",
            owner,
            "--provider",
            manifest["provider_account_id"],
            "--vault-id",
            str(vault_id),
            "--stream-id",
            str(stream_id),
            "--sequencer-url",
            seq_url,
        ],
        cwd=repo,
        env=env,
        timeout=close_timeout,
    )
    if proc.returncode != 0:
        raise E2EError(f"seed close-stream-onchain failed: {proc.stderr or proc.stdout}")


def seed_claim_onchain(
    repo: Path,
    manifest: dict,
    vault_id: int,
    stream_id: int,
) -> None:
    guest = Path(os.environ["PAYMENT_STREAMS_GUEST_BIN"])
    owner = manifest.get("owner_account_id", "")
    is_testnet = os.environ.get("CHAIN", "local").strip().lower() == "testnet"
    wallet_home = Path(os.environ.get("LEE_WALLET_HOME_DIR", repo / ".scaffold" / "wallet"))
    if not is_testnet:
        apply_e2e_wallet_poll_overrides(wallet_home)
    env = os.environ.copy()
    env["LEE_WALLET_HOME_DIR"] = str(wallet_home)
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    claim_timeout = 900 if is_testnet else 300
    proc = run(
        [
            "cargo",
            "run",
            "-q",
            "--manifest-path",
            "examples/Cargo.toml",
            "--bin",
            "seed_localnet_fixture",
            "--",
            "claim-onchain",
            "--program-bin",
            str(guest),
            "--owner",
            owner,
            "--provider",
            manifest["provider_account_id"],
            "--vault-id",
            str(vault_id),
            "--stream-id",
            str(stream_id),
            "--sequencer-url",
            seq_url,
        ],
        cwd=repo,
        env=env,
        timeout=claim_timeout,
    )
    if proc.returncode != 0:
        raise E2EError(f"seed claim-onchain failed: {proc.stderr or proc.stdout}")


def demo_teardown(
    cfg_user: Path,
    cfg_provider: Path,
    manifest: dict,
    artifact: Path,
    repo: Path,
    narrator: "Narrator",
) -> None:
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    vault_id = int(manifest["vault_id"])
    stream_id = manifest_stream_id(manifest)
    stream_alloc = manifest_allocation_lo(manifest)
    close_applied = False
    narrator.phase("Close")
    narrator.step(f"Closing stream {stream_id} on chain")
    if os.environ.get("E2E_CLOSE_VIA", "seed").strip().lower() != "chainaction":
        try:
            release_logoscore_wallet(cfg_user)
            release_logoscore_wallet(cfg_provider)
            seed_close_stream_onchain(repo, manifest, vault_id, stream_id)
            log_artifact(artifact, "demo_close_stream", True, stream_id=stream_id, via="seed_close_stream_onchain")
            emit_module_phase(
                artifact,
                "close_stream",
                True,
                {"stream_id": stream_id, "via": "seed_close_stream_onchain"},
            )
        except (E2EError, subprocess.TimeoutExpired) as exc:
            log_artifact(
                artifact,
                "demo_close_stream",
                False,
                stream_id=stream_id,
                via="seed_close_stream_onchain",
                error=str(exc),
            )
            emit_module_phase(
                artifact,
                "close_stream",
                False,
                {"stream_id": stream_id, "via": "seed_close_stream_onchain", "error": str(exc)},
            )
        finally:
            reopen_logoscore_wallet(cfg_user, seq_url)
            reopen_logoscore_wallet(cfg_provider, seq_url)
            reload_payment_streams_wallet(cfg_user, seq_url)
            reload_payment_streams_wallet(cfg_provider, seq_url)
        inner = vault_status_json(cfg_user, manifest)
        total_lo = int((inner.get("vault_config") or {}).get("total_allocated_lo", stream_alloc))
        # On testnet the close tx can confirm after the seed subprocess returns/times out,
        # so poll the on-chain stream state before deciding whether to fall back to chainAction.
        closed_poll_attempts = 12 if os.environ.get("CHAIN", "local").strip().lower() == "testnet" else 1
        closed = False
        for poll_idx in range(closed_poll_attempts):
            closed = stream_closed_on_chain(cfg_user, manifest)
            if closed or poll_idx == closed_poll_attempts - 1:
                break
            time.sleep(20)
        close_applied = closed
        log_artifact(
            artifact,
            "demo_close_stream_verify",
            closed,
            stream_id=stream_id,
            via="seed_close_stream_onchain",
            total_allocated_lo=total_lo,
            stream_closed=closed,
        )
    if not close_applied:
        close_body = {
            "signer": manifest["owner_account_id"],
            "vault_id": vault_id,
            "stream_id": stream_id,
            "authority": manifest["provider_account_id"],
        }
        parsed: dict[str, Any] = {}
        for attempt in range(4):
            if attempt > 0:
                sync_wallet(cfg_user, seq_url)
                sync_wallet(cfg_provider, seq_url)
                time.sleep(3 * attempt)
            r = logoscore_cmd(
                cfg_provider,
                "call",
                "payment_streams_module",
                "chainAction",
                "closeStream",
                json.dumps(close_body),
                timeout=testnet_chain_action_timeout_s(),
            )
            parsed = call_result(r)
            ok_submit = chain_action_success(parsed)
            tx_close = chain_action_tx_hash(parsed)
            log_artifact(
                artifact,
                "demo_close_stream",
                ok_submit,
                stream_id=stream_id,
                attempt=attempt + 1,
                via="chainAction_closeStream",
                tx_hash=tx_close,
                raw=parsed if attempt == 0 else None,
            )
            if not ok_submit:
                continue
            if tx_close:
                await_chain_action_inclusion(seq_url, parsed, artifact, label="demo_close_stream")
            sync_wallet(cfg_user, seq_url)
            sync_wallet(cfg_provider, seq_url)
            logoscore_cmd(cfg_user, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))
            inner = vault_status_json(cfg_user, manifest)
            total_lo = int((inner.get("vault_config") or {}).get("total_allocated_lo", stream_alloc))
            closed = stream_closed_on_chain(cfg_user, manifest)
            if closed:
                close_applied = True
                emit_module_phase(
                    artifact,
                    "close_stream",
                    True,
                    {
                        "stream_id": stream_id,
                        "attempt": attempt + 1,
                        "via": "chainAction_closeStream",
                        "tx_hash": tx_close,
                    },
                )
                log_artifact(
                    artifact,
                    "demo_close_stream_verify",
                    True,
                    stream_id=stream_id,
                    attempt=attempt + 1,
                    total_allocated_lo=total_lo,
                    stream_closed=closed,
                )
                break
    if not close_applied:
        raise E2EError(
            f"demo_close_stream did not apply on chain after retries (last parsed={parsed!r})"
        )
    vb, tot = vault_status_balances(cfg_user, manifest)
    s_acc, s_unc, s_st = stream_status_fields(cfg_user, manifest, stream_id)
    emit_module_phase(
        artifact,
        "close_state",
        True,
        {
            "vault_balance": vb,
            "total_allocated": tot,
            "stream_accrued": s_acc,
            "stream_unaccrued": s_unc,
            "stream_state": s_st,
        },
    )
    log_vault_liquidity(cfg_user, manifest, artifact, phase="vault_liquidity_after_close")

    narrator.phase("Claim")
    accrued = stream_accrued_lo(cfg_user, vault_id, stream_id)
    if accrued <= 0:
        claim_extra = {"skipped": True, "reason": "zero_accrued", "stream_id": stream_id}
        emit_claim_with_demo_alias(artifact, True, claim_extra)
        return

    narrator.step(f"Claiming residual accrued ({accrued}) on closed stream {stream_id}")
    # Prefer the direct-submit claim path (same rationale as close: testnet chainAction is
    pre_provider = account_balance_seq(seq_url, manifest["provider_account_id"])
    pre_vault, _ = vault_status_balances(cfg_user, manifest)
    if os.environ.get("E2E_CLOSE_VIA", "seed").strip().lower() != "chainaction":
        try:
            release_logoscore_wallet(cfg_user)
            release_logoscore_wallet(cfg_provider)
            seed_claim_onchain(repo, manifest, vault_id, stream_id)
            log_artifact(
                artifact,
                "demo_claim",
                True,
                skipped=False,
                accrued_lo=accrued,
                via="seed_claim_onchain",
                stream_id=stream_id,
            )
            claim_extra = {
                "skipped": False,
                "accrued_lo": accrued,
                "via": "seed_claim_onchain",
                "stream_id": stream_id,
            }
            emit_claim_with_demo_alias(artifact, True, claim_extra)
        except (E2EError, subprocess.TimeoutExpired) as exc:
            log_artifact(
                artifact,
                "demo_claim",
                False,
                skipped=False,
                accrued_lo=accrued,
                via="seed_claim_onchain",
                error=str(exc),
                stream_id=stream_id,
            )
            emit_claim_with_demo_alias(
                artifact,
                False,
                {
                    "skipped": False,
                    "accrued_lo": accrued,
                    "via": "seed_claim_onchain",
                    "error": str(exc),
                    "stream_id": stream_id,
                },
            )
        finally:
            reopen_logoscore_wallet(cfg_user, seq_url)
            reopen_logoscore_wallet(cfg_provider, seq_url)
            reload_payment_streams_wallet(cfg_user, seq_url)
            reload_payment_streams_wallet(cfg_provider, seq_url)
        # The claim tx can confirm after the seed subprocess returns/times out; poll the
        # accrued balance (0 once claimed/settled) before falling back to chainAction.
        claim_poll_attempts = 12 if os.environ.get("CHAIN", "local").strip().lower() == "testnet" else 1
        for poll_idx in range(claim_poll_attempts):
            if stream_accrued_lo(cfg_user, vault_id, stream_id) < accrued:
                sync_wallet(cfg_user, seq_url)
                sync_wallet(cfg_provider, seq_url)
                logoscore_cmd(cfg_user, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))
                post_provider = account_balance_seq(seq_url, manifest["provider_account_id"])
                post_vault, _ = vault_status_balances(cfg_user, manifest)
                received = max(0, post_provider - pre_provider)
                vault_drop = pre_vault - post_vault
                bal_extra = {
                    "received": received,
                    "provider_pre": pre_provider,
                    "provider_post": post_provider,
                    "vault_pre": pre_vault,
                    "vault_post": post_vault,
                    "attempts": poll_idx + 1,
                }
                if received > 0 and vault_drop != received:
                    bal_extra["hint"] = "vault_drop_mismatch"
                emit_module_phase(artifact, "claim_balance", received > 0, bal_extra)
                log_vault_liquidity(cfg_user, manifest, artifact, phase="vault_liquidity_after_claim")
                return
            if poll_idx < claim_poll_attempts - 1:
                time.sleep(20)
        # Demo policy: claim is optional. The headline demo (create -> fundable ->
        # paid Store query -> close) does not need claim to recycle funds, and the
        # provider claim has not reliably confirmed on public testnet (see
        # docs/testnet-claim-known-issue.md). On testnet, or whenever E2E_CLAIM_OPTIONAL
        # is set, treat an unconfirmed claim as a pass instead of failing teardown.
        is_testnet = os.environ.get("CHAIN", "local").strip().lower() == "testnet"
        claim_optional = os.environ.get(
            "E2E_CLAIM_OPTIONAL", "1" if is_testnet else "0"
        ).strip().lower() not in ("0", "false", "no")
        if claim_optional:
            claim_extra = {
                "skipped": True,
                "optional": True,
                "claimed": False,
                "reason": "claim_optional_unconfirmed",
                "accrued_lo": accrued,
                "stream_id": stream_id,
            }
            emit_claim_with_demo_alias(artifact, True, claim_extra)
            return

    claim_body = {
        "owner": manifest["owner_account_id"],
        "provider": manifest["provider_account_id"],
        "vault_id": vault_id,
        "stream_id": stream_id,
    }
    r = logoscore_cmd(
        cfg_provider,
        "call",
        "payment_streams_module",
        "chainAction",
        "claim",
        json.dumps(claim_body),
        timeout=testnet_chain_action_timeout_s(),
    )
    parsed = call_result(r)
    ok_claim = chain_action_success(parsed)
    tx_claim = chain_action_tx_hash(parsed)
    claim_extra = {
        "skipped": False,
        "accrued_lo": accrued,
        "tx_hash": tx_claim,
        "stream_id": stream_id,
        "via": "chainAction_claim",
    }
    log_artifact(
        artifact,
        "demo_claim",
        ok_claim,
        skipped=False,
        accrued_lo=accrued,
        tx_hash=tx_claim,
        stream_id=stream_id,
    )
    emit_claim_with_demo_alias(artifact, ok_claim, claim_extra)
    if not ok_claim:
        raise E2EError(f"demo_claim failed: {parsed}")
    if tx_claim:
        await_chain_action_inclusion(seq_url, parsed, artifact, label="demo_claim")
    sync_wallet(cfg_user, seq_url)
    sync_wallet(cfg_provider, seq_url)
    logoscore_cmd(cfg_user, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))
    post_provider = account_balance_seq(seq_url, manifest["provider_account_id"])
    post_vault, _ = vault_status_balances(cfg_user, manifest)
    received = max(0, post_provider - pre_provider)
    vault_drop = pre_vault - post_vault
    bal_extra = {
        "received": received,
        "provider_pre": pre_provider,
        "provider_post": post_provider,
        "vault_pre": pre_vault,
        "vault_post": post_vault,
        "attempts": 1,
    }
    if received > 0 and vault_drop != received:
        bal_extra["hint"] = "vault_drop_mismatch"
    emit_module_phase(artifact, "claim_balance", received > 0, bal_extra)
    log_vault_liquidity(cfg_user, manifest, artifact, phase="vault_liquidity_after_claim")


def user_prepare_proof(
    cfg: Path,
    manifest: dict,
    n8_wire: str,
    provider_peer_id: str,
) -> str:
    provider_b58 = manifest["provider_account_id"]
    r = logoscore_cmd(
        cfg,
        "call",
        "payment_streams_module",
        "registerProviderMapping",
        provider_peer_id,
        provider_b58,
    )
    ensure_ok(call_result(r), "registerProviderMapping")

    chain = os.environ.get("CHAIN", "local").strip().lower()
    vault_id = int(manifest["vault_id"])
    stream_id = manifest_stream_id(manifest)
    seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
    allow_depleted = os.environ.get("PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF", "").strip().lower() in (
        "1",
        "true",
        "yes",
    )

    for attempt in range(8):
        sync_wallet(cfg, seq_url)
        logoscore_cmd(cfg, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))
        if not allow_depleted and not allocation_available(cfg, vault_id, stream_id, manifest):
            if chain == "testnet":
                raise E2EError(
                    "testnet stream has insufficient unaccrued allocation after create_demo_stream"
                )
            topup = {
                "signer": manifest["owner_account_id"],
                "vault_id": vault_id,
                "stream_id": stream_id,
                "increase_lo": default_topup_increase_lo(manifest) + attempt * 200,
                "increase_hi": 0,
            }
            logoscore_cmd(
                cfg,
                "call",
                "payment_streams_module",
                "chainAction",
                "topUpStream",
                json.dumps(topup),
                timeout=testnet_chain_action_timeout_s(),
            )
            continue

        r = logoscore_cmd(
            cfg,
            "call",
            "payment_streams_module",
            "prepareEligibilityProofWithStreamProofForStoreQuery",
            n8_wire,
            provider_peer_id,
            str(stream_id),
            timeout=240 if chain == "testnet" else 120,
        )
        parsed = call_result(r)
        if parsed.get("status") != "ok":
            raise E2EError(f"prepareEligibility: {parsed}")
        inner = json.loads(parsed["result"]) if isinstance(parsed.get("result"), str) else parsed.get("result", {})
        if inner.get("status") == "ok":
            return inner["bytes_hex"]
        if inner.get("code") == "STREAM_DEPLETED" and os.environ.get("E2E_ALLOW_TOPUP_RETRY", "").strip() == "1":
            continue
        raise E2EError(f"prepareEligibility inner: {inner}")
    raise E2EError("prepareEligibility failed after retries")


def wait_store_query(cfg: Path, query_json: str, provider_addr: str, log_path: Path) -> dict:
    log_path.parent.mkdir(parents=True, exist_ok=True)
    if log_path.is_file():
        log_path.unlink()

    watch = subprocess.Popen(
        ["logoscore", "--config-dir", str(cfg), "watch", "delivery_module"],
        stdout=log_path.open("w"),
        stderr=subprocess.STDOUT,
        text=True,
        start_new_session=True,
    )
    time.sleep(1)
    try:
        r = logoscore_cmd(cfg, "call", "delivery_module", "storeQuery", query_json, provider_addr)
        dispatch = call_result(r)
        if dispatch.get("status") != "ok":
            err_tail = ""
            err_file = log_path.parent / "logoscore-daemon.stderr"
            if err_file.is_file():
                err_tail = err_file.read_text()[-3000:]
            raise E2EError(f"storeQuery dispatch: {json.dumps(dispatch)} daemon_stderr={err_tail!r}")
        inner = dispatch.get("result")
        if isinstance(inner, str):
            try:
                inner = json.loads(inner)
            except json.JSONDecodeError:
                inner = {}
        if isinstance(inner, dict) and inner.get("success") is False:
            raise E2EError(f"storeQuery dispatch: {inner.get('error') or inner}")

        deadline = time.time() + STORE_QUERY_TIMEOUT_S
        blob = ""
        while time.time() < deadline:
            if log_path.is_file():
                blob = log_path.read_text()
                if "storeQueryCompleted" in blob:
                    break
            time.sleep(0.5)
        if "storeQueryCompleted" not in blob:
            raise E2EError(f"timeout waiting storeQueryCompleted; log={blob[-2000:]!r}")
        return parse_store_query_completed(blob)
    finally:
        watch.terminate()
        try:
            watch.wait(timeout=5)
        except subprocess.TimeoutExpired:
            watch.kill()


def provider_verify_verdict(cfg_provider: Path, proof_hex: str, n8_wire: str) -> dict:
    """Directly invoke the provider verifier to surface the eligibility code/desc.

    The client only ever sees ``BAD_REQUEST``; the verdict (code + message) rides the wire
    response's eligibility status and is otherwise invisible. Calling the already-installed
    module method reproduces the inbound verify decision without a rebuild.
    """
    try:
        r = logoscore_cmd(
            cfg_provider,
            "call",
            "payment_streams_module",
            "verifyEligibilityForStoreQuery",
            proof_hex,
            n8_wire,
            "e2e-diagnostic",
        )
        parsed = call_result(r)
        inner = parsed.get("result")
        if isinstance(inner, str):
            try:
                inner = json.loads(inner)
            except json.JSONDecodeError:
                return {"raw": inner}
        return inner if isinstance(inner, dict) else {"raw": inner}
    except E2EError as e:
        return {"error": str(e)}


def parse_store_query_completed(blob: str) -> dict:
    for line in reversed(blob.splitlines()):
        if "storeQueryCompleted" not in line:
            continue
        try:
            obj = json.loads(line.strip())
        except json.JSONDecodeError:
            obj = None
        if isinstance(obj, dict):
            data = obj.get("data")
            if isinstance(data, dict):
                success = data.get("arg0", data.get("success"))
                resp_raw = data.get("arg1", data.get("responseJson", ""))
                if isinstance(resp_raw, str) and resp_raw.strip().startswith("{"):
                    try:
                        return json.loads(resp_raw)
                    except json.JSONDecodeError:
                        pass
                if success is False and isinstance(resp_raw, str):
                    return {"statusCode": 0, "error": resp_raw, "messages": []}
            if "responseJson" in obj or "messages" in obj:
                return obj
        for m in re.finditer(r"\{.*\}", line):
            try:
                inner = json.loads(m.group(0))
            except json.JSONDecodeError:
                continue
            if inner.get("event") == "storeQueryCompleted" and isinstance(inner.get("data"), dict):
                data = inner["data"]
                resp_raw = data.get("arg1", "")
                if isinstance(resp_raw, str) and resp_raw.strip().startswith("{"):
                    return json.loads(resp_raw)
                if data.get("arg0") is False:
                    return {"statusCode": 0, "error": str(resp_raw), "messages": []}
            if "responseJson" in inner or "messages" in inner:
                return inner
    raise E2EError(f"could not parse storeQueryCompleted from {blob[-1500:]!r}")


def find_ps_state_file(persist_root: Path) -> Path:
    matches = sorted(persist_root.glob("**/payment_streams_state.json"))
    if not matches:
        raise E2EError(f"no payment_streams_state.json under {persist_root}")
    for path in matches:
        if "payment_streams_module" in path.parts:
            return path
    return matches[0]


def provider_ps_state_path(persist_provider: Path) -> Path:
    """Resolve the provider's payment_streams_state.json path, creating the
    instance dir if no state file exists yet. The module writes state into a
    payment_streams_module/<instance_hash>/ subdirectory; seed there so the
    reload after seeding picks up the acceptance row."""
    module_root = persist_provider / "payment_streams_module"
    existing = sorted(module_root.glob("*/payment_streams_state.json"))
    if existing:
        return existing[0]
    # No state file yet: pick the first instance dir (or create one) and return
    # the canonical path. seed_provider_acceptance.py writes the file.
    instance_dirs = [d for d in module_root.iterdir() if d.is_dir()] if module_root.is_dir() else []
    if instance_dirs:
        instance_dir = instance_dirs[0]
    else:
        instance_dir = module_root / uuid.uuid4().hex[:12]
    instance_dir.mkdir(parents=True, exist_ok=True)
    return instance_dir / "payment_streams_state.json"


def message_count(response: dict) -> int:
    if "messages" in response:
        return len(response["messages"])
    if "responseJson" in response:
        inner = response["responseJson"]
        if isinstance(inner, str):
            inner = json.loads(inner)
        return len(inner.get("messages", []))
    return 0


def store_status_code(response: dict) -> int | None:
    body = response
    if "responseJson" in body and isinstance(body["responseJson"], str):
        body = json.loads(body["responseJson"])
    sc = body.get("statusCode") or body.get("status")
    if isinstance(sc, int):
        return sc
    if isinstance(sc, str) and sc.isdigit():
        return int(sc)
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", type=Path, default=Path.cwd())
    ap.add_argument("--phase", choices=("core", "claim", "all"), default="all")
    ap.add_argument("--artifact", type=Path, required=True)
    ap.add_argument(
        "--verbosity",
        choices=("quiet", "normal", "verbose"),
        default=os.environ.get("E2E_VERBOSITY", "verbose"),
    )
    args = ap.parse_args()

    global narrator
    narrator = Narrator(args.verbosity)

    narrator.header(
        "Scenario: User proves stream eligibility to Provider\n"
        f"[{time.strftime('%H:%M:%S')}]          for paid Store query (LIP-155, RFC 73 tag 30)"
    )

    repo = args.repo.resolve()
    artifact = args.artifact
    artifact.parent.mkdir(parents=True, exist_ok=True)
    if artifact.exists():
        artifact.unlink()
    timer = RunTimer(artifact)

    e2e = repo / ".scaffold" / "e2e"
    modules_user = Path(os.environ.get("MODULES_USER", e2e / "user" / "modules"))
    modules_provider = Path(os.environ.get("MODULES_PROVIDER", e2e / "provider" / "modules"))
    cfg_user = Path(os.environ.get("LOGOSCORE_CONFIG_USER", e2e / "user" / "logoscore"))
    cfg_provider = Path(os.environ.get("LOGOSCORE_CONFIG_PROVIDER", e2e / "provider" / "logoscore"))
    persist_user = Path(os.environ.get("PERSIST_USER", e2e / "user" / "persist"))
    persist_provider = Path(os.environ.get("PERSIST_PROVIDER", e2e / "provider" / "persist"))
    manifest_path = Path(os.environ.get("FIXTURE_MANIFEST", repo / "fixtures" / "localnet.json"))
    wallet_config = Path(os.environ.get("WALLET_CONFIG", repo / ".scaffold" / "wallet" / "wallet_config.json"))
    wallet_storage = Path(os.environ.get("WALLET_STORAGE", repo / ".scaffold" / "wallet" / "storage.json"))
    guest_bin = Path(
        os.environ.get(
            "PAYMENT_STREAMS_GUEST_BIN",
            repo / "methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin",
        )
    )
    provider_ad = Path(os.environ.get("E2E_PROVIDER_AD", e2e / "provider-advertisement.json"))

    os.environ["PAYMENT_STREAMS_GUEST_BIN"] = str(guest_bin)
    os.environ["REPO"] = str(repo)
    os.environ["FIXTURE_MANIFEST"] = str(manifest_path)

    manifest = json.loads(manifest_path.read_text())
    if not store_reuse_baseline_vault():
        narrator.phase("Vault Ensure")
        deposit_lo = seed_deposit_amount_lo(manifest)
        narrator.step("Ensuring fresh vault for this run (init + deposit)")
        narrator.value(f"target deposit={deposit_lo} lo (allocation + buffer)")
        ensure_fresh_vault_for_store_run(repo, manifest_path, manifest, artifact)
        manifest.clear()
        manifest.update(json.loads(manifest_path.read_text()))
        narrator.ok(f"Vault {manifest.get('vault_id')} ready on chain")
    log_artifact(
        artifact,
        "run_config",
        True,
        e2e_subproc_timeout_s=e2e_subprocess_timeout_s(),
        stream_fundable_wait_s=stream_fundable_wait_s(),
        publish_wait_s=PUBLISH_WAIT_S,
        create_via=local_e2e_create_via() if os.environ.get("CHAIN", "local").strip().lower() == "local" else testnet_e2e_create_via(),
        skip_build=os.environ.get("SKIP_BUILD", ""),
        **wallet_tx_poll_budget_s(wallet_config),
    )
    if os.environ.get("CHAIN", "local").strip().lower() == "local":
        strip_snapshot_stream_fields(manifest, manifest_path)
    n8_wire = os.environ.get("N8_WIRE_HEX", "").strip()
    if not n8_wire:
        narrator.phase("Environment Setup")
        narrator.step("Computing N8 canonical wire format")
        n8_proc = run(
            ["cargo", "run", "-q", "--release", "-p", "lez-payment-streams-core", "--bin", "n8_canonical_wire_hex"],
            cwd=repo,
            timeout=120,
        )
        if n8_proc.returncode != 0:
            log_artifact(artifact, "n8_wire", False, error=n8_proc.stderr)
            narrator.fail("N8 wire computation failed")
            narrator.hint("Check lez-payment-streams-core builds successfully")
            return 1
        n8_wire = n8_proc.stdout.strip()
        narrator.ok("N8 wire computed")
    timer.mark("n8_wire_ready")

    user_ports_shift = 0
    provider_ports_shift = 100
    provider_tcp = 60000 + provider_ports_shift

    try:
        for d in (modules_user, modules_provider, cfg_user, cfg_provider, persist_user, persist_provider):
            d.mkdir(parents=True, exist_ok=True)

        if args.phase in ("core", "all"):
            reset_payment_streams_module_persist(persist_user, persist_provider)

        if args.phase in ("core", "all"):
            # --- Provider daemon ---
            narrator.phase("Environment Setup")
            narrator.step("Starting provider logoscore, loading modules")
            start_daemon(cfg_provider, modules_provider, persist_provider)
            load_modules(cfg_provider)
            open_wallet(cfg_provider, wallet_config, wallet_storage)
            sync_wallet(cfg_provider, manifest.get("sequencer_url", "http://127.0.0.1:3040"))

            provider_create = {
                "mode": "Core",
                **local_waku_json_base(),
                "portsShift": provider_ports_shift,
                "relay": True,
                "store": True,
                "storeMessageRetentionPolicy": "capacity:10000",
                "storeMessageDbUrl": f"sqlite://{(persist_provider / 'store.sqlite3').as_posix()}",
            }
            delivery_create_start(
                cfg_provider, provider_create, persist=persist_provider, label="provider"
            )
            set_eligibility_verifier(cfg_provider, "payment_streams_module")

            peer_id = get_node_info(cfg_provider, "MyPeerId")
            provider_addr = loopback_multiaddr(peer_id, provider_tcp)

            ad = {
                "provider_peer_id": peer_id,
                "provider_store_multiaddr": provider_addr,
                "content_topic": CONTENT_TOPIC,
                "service_id": "/vac/waku/store-query/3.0.0",
            }
            provider_ad.write_text(json.dumps(ad, indent=2) + "\n")
            log_artifact(artifact, "provider_ad", True, **ad)
            timer.mark("provider_delivery_up")
            narrator.ok("Provider ready: delivery_module (Store enabled), payment_streams_module")
            narrator.value(f"provider peer_id={peer_id}, store multiaddr={provider_addr}")

            # --- User daemon ---
            narrator.step("Starting user logoscore, loading modules")
            start_daemon(cfg_user, modules_user, persist_user)
            load_modules(cfg_user)
            open_wallet(cfg_user, wallet_config, wallet_storage)
            sync_wallet(cfg_user, manifest.get("sequencer_url", "http://127.0.0.1:3040"))

            user_create = {
                "mode": "Core",
                **local_waku_json_base(),
                "portsShift": user_ports_shift,
                "relay": True,
                "store": False,
                "staticnodes": [provider_addr],
                "storenode": provider_addr,
            }
            delivery_create_start(cfg_user, user_create, persist=persist_user, label="user")
            # Outbound proof via eligibilityProofHex in query JSON (hook deadlocks Approach A).
            narrator.ok("User ready: delivery_module (Store client), payment_streams_module")

            seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
            sync_wallet(cfg_user, seq_url)
            sync_wallet(cfg_provider, seq_url)

            wallet_home = Path(wallet_config).parent
            narrator.step("Ensuring owner and provider accounts under authenticated_transfer")
            run_auth_transfer_ensure(
                repo, cfg_user, cfg_provider, manifest, artifact, wallet_home
            )
            narrator.ok("authenticated_transfer ensure complete (see auth_init_* in artifact)")

            if (
                os.environ.get("CHAIN", "local").strip().lower() == "local"
                and continuation_e2e_run()
                and not os.environ.get("E2E_PRECREATED_STREAM_ID", "").strip()
            ):
                ensure_sequencer_advancing(repo, seq_url, artifact)
                sync_wallet(cfg_user, seq_url)
                precreate_stream_before_daemons(
                    repo, manifest_path, manifest, artifact, wallet_config, cfg_user
                )

            if os.environ.get("E2E_PRECREATED_STREAM_ID", "").strip():
                create_demo_stream_for_run(
                    cfg_user,
                    cfg_provider,
                    repo,
                    manifest_path,
                    manifest,
                    persist_user,
                    artifact,
                    wallet_config=wallet_config,
                )
                timer.mark("create_demo_stream_done")

            if not os.environ.get("E2E_PRECREATED_STREAM_ID", "").strip():
                narrator.phase("Stream Creation")
                vault_id = int(manifest.get("vault_id", 0))
                rate = int(manifest.get("stream_rate", 1))
                alloc = manifest_allocation_lo(manifest)
                narrator.step("User creates payment stream to provider")
                narrator.value(f"rate={rate} token/sec, allocation={alloc} tokens, vault={vault_id}")
                create_demo_stream_for_run(
                    cfg_user,
                    cfg_provider,
                    repo,
                    manifest_path,
                    manifest,
                    persist_user,
                    artifact,
                    wallet_config=wallet_config,
                )
                timer.mark("create_demo_stream_done")
                stream_id = manifest_stream_id(manifest)
                narrator.ok(f"Stream created: stream_id={stream_id}, status=ACTIVE")
                min_unaccrued = min_unaccrued_lo_for_proof(manifest)
                narrator.step(f"Waiting for stream to become fundable (need ≥{min_unaccrued} unaccrued tokens)")
                narrator.concept("Accrual is timestamp-based: derived from on-chain")
                narrator.concept("accrued_as_of field. On testnet, granularity is limited")
                narrator.concept("by block time.")
            else:
                stream_id = manifest_stream_id(manifest)

            narrator.step("Publishing test messages to Store...")
            logoscore_cmd(cfg_user, "call", "delivery_module", "subscribe", CONTENT_TOPIC)
            logoscore_cmd(cfg_provider, "call", "delivery_module", "subscribe", CONTENT_TOPIC)
            payload = f"e2e-{uuid.uuid4().hex[:8]}"
            logoscore_cmd(cfg_user, "call", "delivery_module", "send", CONTENT_TOPIC, payload)
            publish_wait = PUBLISH_WAIT_S
            if continuation_e2e_run():
                publish_wait = int(os.environ.get("E2E_CONTINUATION_PUBLISH_WAIT_S", "5"))
            time.sleep(publish_wait)
            narrator.ok("Messages published and propagated")
            sync_wallet(cfg_user, seq_url)
            sync_wallet(cfg_provider, seq_url)
            timer.mark("messaging_publish_wait")

            # Mint proof immediately before storeQuery so provider verify still sees unaccrued balance.
            narrator.phase("Eligibility Proof Generation")
            narrator.step("User generates LIP-155 eligibility proof from active stream")
            narrator.concept("Proof derives from on-chain stream state, valid for current block")
            proof_hex = user_prepare_proof(cfg_user, manifest, n8_wire, peer_id)
            narrator.ok(f"Proof generated: {len(proof_hex) // 2} bytes, stream_id={stream_id}")
            seed_provider_session_from_user(persist_user, persist_provider, manifest_path, repo)
            reload_provider_payment_streams_module(cfg_provider)
            sync_wallet(cfg_provider, seq_url)
            vault_id = int(manifest.get("vault_id", 0))
            logoscore_cmd(
                cfg_provider, "call", "payment_streams_module", "rediscoverStreams", str(vault_id)
            )

            narrator.phase("Paid Store Query")
            narrator.step("User sends Store query with eligibility proof attached")
            query = dict(N8_REFERENCE_QUERY)
            query["eligibilityProofHex"] = proof_hex
            query_json = json.dumps(query, separators=(",", ":"))
            watch_log = persist_user / "store-query-events.log"
            response = None
            last_err = ""
            for attempt in range(STORE_QUERY_RETRIES):
                if attempt > 0:
                    time.sleep(10)
                try:
                    response = wait_store_query(cfg_user, query_json, provider_addr, watch_log)
                    if "PEER_DIAL_FAILURE" in str(response.get("error", "")):
                        last_err = str(response.get("error"))
                        continue
                    break
                except E2EError as e:
                    last_err = str(e)
                    if "PEER_DIAL_FAILURE" in last_err:
                        continue
                    raise
            if response is None:
                raise E2EError(f"storeQuery failed after retries: {last_err}")
            mc = message_count(response)
            sc = store_status_code(response)
            store_ok = mc > 0 or sc == 200
            log_artifact(
                artifact,
                "store_query_success",
                store_ok,
                message_count=mc,
                status=sc,
                response_preview=str(response)[:500],
            )
            if store_ok:
                narrator.step("Provider verifies proof against LEZ on-chain state")
                narrator.ok(f"Proof valid: stream active, serving historical messages")
                narrator.ok(f"Store query returned {mc} messages, status {sc}")
            else:
                narrator.fail(f"Store query failed: {mc} messages, status {sc}")
                narrator.hint("Check provider verifier, stream state, and proof freshness")
                verdict = provider_verify_verdict(cfg_provider, proof_hex, n8_wire)
                log_artifact(
                    artifact,
                    "store_query_eligibility_verdict",
                    False,
                    eligibility=verdict.get("eligibility"),
                    message=verdict.get("message"),
                    verdict=verdict,
                )
                raise E2EError(
                    f"expected store query success, got {response!r}; provider verdict={verdict!r}"
                )

            # Missing proof (no eligibilityProofHex; provider verifier enabled)
            narrator.phase("Rejection Path")
            narrator.step("User sends Store query without eligibility proof")
            fail_query = dict(N8_REFERENCE_QUERY)
            fail_query["requestId"] = uuid.uuid4().hex[:16]
            fail_json = json.dumps(fail_query, separators=(",", ":"))
            fail_log = persist_user / "store-query-fail-events.log"
            fail_resp = wait_store_query(cfg_user, fail_json, provider_addr, fail_log)
            status = store_status_code(fail_resp)
            mc_fail = message_count(fail_resp)
            ok_fail = mc_fail == 0 and (status == 400 or status is None)
            log_artifact(
                artifact,
                "store_query_missing_proof",
                ok_fail,
                status=status,
                message_count=mc_fail,
            )
            if ok_fail:
                narrator.ok(f"Query rejected (no eligibility proof), {mc_fail} messages returned")
            else:
                narrator.fail(f"Missing-proof path unexpected: {mc_fail} messages, status {status}")
                narrator.hint("Provider verifier may not be enabled or configured correctly")
                raise E2EError(f"missing-proof path unexpected: {fail_resp!r}")

            narrator.phase("Settlement")
            demo_teardown(cfg_user, cfg_provider, manifest, artifact, repo, narrator)
            timer.mark("core_teardown_done")
            narrator.ok("Stream closed, residual claim verified, vault liquidity checked")

        if args.phase == "claim":
            log_artifact(
                artifact,
                "claim_phase",
                True,
                skipped=True,
                note="teardown runs at end of core (Step 24c)",
            )

    except E2EError as e:
        timer.mark("fatal")
        log_artifact(artifact, "fatal", False, error=str(e))
        narrator.fail(str(e))
        narrator.hint("Check artifact log for full phase details")
        narrator.complete_fail(artifact, 1)
        return 1
    finally:
        log_artifact(
            artifact,
            "run_total",
            True,
            elapsed_s=round(time.monotonic() - timer.t0, 2),
        )
        stop_daemon(cfg_user)
        stop_daemon(cfg_provider)

    narrator.complete(artifact)
    return 0


if __name__ == "__main__":
    sys.exit(main())
