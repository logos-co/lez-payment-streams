"""Unit tests for verify/lib/harness_policy.py."""
from __future__ import annotations

import harness_policy as hp


def test_logoscore_accepts_pre_release_66c4194() -> None:
    text = (
        "logoscore version pre-release-66c4194\n"
        "commit: 66c4194ca6d3b556866cf22f5da912d235885dd8\n"
    )
    assert hp.logoscore_meets_min_revision(text) is True


def test_logoscore_rejects_v020_797b98a() -> None:
    text = "logoscore version 0.2.0\ncommit: 797b98a0123456789abcdef\n"
    assert hp.logoscore_meets_min_revision(text) is False


def test_logoscore_accepts_newer_pre_release() -> None:
    assert hp.logoscore_meets_min_revision("logoscore version pre-release-aabbcc1\n") is True


def test_logoscore_accepts_released_0_2_1() -> None:
    assert hp.logoscore_meets_min_revision("logoscore version 0.2.1\n") is True


def test_close_state_rejects_missing() -> None:
    assert hp.close_state_ok(-1) is False
    assert hp.close_state_ok(None) is False
    assert hp.close_state_ok("x") is False
    assert hp.close_state_ok(0) is False
    assert hp.close_state_ok(2) is True


def test_zero_accrual_claim_is_not_strict_success() -> None:
    assert hp.claim_zero_accrued_counts_as_success(False) is False
    assert hp.claim_zero_accrued_counts_as_success(True) is True


def test_parse_duration_seconds() -> None:
    assert hp.parse_duration_seconds("15s") == 15.0
    assert hp.parse_duration_seconds("45s") == 45.0
    assert hp.parse_duration_seconds("1m") == 60.0


def test_observed_cadence_ok() -> None:
    assert hp.observed_cadence_ok(45.0, 44.0) is True
    assert hp.observed_cadence_ok(15.0, 40.0) is False


def test_apply_sequencer_block_create_timeout() -> None:
    cfg = {"block_create_timeout": "15s", "port": 3040}
    same, changed = hp.apply_sequencer_block_create_timeout(cfg, "15s")
    assert changed is False
    assert same["block_create_timeout"] == "15s"
    updated, changed = hp.apply_sequencer_block_create_timeout(cfg, "45s")
    assert changed is True
    assert updated["block_create_timeout"] == "45s"
    assert cfg["block_create_timeout"] == "15s"


def test_cadence_seconds_from_block_log() -> None:
    text = "\n".join(
        [
            "[2026-08-15T11:26:00Z INFO  sequencer_service] Block with id 1 created",
            "[2026-08-15T11:26:45Z INFO  sequencer_service] Block with id 2 created",
            "[2026-08-15T11:27:30Z INFO  sequencer_service] Block with id 3 created",
        ]
    )
    assert hp.cadence_seconds_from_block_log(text) == 45.0
    assert hp.cadence_seconds_from_block_log("no blocks") is None
