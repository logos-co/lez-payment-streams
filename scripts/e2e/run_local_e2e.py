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
from typing import Any


CONTENT_TOPIC = "/my-app/1/chat/proto"
N8_REFERENCE_QUERY = {
    "requestId": "req-1",
    "includeData": True,
    "pubsubTopic": "/waku/2/topic",
    "contentTopics": [CONTENT_TOPIC],
    "timeStart": 10,
    "paginationForward": True,
    "paginationLimit": 100,
    "messageHashes": ["0101010101010101010101010101010101010101010101010101010101010101",
                      "0202020202020202020202020202020202020202020202020202020202020202"],
}
STORE_QUERY_TIMEOUT_S = 120
PUBLISH_WAIT_S = 15
PEER_MESH_WAIT_S = 45
STORE_QUERY_RETRIES = 4
DAEMON_START_WAIT_S = 6


class E2EError(Exception):
    pass


def log_artifact(artifact: Path, phase: str, ok: bool, **fields: Any) -> None:
    row = {"phase": phase, "ok": ok, **fields}
    with artifact.open("a") as f:
        f.write(json.dumps(row, separators=(",", ":")) + "\n")


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
            '{"jsonrpc":"2.0","id":1,"method":"getBlockHeight","params":[]}',
        ],
        timeout=30,
    )
    if proc.returncode != 0 or not proc.stdout.strip():
        return None
    try:
        data = json.loads(proc.stdout)
        result = data.get("result")
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


def stream_allocation_available(cfg: Path, vault_id: int, stream_id: int) -> bool:
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
        if int(row.get("stream_id", -1)) != stream_id:
            continue
        unaccrued = int(row.get("unaccrued_lo", 0))
        if unaccrued > 0:
            return True
        accrued = int(row.get("accrued_lo", 0))
        # stream_state 0 often means active; treat low accrued as funds available
        if accrued < 70:
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
            "-q",
        ],
        stdout=subprocess.DEVNULL,
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

    topup = {
        "signer": manifest["owner_account_id"],
        "vault_id": int(manifest["vault_id"]),
        "stream_id": int(manifest["stream_id"]),
        "increase_lo": 200,
        "increase_hi": 0,
    }
    allow_depleted = os.environ.get("PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF", "").strip().lower() in (
        "1",
        "true",
        "yes",
    )
    vault_id = int(manifest["vault_id"])
    stream_id = int(manifest["stream_id"])
    for attempt in range(8):
        logoscore_cmd(cfg, "call", "payment_streams_module", "chainAction", "topUpStream", json.dumps(topup))
        for _ in range(12):
            sync_wallet(cfg, manifest.get("sequencer_url", "http://127.0.0.1:3040"))
            logoscore_cmd(cfg, "call", "payment_streams_module", "rediscoverStreams", str(vault_id))
            time.sleep(2)
            if allow_depleted or stream_allocation_available(cfg, vault_id, stream_id):
                break

        r = logoscore_cmd(
            cfg,
            "call",
            "payment_streams_module",
            "prepareEligibilityForStoreQuery",
            n8_wire,
            provider_peer_id,
        )
        parsed = call_result(r)
        if parsed.get("status") != "ok":
            raise E2EError(f"prepareEligibility: {parsed}")
        inner = json.loads(parsed["result"]) if isinstance(parsed.get("result"), str) else parsed.get("result", {})
        if inner.get("status") == "ok":
            return inner["bytes_hex"]
        if inner.get("code") == "STREAM_DEPLETED":
            topup["increase_lo"] = int(topup.get("increase_lo", 200)) + 200
            continue
        raise E2EError(f"prepareEligibility inner: {inner}")
    raise E2EError("prepareEligibility: stream still depleted after top-up retries")


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
    args = ap.parse_args()

    repo = args.repo.resolve()
    artifact = args.artifact
    artifact.parent.mkdir(parents=True, exist_ok=True)
    if artifact.exists():
        artifact.unlink()

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
    n8_wire = os.environ.get("N8_WIRE_HEX", "").strip()
    if not n8_wire:
        n8_proc = run(
            ["cargo", "run", "-q", "-p", "lez-payment-streams-core", "--bin", "n8_canonical_wire_hex"],
            cwd=repo,
            timeout=120,
        )
        if n8_proc.returncode != 0:
            log_artifact(artifact, "n8_wire", False, error=n8_proc.stderr)
            return 1
        n8_wire = n8_proc.stdout.strip()

    user_ports_shift = 0
    provider_ports_shift = 100
    provider_tcp = 60000 + provider_ports_shift

    try:
        for d in (modules_user, modules_provider, cfg_user, cfg_provider, persist_user, persist_provider):
            d.mkdir(parents=True, exist_ok=True)

        if args.phase in ("core", "all"):
            # --- Provider daemon ---
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

            # --- User daemon ---
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

            proof_hex = user_prepare_proof(cfg_user, manifest, n8_wire, peer_id)
            user_state = find_ps_state_file(persist_user)
            provider_state = find_ps_state_file(persist_provider)
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
            logoscore_cmd(cfg_provider, "unload-module", "payment_streams_module")
            logoscore_cmd(cfg_provider, "load-module", "payment_streams_module")

            seq_url = manifest.get("sequencer_url", "http://127.0.0.1:3040")
            sync_wallet(cfg_user, seq_url)
            sync_wallet(cfg_provider, seq_url)

            logoscore_cmd(cfg_user, "call", "delivery_module", "subscribe", CONTENT_TOPIC)
            logoscore_cmd(cfg_provider, "call", "delivery_module", "subscribe", CONTENT_TOPIC)
            payload = f"e2e-{uuid.uuid4().hex[:8]}"
            logoscore_cmd(cfg_user, "call", "delivery_module", "send", CONTENT_TOPIC, payload)
            time.sleep(PUBLISH_WAIT_S)
            time.sleep(15)

            sync_wallet(cfg_user, seq_url)
            sync_wallet(cfg_provider, seq_url)

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
            if not store_ok:
                raise E2EError(f"expected store query success, got {response!r}")

            # Missing proof (no eligibilityProofHex; provider verifier enabled)
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
            if not ok_fail:
                raise E2EError(f"missing-proof path unexpected: {fail_resp!r}")

        if args.phase in ("claim", "all"):
            claim_body = {
                "provider": manifest["provider_account_id"],
                "vault_id": int(manifest["vault_id"]),
                "stream_id": int(manifest["stream_id"]),
            }
            r = logoscore_cmd(
                cfg_provider,
                "call",
                "payment_streams_module",
                "chainAction",
                "claim",
                json.dumps(claim_body),
            )
            parsed = call_result(r)
            ok_claim = parsed.get("status") == "ok"
            tx = None
            if ok_claim and isinstance(parsed.get("result"), str):
                inner = json.loads(parsed["result"])
                ok_claim = inner.get("success", False)
                tx = inner.get("tx_hash")
            log_artifact(artifact, "claim", ok_claim, tx_hash=tx, raw=parsed)
            if not ok_claim:
                raise E2EError(f"claim failed: {parsed}")

    except E2EError as e:
        log_artifact(artifact, "fatal", False, error=str(e))
        print(f"E2E failed: {e}", file=sys.stderr)
        return 1
    finally:
        stop_daemon(cfg_user)
        stop_daemon(cfg_provider)

    print(f"E2E OK — artifact {artifact}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
