#!/usr/bin/env python3
"""Public testnet sequencer JSON-RPC helpers (lez jsonrpsee API)."""
from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request
from typing import Any

DEFAULT_SEQUENCER = os.environ.get("TESTNET_SEQUENCER", "https://testnet.lez.logos.co/")

_B58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def base58_to_hex(value: str) -> str:
    num = 0
    for char in value:
        num = num * 58 + _B58_ALPHABET.index(char)
    raw = num.to_bytes((num.bit_length() + 7) // 8, "big") if num else b""
    # Preserve leading zero bytes (encoded as leading '1's in base58).
    pad = len(value) - len(value.lstrip("1"))
    raw = b"\x00" * pad + raw
    if len(raw) != 32:
        raise ValueError(f"expected 32-byte account id, got {len(raw)} bytes from base58")
    return raw.hex()


def rpc_call(
    method: str,
    params: dict | list | None = None,
    *,
    sequencer_url: str | None = None,
    timeout: float = 20.0,
) -> Any:
    url = sequencer_url or DEFAULT_SEQUENCER
    payload = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params if params is not None else {},
    }
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8")
    except urllib.error.URLError as err:
        raise RuntimeError(f"sequencer unreachable at {url}: {err}") from err

    try:
        parsed = json.loads(body)
    except json.JSONDecodeError as err:
        raise RuntimeError(f"non-JSON response from {url}: {body[:200]!r}") from err

    if parsed.get("error"):
        err = parsed["error"]
        msg = err.get("message") or err.get("data") or str(err)
        raise RuntimeError(f"RPC {method} failed: {msg}")

    return parsed.get("result")


def block_height(*, sequencer_url: str | None = None) -> int:
    """Return chain tip height via getLastBlockId (jsonrpsee)."""
    url = sequencer_url or DEFAULT_SEQUENCER
    result = rpc_call("getLastBlockId", [], sequencer_url=url)
    if isinstance(result, int):
        return result
    if isinstance(result, str) and result.isdigit():
        return int(result)
    raise RuntimeError(f"unexpected getLastBlockId shape: {result!r}")


def probe_sequencer(*, sequencer_url: str | None = None) -> bool:
    url = sequencer_url or DEFAULT_SEQUENCER
    try:
        block_height(sequencer_url=url)
        return True
    except RuntimeError:
        return False


def main(argv: list[str] | None = None) -> int:
    args = argv if argv is not None else sys.argv[1:]
    if not args or args[0] in ("-h", "--help"):
        print("usage: testnet_rpc.py block-height|hello", file=sys.stderr)
        return 2
    cmd = args[0]
    url = os.environ.get("TESTNET_SEQUENCER", DEFAULT_SEQUENCER)
    if cmd == "block-height":
        print(block_height(sequencer_url=url))
        return 0
    if cmd == "hello":
        print(json.dumps(rpc_call("hello", {}, sequencer_url=url)))
        return 0
    if cmd == "account-hex":
        if len(args) < 2:
            print("usage: testnet_rpc.py account-hex <base58>", file=sys.stderr)
            return 2
        print(base58_to_hex(args[1]))
        return 0
    print(f"unknown command: {cmd}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
