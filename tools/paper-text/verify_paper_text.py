#!/usr/bin/env python3

from __future__ import annotations

import argparse
import re
from pathlib import Path


HEADING_RE = re.compile(r"^(#{2,4})\s+(.+)$", re.MULTILINE)
NUMBERED_HEADING_RE = re.compile(
    r"^(?:##|###|####)\s+([1-9](?:\.[0-9]+)*)\s+", re.MULTILINE
)


def numbered_heading_tuple(text: str) -> tuple[int, ...]:
    return tuple(int(part) for part in text.split("."))


def verify(text: str) -> list[str]:
    issues: list[str] = []
    if "## Page " in text:
        issues.append("page headings still present")
    for required in ("## Abstract", "## References"):
        if required not in text:
            issues.append(f"missing required heading: {required}")

    numbered = [
        numbered_heading_tuple(match.group(1))
        for match in NUMBERED_HEADING_RE.finditer(text)
    ]
    if not numbered:
        issues.append("no numbered headings found")
    else:
        previous = numbered[0]
        for current in numbered[1:]:
            if current < previous and current[0] >= previous[0]:
                issues.append(
                    f"non-monotonic heading order around {previous} -> {current}"
                )
                break
            previous = current

    headings = [match.group(2) for match in HEADING_RE.finditer(text)]
    if "1 Introduction" not in headings:
        issues.append("missing 1 Introduction heading")
    if "6 Related Work" not in headings:
        issues.append("missing 6 Related Work heading")
    if "7 Conclusions" not in headings:
        issues.append("missing 7 Conclusions heading")

    return issues


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_md", type=Path)
    args = parser.parse_args()

    text = args.input_md.read_text(encoding="utf-8")
    issues = verify(text)
    if issues:
        for issue in issues:
            print(f"ERROR: {issue}")
        return 1
    print("verification passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
