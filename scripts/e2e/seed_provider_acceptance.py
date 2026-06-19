#!/usr/bin/env python3
"""Copy session_public_key_hex from user PS state into provider provider_acceptances (Step 17 warm-up)."""
from __future__ import annotations

import argparse
import base64
import json
import sys
from pathlib import Path


def b58decode_account_to_hex(b58: str) -> str:
    alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    num = 0
    for ch in b58:
        num = num * 58 + alphabet.index(ch)
    combined = num.to_bytes(32, "big") if num.bit_length() <= 256 else num.to_bytes(33, "big")
    if len(combined) >= 32:
        combined = combined[-32:]
    else:
        combined = b"\x00" * (32 - len(combined)) + combined
    return combined.hex()


def load_state(path: Path) -> dict:
    if not path.is_file():
        return {"schema_version": 2, "provider_acceptances": []}
    return json.loads(path.read_text())


def save_state(path: Path, state: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(state, indent=2) + "\n")


def session_from_user_negotiations(user_state: dict, vault_id: int, provider_id_hex: str) -> tuple[str | None, str | None]:
    # Iterate newest-first: prepareEligibility appends a fresh negotiation per run, so a
    # stale row from a prior session (different provider key) must not win when persist
    # was not cleared. When provider_id_hex is supplied we only accept the matching row.
    for row in reversed(user_state.get("negotiations", [])):
        if int(row.get("vault_id", -1)) != vault_id:
            continue
        row_provider = row.get("provider_id_hex", "").lower()
        if provider_id_hex and row_provider != provider_id_hex.lower():
            continue
        pub = row.get("session_public_key_hex", "")
        if pub:
            pid = row_provider or provider_id_hex
            return pub.lower(), pid
    return None, None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--user-state", type=Path, required=True)
    ap.add_argument("--provider-state", type=Path, required=True)
    ap.add_argument("--manifest", type=Path, required=True)
    ap.add_argument("--vault-id", type=int, default=-1)
    ap.add_argument("--stream-id", type=int, default=-1)
    args = ap.parse_args()

    manifest = json.loads(args.manifest.read_text())
    provider_b58 = manifest["provider_account_id"]
    vault_id = int(manifest["vault_id"]) if args.vault_id < 0 else args.vault_id
    stream_id = int(manifest["stream_id"]) if args.stream_id < 0 else args.stream_id

    user_state = load_state(args.user_state)
    # Match the negotiation for the current provider (manifest) so a stale row from a
    # previous session under a different provider key cannot leak in.
    manifest_provider_hex = b58decode_account_to_hex(provider_b58)
    session_hex, provider_id_hex = session_from_user_negotiations(user_state, vault_id, manifest_provider_hex)
    if not session_hex:
        # Fall back to newest-any-provider only if no provider-specific match exists.
        session_hex, provider_id_hex = session_from_user_negotiations(user_state, vault_id, "")
    if not session_hex:
        print("ERROR: no session_public_key_hex in user negotiations", file=sys.stderr)
        return 1
    if not provider_id_hex:
        provider_id_hex = manifest_provider_hex

    provider_state = load_state(args.provider_state)
    if provider_state.get("schema_version", 1) < 2:
        provider_state["schema_version"] = 2
    provider_state.setdefault("peer_mappings", {})
    provider_state.setdefault("negotiations", [])
    provider_state.setdefault("inventory", [])
    acc = provider_state.setdefault("provider_acceptances", [])

    row = {
        "vault_id": vault_id,
        "provider_id_hex": provider_id_hex,
        "stream_id": stream_id,
        "session_public_key_hex": session_hex,
        "policy_at_acceptance": {
            "min_rate": "1",
            "min_allocation": "1",
            "max_create_stream_deadline_delay": "3600",
        },
    }
    replaced = False
    for i, existing in enumerate(acc):
        if (
            int(existing.get("vault_id", -1)) == vault_id
            and existing.get("provider_id_hex", "").lower() == provider_id_hex
        ):
            acc[i] = {**existing, **row}
            replaced = True
            break
    if not replaced:
        acc.append(row)

    save_state(args.provider_state, provider_state)
    print(json.dumps({"ok": True, "provider_id_hex": provider_id_hex, "session_public_key_hex": session_hex}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
