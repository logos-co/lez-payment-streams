#!/usr/bin/env python3
"""Quick local sequencer health: RPC latency and block production over a short window."""
from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path


def rpc(url: str, method: str, params: list) -> tuple[float, object]:
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"})
    t0 = time.monotonic()
    with urllib.request.urlopen(req, timeout=10) as resp:
        payload = json.loads(resp.read().decode())
    ms = (time.monotonic() - t0) * 1000.0
    if "error" in payload:
        raise RuntimeError(f"{method}: {payload['error']}")
    return ms, payload["result"]


def load_wallet_poll_budget(repo: Path) -> dict:
    cfg = Path(os.environ.get("WALLET_CONFIG", repo / ".scaffold" / "wallet" / "wallet_config.json"))
    poll_cap_s = 8.0
    max_attempts = 22
    cap_raw = os.environ.get("E2E_WALLET_POLL_MAX_DELAY", "8s").strip()
    if cap_raw.endswith("s"):
        try:
            poll_cap_s = float(cap_raw[:-1])
        except ValueError:
            pass
    attempts_raw = os.environ.get("E2E_WALLET_POLL_MAX_ATTEMPTS", "22").strip()
    try:
        max_attempts = max(5, int(attempts_raw))
    except ValueError:
        pass
    if cfg.is_file():
        data = json.loads(cfg.read_text())
        if not os.environ.get("E2E_WALLET_POLL_MAX_DELAY"):
            raw = data.get("seq_poll_timeout", "12s")
            if isinstance(raw, str) and raw.endswith("s"):
                poll_cap_s = float(raw[:-1])
        if not os.environ.get("E2E_WALLET_POLL_MAX_ATTEMPTS"):
            max_attempts = int(data.get("seq_tx_poll_max_blocks", max_attempts))
    min_delay_s = 0.25
    delay = min_delay_s
    worst_s = 0.0
    for _ in range(max(0, max_attempts - 1)):
        worst_s += delay
        delay = min(delay * 2.0, poll_cap_s)
    return {
        "wallet_config": str(cfg),
        "seq_tx_poll_max_blocks": max_attempts,
        "seq_poll_timeout_s": poll_cap_s,
        "tx_poll_worst_case_s": round(worst_s, 1),
        "tx_poll_backoff": "exp_min_250ms_cap_seq_poll_timeout",
        "note": "E2E seed copies wallet and applies E2E_WALLET_POLL_* overrides before create",
    }


def main() -> int:
    repo = Path(os.environ.get("REPO", Path.cwd())).resolve()
    base = os.environ.get("SEQUENCER_URL", "http://127.0.0.1:3040").rstrip("/")
    window = float(os.environ.get("SEQ_PROBE_WINDOW_S", "8"))

    print(f"sequencer_url={base}")
    budget = load_wallet_poll_budget(repo)
    print("wallet_tx_poll_budget:", json.dumps(budget, indent=2))

    try:
        health_ms, _ = rpc(base, "checkHealth", [])
        block_ms, b0 = rpc(base, "getLastBlockId", [])
    except (urllib.error.URLError, RuntimeError, TimeoutError) as e:
        print(f"ERROR: sequencer not reachable: {e}", file=sys.stderr)
        return 1

    print(f"checkHealth_ms={health_ms:.1f}")
    print(f"getLastBlockId_ms={block_ms:.1f} block_id={b0}")

    time.sleep(window)
    _, b1 = rpc(base, "getLastBlockId", [])
    delta = int(b1) - int(b0)
    rate = delta / window if window > 0 else 0.0
    print(f"block_delta_over_{window}s={delta} (~{rate:.3f} blocks/s)")

    if delta == 0:
        print(
            "WARN: no new blocks in probe window — wallet create may hit "
            "'Transaction not found in preconfigured amount of blocks' even if tx was submitted.",
            file=sys.stderr,
        )
    elif rate > 0.5:
        print("blocks are advancing quickly; long E2E waits are likely harness retries/timeouts, not block time.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
