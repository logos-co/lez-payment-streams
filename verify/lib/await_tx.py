#!/usr/bin/env python3
"""Sequencer inclusion wait shared by Store and module E2E (D50.2).

Wall-clock budget: E2E_TX_ONCHAIN_WAIT_S (default 110s).
Poll interval: E2E_TX_ONCHAIN_POLL_S (default 0.5s, exponential backoff, cap 5s).
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


DEFAULT_WAIT_S = 110
DEFAULT_POLL_S = 0.5
DEFAULT_SEQ_URL = "http://127.0.0.1:3040"


class AwaitTxError(Exception):
    def __init__(
        self,
        message: str,
        *,
        tx_hash: str = "",
        attempts: int = 0,
        elapsed_s: float = 0.0,
    ) -> None:
        super().__init__(message)
        self.tx_hash = tx_hash
        self.attempts = attempts
        self.elapsed_s = elapsed_s


def normalize_tx_hash(tx_hash: str) -> str:
    h = tx_hash.strip().lower().removeprefix("0x")
    if len(h) != 64 or any(c not in "0123456789abcdef" for c in h):
        raise AwaitTxError(f"invalid tx_hash for wait: {tx_hash!r}", tx_hash=h)
    return h


def tx_onchain_wait_s() -> float:
    explicit = os.environ.get("E2E_TX_ONCHAIN_WAIT_S", "").strip()
    if explicit:
        try:
            return float(max(30, int(explicit)))
        except ValueError:
            pass
    return float(DEFAULT_WAIT_S)


def sequencer_get_transaction(seq_url: str, tx_hash: str) -> Any:
    body = json.dumps(
        {"jsonrpc": "2.0", "id": 1, "method": "getTransaction", "params": [tx_hash]}
    ).encode()
    req = urllib.request.Request(
        seq_url.rstrip("/"),
        data=body,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=15) as resp:
        payload = json.loads(resp.read().decode())
    if "error" in payload:
        raise AwaitTxError(f"sequencer RPC getTransaction: {payload['error']}")
    return payload.get("result")


def wait_for_sequencer_tx(
    seq_url: str,
    tx_hash: str,
    *,
    wait_s: float | None = None,
    poll_s: float | None = None,
) -> dict[str, Any]:
    h = normalize_tx_hash(tx_hash)
    budget = float(tx_onchain_wait_s() if wait_s is None else wait_s)
    poll = DEFAULT_POLL_S if poll_s is None else float(poll_s)
    env_poll = os.environ.get("E2E_TX_ONCHAIN_POLL_S", "").strip()
    if poll_s is None and env_poll:
        try:
            poll = float(env_poll)
        except ValueError:
            poll = DEFAULT_POLL_S
    t0 = time.monotonic()
    delay = max(0.25, poll)
    attempt = 0
    while time.monotonic() - t0 <= budget:
        attempt += 1
        try:
            found = sequencer_get_transaction(seq_url, h)
        except (AwaitTxError, OSError, urllib.error.URLError, TimeoutError, json.JSONDecodeError):
            found = None
        if found is not None:
            return {
                "tx_hash": h,
                "attempts": attempt,
                "elapsed_s": round(time.monotonic() - t0, 2),
            }
        time.sleep(delay)
        delay = min(delay * 1.5, 5.0)
    elapsed = round(time.monotonic() - t0, 2)
    raise AwaitTxError(
        f"transaction {h} never appeared on sequencer (waited {budget}s)",
        tx_hash=h,
        attempts=attempt,
        elapsed_s=elapsed,
    )


def _append_artifact(path: Path, ok: bool, payload: dict[str, Any]) -> None:
    row = {"phase": "wait_tx_on_chain", "ok": ok, **payload}
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(row) + "\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="await_tx")
    parser.add_argument("tx_hash")
    parser.add_argument("--seq-url", default=os.environ.get("SEQUENCER_URL", DEFAULT_SEQ_URL))
    parser.add_argument("--wait-s", type=float, default=None)
    parser.add_argument("--poll-s", type=float, default=None)
    parser.add_argument("--artifact", default="", help="optional JSONL path")
    args = parser.parse_args(argv)
    artifact = Path(args.artifact) if args.artifact else None
    try:
        result = wait_for_sequencer_tx(
            args.seq_url,
            args.tx_hash,
            wait_s=args.wait_s,
            poll_s=args.poll_s,
        )
    except AwaitTxError as exc:
        if artifact is not None:
            _append_artifact(
                artifact,
                False,
                {
                    "tx_hash": exc.tx_hash,
                    "attempts": exc.attempts,
                    "elapsed_s": exc.elapsed_s,
                    "error": str(exc),
                },
            )
        print(str(exc), file=sys.stderr)
        return 1
    if artifact is not None:
        _append_artifact(artifact, True, result)
    return 0


if __name__ == "__main__":
    sys.exit(main())
