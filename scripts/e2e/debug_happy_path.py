#!/usr/bin/env python3
"""Minimal dual-host probe: prepare proof on user, verify on provider (no Store)."""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "e2e"))
import run_local_e2e as e2e  # noqa: E402


def main() -> int:
    os.environ.setdefault("REPO", str(REPO))
    manifest_path = Path(os.environ.get("FIXTURE_MANIFEST", REPO / "fixtures" / "localnet.json"))
    manifest = json.loads(manifest_path.read_text())
    seq = manifest.get("sequencer_url", "http://127.0.0.1:3040")

    n8 = os.environ.get("N8_WIRE_HEX", "").strip()
    if not n8:
        p = subprocess.run(
            ["cargo", "run", "-q", "-p", "lez-payment-streams-core", "--bin", "n8_canonical_wire_hex"],
            cwd=REPO,
            capture_output=True,
            text=True,
        )
        n8 = p.stdout.strip()

    e2e_base = REPO / ".scaffold" / "e2e"
    cfg_u = Path(os.environ.get("LOGOSCORE_CONFIG_USER", e2e_base / "user" / "logoscore"))
    cfg_p = Path(os.environ.get("LOGOSCORE_CONFIG_PROVIDER", e2e_base / "provider" / "logoscore"))
    mod_u = Path(os.environ.get("MODULES_USER", e2e_base / "user" / "modules"))
    mod_p = Path(os.environ.get("MODULES_PROVIDER", e2e_base / "provider" / "modules"))
    pers_u = Path(os.environ.get("PERSIST_USER", e2e_base / "user" / "persist"))
    pers_p = Path(os.environ.get("PERSIST_PROVIDER", e2e_base / "provider" / "persist"))
    wc = Path(os.environ.get("WALLET_CONFIG", REPO / ".scaffold" / "wallet" / "wallet_config.json"))
    ws = Path(os.environ.get("WALLET_STORAGE", REPO / ".scaffold" / "wallet" / "storage.json"))

    os.environ["FIXTURE_MANIFEST"] = str(manifest_path)
    os.environ["PAYMENT_STREAMS_GUEST_BIN"] = str(
        os.environ.get(
            "PAYMENT_STREAMS_GUEST_BIN",
            REPO / "methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin",
        )
    )

    try:
        e2e.start_daemon(cfg_p, mod_p, pers_p)
        e2e.load_modules(cfg_p)
        e2e.open_wallet(cfg_p, wc, ws)
        e2e.sync_wallet(cfg_p, seq)

        prov_create = {
            "mode": "Core",
            **e2e.local_waku_json_base(),
            "portsShift": 100,
            "relay": True,
            "store": True,
            "storeMessageRetentionPolicy": "capacity:10000",
            "storeMessageDbUrl": f"sqlite://{(pers_p / 'store.sqlite3').as_posix()}",
        }
        e2e.delivery_create_start(cfg_p, prov_create, persist=pers_p, label="provider")
        e2e.set_eligibility_verifier(cfg_p, "payment_streams_module")
        provider_peer = e2e.get_node_info(cfg_p, "MyPeerId")

        e2e.start_daemon(cfg_u, mod_u, pers_u)
        e2e.load_modules(cfg_u)
        e2e.open_wallet(cfg_u, wc, ws)
        e2e.sync_wallet(cfg_u, seq)

        user_create = {
            "mode": "Core",
            **e2e.local_waku_json_base(),
            "portsShift": 0,
            "relay": True,
            "store": False,
            "staticnodes": [e2e.loopback_multiaddr(provider_peer, 60100)],
        }
        e2e.delivery_create_start(cfg_u, user_create, persist=pers_u, label="user")
        user_peer = e2e.get_node_info(cfg_u, "MyPeerId")

        proof = e2e.user_prepare_proof(cfg_u, manifest, n8, provider_peer)
        user_state = e2e.find_ps_state_file(pers_u)
        provider_state = e2e.find_ps_state_file(pers_p)
        seed = subprocess.run(
            [
                sys.executable,
                str(REPO / "scripts/e2e/seed_provider_acceptance.py"),
                "--user-state",
                str(user_state),
                "--provider-state",
                str(provider_state),
                "--manifest",
                str(manifest_path),
            ],
            capture_output=True,
            text=True,
        )
        print("seed:", seed.stdout or seed.stderr)
        e2e.logoscore_cmd(cfg_p, "unload-module", "payment_streams_module")
        e2e.logoscore_cmd(cfg_p, "load-module", "payment_streams_module")
        e2e.sync_wallet(cfg_u, seq)
        e2e.sync_wallet(cfg_p, seq)

        for label, peer in [("provider_peer", provider_peer), ("user_peer", user_peer)]:
            r = e2e.logoscore_cmd(
                cfg_p,
                "call",
                "payment_streams_module",
                "verifyEligibilityForStoreQuery",
                proof,
                n8,
                peer,
            )
            print(f"verify({label}={peer}):", r.stdout.strip()[-500:])

        query = dict(e2e.N8_REFERENCE_QUERY)
        query["eligibilityProofHex"] = proof
        qjson = json.dumps(query, separators=(",", ":"))
        provider_addr = e2e.loopback_multiaddr(provider_peer, 60100)
        watch_log = pers_u / "store-query-events.log"
        ps_watch = subprocess.Popen(
            ["logoscore", "--config-dir", str(cfg_p), "watch", "payment_streams_module"],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        time.sleep(0.5)
        resp = e2e.wait_store_query(cfg_u, qjson, provider_addr, watch_log)
        ps_watch.terminate()
        ps_out = ps_watch.stdout.read() if ps_watch.stdout else ""
        print("provider PS watch during storeQuery:", ps_out[-1500:])
        print("storeQuery:", resp)

        e2e.logoscore_cmd(cfg_p, "call", "delivery_module", "setEligibilityVerifier", "")
        watch_log2 = pers_u / "store-query-no-verify.log"
        resp2 = e2e.wait_store_query(cfg_u, qjson, provider_addr, watch_log2)
        print("storeQuery (verifier off):", resp2)
    finally:
        e2e.stop_daemon(cfg_u)
        e2e.stop_daemon(cfg_p)
    return 0


if __name__ == "__main__":
    sys.exit(main())
