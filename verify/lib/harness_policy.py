"""Pure harness policy helpers for real-prove E2E (timeout, liveness, artifacts)."""
from __future__ import annotations

import re

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


def observed_cadence_ok(
    requested_s: float, observed_s: float, *, tolerance: float = 0.35
) -> bool:
    """True when observed block cadence is within tolerance of the request."""
    if requested_s <= 0 or observed_s <= 0:
        return False
    delta = abs(observed_s - requested_s) / requested_s
    return delta <= tolerance
