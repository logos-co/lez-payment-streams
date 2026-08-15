"""Pure harness policy helpers for real-prove E2E (timeout, liveness, artifacts)."""
from __future__ import annotations

import json
import re
from datetime import datetime
from typing import Any

LOGOSCORE_MIN_LABEL = "pre-release-66c4194"
LOGOSCORE_KNOWN_BAD_COMMITS = ("797b98a",)
LOGOSCORE_MIN_MARKERS = (
    "pre-release-66c4194",
    "66c4194ca6d3",
    "commit: 66c4194",
)
CLOSED_STREAM_STATE = 2
MISSING_STREAM_STATE = -1


def logoscore_meets_min_revision(text: str) -> bool:
    """True when logoscore is pre-release-66c4194 or newer.

    v0.2.0 commit 797b98a ignores LOGOSCORE_RPC_TIMEOUT_MS and is rejected.
    """
    blob = text or ""
    if any(bad in blob for bad in LOGOSCORE_KNOWN_BAD_COMMITS):
        return False
    if any(marker in blob for marker in LOGOSCORE_MIN_MARKERS):
        return True
    if "pre-release-" in blob:
        return True
    match = re.search(r"version\s+(\d+)\.(\d+)\.(\d+)", blob)
    if match:
        ver = tuple(int(part) for part in match.groups())
        return ver >= (0, 2, 1)
    return False


def close_state_ok(stream_state: int | None) -> bool:
    """close_state succeeds only when the stream is Closed (2)."""
    try:
        return int(stream_state) == CLOSED_STREAM_STATE
    except (TypeError, ValueError):
        return False


def claim_zero_accrued_counts_as_success(claim_optional: bool) -> bool:
    """Skipped zero-accrual claims succeed only when strict claim is off."""
    return bool(claim_optional)


def parse_duration_seconds(value: str) -> float:
    """Parse LEZ duration strings such as 15s, 45s, 1m."""
    raw = (value or "").strip()
    match = re.fullmatch(r"(\d+(?:\.\d+)?)(ms|s|m|h)?", raw)
    if not match:
        raise ValueError(f"invalid duration: {value!r}")
    amount = float(match.group(1))
    unit = match.group(2) or "s"
    scale = {"ms": 0.001, "s": 1.0, "m": 60.0, "h": 3600.0}[unit]
    return amount * scale


def clock50_prove_window_ready(
    clock_block: int | None,
    tip: int | None,
    *,
    rem_max: int = 2,
    stale_max: int = 2,
) -> bool:
    """True when CLOCK_50 is early in its epoch and the wallet clock matches tip."""
    try:
        clock = int(clock_block)
        head = int(tip)
    except (TypeError, ValueError):
        return False
    if clock < 0 or head < 0:
        return False
    if clock % 50 > rem_max:
        return False
    if head - clock > stale_max:
        return False
    return True


def clock50_attempts_for_cadence(kind: str, block_s: float) -> int:
    """Poll budget for one CLOCK_50 epoch plus margin at the given block time."""
    epoch_s = 50.0 * max(float(block_s), 1.0)
    if kind == "advance":
        return max(120, int(epoch_s / 5.0) + 24)
    if kind == "window":
        return max(90, int(epoch_s / 2.0) + 30)
    raise ValueError(f"unknown clock50 attempt kind: {kind}")


def observed_cadence_ok(
    requested_s: float, observed_s: float, *, tolerance: float = 0.35
) -> bool:
    """True when observed block cadence is within tolerance of the request."""
    if requested_s <= 0 or observed_s <= 0:
        return False
    delta = abs(observed_s - requested_s) / requested_s
    return delta <= tolerance


DEFAULT_LOCALNET_BLOCK_TIME = "15s"
_BLOCK_CREATED_RE = re.compile(
    r"\[(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})Z[^\]]*\] Block with id (\d+) created"
)


def sequencer_block_create_timeout(config: dict[str, Any]) -> str:
    return str(config.get("block_create_timeout") or "").strip()


def apply_sequencer_block_create_timeout(
    config: dict[str, Any], duration: str
) -> tuple[dict[str, Any], bool]:
    """Return (config, changed) after setting block_create_timeout."""
    wanted = (duration or "").strip()
    if not wanted:
        raise ValueError("empty block_create_timeout")
    parse_duration_seconds(wanted)
    current = sequencer_block_create_timeout(config)
    if current == wanted:
        return config, False
    out = dict(config)
    out["block_create_timeout"] = wanted
    return out, True


def cadence_seconds_from_block_log(text: str, *, min_samples: int = 3) -> float | None:
    """Mean inter-block seconds from sequencer 'Block with id N created' lines."""
    stamps: list[datetime] = []
    for line in (text or "").splitlines():
        match = _BLOCK_CREATED_RE.search(line)
        if not match:
            continue
        stamps.append(datetime.fromisoformat(match.group(1)))
    if len(stamps) < min_samples:
        return None
    recent = stamps[-(min_samples):]
    gaps = [
        (recent[i] - recent[i - 1]).total_seconds()
        for i in range(1, len(recent))
        if (recent[i] - recent[i - 1]).total_seconds() > 0
    ]
    if not gaps:
        return None
    return sum(gaps) / len(gaps)


def write_json(path: str, payload: dict[str, Any]) -> None:
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2)
        handle.write("\n")
