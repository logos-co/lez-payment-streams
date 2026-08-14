#!/usr/bin/env bash
# Fail on broken relative markdown links in living tracked .md files.
# Skips docs/plan/, docs/archive/, and docs/external/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

python3 - <<'PY'
from __future__ import annotations

import re
import subprocess
from pathlib import Path

ROOT = Path(".").resolve()
SKIP_PREFIXES = (
    "docs/plan/",
    "docs/archive/",
    "docs/external/",
)
LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")


def listed_md() -> list[Path]:
    out = subprocess.check_output(["git", "ls-files", "*.md"], text=True)
    files = []
    for line in out.splitlines():
        if not line:
            continue
        if any(line.startswith(p) for p in SKIP_PREFIXES):
            continue
        files.append(Path(line))
    return files


def should_check(target: str) -> bool:
    t = target.strip()
    if not t or t.startswith("#"):
        return False
    if re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*:", t):
        return False
    return True


failures = 0
for md in listed_md():
    text = md.read_text(encoding="utf-8")
    for match in LINK_RE.finditer(text):
        raw = match.group(1).strip()
        href = raw.split()[0].strip("<>")
        if not should_check(href):
            continue
        path_part = href.split("#", 1)[0]
        if not path_part:
            continue
        dest = (md.parent / path_part).resolve()
        try:
            dest.relative_to(ROOT)
        except ValueError:
            # Link escaped the repo (e.g. ../outside). Still require it exists.
            pass
        if not dest.exists():
            print(f"BROKEN {md}: {href}")
            failures += 1

if failures:
    print(f"=== {failures} broken relative link(s) ===")
    raise SystemExit(1)
print("=== relative links ok ===")
PY
