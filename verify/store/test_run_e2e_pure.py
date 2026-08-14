"""Unit tests for pure helpers in verify/store/run_e2e.py."""
from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

import run_e2e as rle  # noqa: E402


def test_vault_config_is_empty_from_probe() -> None:
    assert rle.vault_config_is_empty_from_probe(False) is True
    assert rle.vault_config_is_empty_from_probe(True) is False


def test_scan_first_empty_vault_id() -> None:
    seen: list[int] = []

    def is_empty(vid: int) -> bool:
        seen.append(vid)
        return vid >= 3

    assert rle.scan_first_empty_vault_id(is_empty) == 3
    assert seen == [0, 1, 2, 3]


def test_min_unaccrued_lo_for_proof() -> None:
    assert rle.min_unaccrued_lo_for_proof({"allocation": 80}) == 64
    assert rle.min_unaccrued_lo_for_proof({"allocation": 400}) == 100


def test_manifest_allocation_lo() -> None:
    assert rle.manifest_allocation_lo({"allocation": 400}) == 400
    assert rle.manifest_allocation_lo({"stream_allocation": 80}) == 80
    assert rle.manifest_allocation_lo({}) == 200


def test_strip_snapshot_stream_fields(tmp_path: Path) -> None:
    manifest_path = tmp_path / "m.json"
    manifest_path.write_text(
        '{"stream_id": 1, "stream_config_account_id": "x", "allocation": 400}\n'
    )
    manifest = {"stream_id": 1, "stream_config_account_id": "x", "allocation": 400}
    rle.strip_snapshot_stream_fields(manifest, manifest_path)
    assert "stream_id" not in manifest
    data = __import__("json").loads(manifest_path.read_text())
    assert "stream_id" not in data


def test_seed_deposit_amount_lo() -> None:
    assert rle.seed_deposit_amount_lo({"allocation": 400}) == 500
    assert rle.seed_deposit_amount_lo({"demo_deposit_amount": 600}) == 600


def test_testnet_write_manifest_program_id() -> None:
    pid = "c30781ea9d7cc7b3be36f459ce9094644b984224d3d3119a644bb1b21ba2982a"
    assert rle.testnet_write_manifest_program_id(pid, pid) == pid
    assert rle.testnet_write_manifest_program_id(pid.upper(), pid) == pid
    assert rle.testnet_write_manifest_program_id(pid, None) == pid
    try:
        rle.testnet_write_manifest_program_id(None, pid)
        raise AssertionError("expected E2EError when env hex is missing")
    except rle.E2EError:
        pass
    try:
        rle.testnet_write_manifest_program_id("aa" * 32, pid)
        raise AssertionError("expected E2EError on mismatch")
    except rle.E2EError:
        pass


def test_vault_deposit_preflight() -> None:
    # Existing vault already covers createStream; do not top up to deposit_lo.
    assert (
        rle.vault_deposit_preflight(420, 500, 400, 300, False) == "skip"
    )
    assert (
        rle.vault_deposit_preflight(500, 500, 400, 0, False) == "skip"
    )
    # Fresh or underfunded vault, public owner cannot cover deposit_lo.
    assert (
        rle.vault_deposit_preflight(0, 500, 400, 300, False) == "abort"
    )
    assert (
        rle.vault_deposit_preflight(None, 500, 400, 100, False) == "abort"
    )
    # Public owner can cover deposit_lo.
    assert (
        rle.vault_deposit_preflight(0, 500, 400, 600, False) == "deposit"
    )
    assert (
        rle.vault_deposit_preflight(100, 500, 400, 600, False) == "deposit"
    )
    # Private owner: do not gate on public getAccount balance.
    assert (
        rle.vault_deposit_preflight(0, 500, 400, 0, True) == "deposit"
    )


def test_testnet_e2e_create_via_default(monkeypatch) -> None:
    monkeypatch.delenv("E2E_CREATE_VIA", raising=False)
    assert rle.testnet_e2e_create_via() == "chainaction"
    monkeypatch.setenv("E2E_CREATE_VIA", "seed")
    assert rle.testnet_e2e_create_via() == "seed"


def test_reset_cloned_wallet_sync_cursor(tmp_path: Path) -> None:
    import json

    storage = tmp_path / "storage.json"
    storage.write_text(json.dumps({"last_synced_block": 7396, "key_chain": {}}) + "\n")
    rle._reset_cloned_wallet_sync_cursor(storage)
    assert json.loads(storage.read_text())["last_synced_block"] == 0

    rle._reset_cloned_wallet_sync_cursor(storage)
    assert json.loads(storage.read_text())["last_synced_block"] == 0

    missing = tmp_path / "absent.json"
    rle._reset_cloned_wallet_sync_cursor(missing)
    assert not missing.exists()
