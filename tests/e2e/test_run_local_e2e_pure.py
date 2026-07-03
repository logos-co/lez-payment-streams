"""Unit tests for pure helpers in scripts/e2e/run_local_e2e.py."""
from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "e2e"))

import run_local_e2e as rle  # noqa: E402


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


def test_testnet_e2e_create_via_default(monkeypatch) -> None:
    monkeypatch.delenv("E2E_CREATE_VIA", raising=False)
    assert rle.testnet_e2e_create_via() == "chainaction"
    monkeypatch.setenv("E2E_CREATE_VIA", "seed")
    assert rle.testnet_e2e_create_via() == "seed"
