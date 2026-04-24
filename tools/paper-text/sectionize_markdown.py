#!/usr/bin/env python3

from __future__ import annotations

import argparse
import re
from pathlib import Path


PAGE_RE = re.compile(r"^## Page \d+$", re.MULTILINE)


REPLACEMENTS: list[tuple[re.Pattern[str], str, int]] = [
    (re.compile(r"\bAbstract\s+"), "## Abstract\n\n", 1),
    (re.compile(r"\bCCS Concepts:\s*"), "## CCS Concepts\n\n", 1),
    (re.compile(r"\bKeywords:\s*"), "## Keywords\n\n", 1),
    (re.compile(r"\bACM Reference Format:\s*"), "## ACM Reference Format\n\n", 1),
    (
        re.compile(r"\bIntroduction\s+Modern cloud computing"),
        "## 1 Introduction\n\nModern cloud computing",
        1,
    ),
    (
        re.compile(r"\b2 Characteristics of RPCs at Hyperscale\s*"),
        "## 2 Characteristics of RPCs at Hyperscale\n\n",
        1,
    ),
    (
        re.compile(r"Characteristics of RPCs at Hyperscale\s+This section analyzes"),
        "## 2 Characteristics of RPCs at Hyperscale\n\nThis section analyzes",
        1,
    ),
    (re.compile(r"\b2\.1 Methodology\s*"), "### 2.1 Methodology\n\n", 1),
    (
        re.compile(r"\b2\.2 Why is RPC Evaluation Important\?\s*"),
        "### 2.2 Why is RPC Evaluation Important?\n\n",
        1,
    ),
    (
        re.compile(r"\b2\.3 Not all RPCs are the same\.\s*"),
        "### 2.3 Not all RPCs are the same.\n\n",
        1,
    ),
    (
        re.compile(r"\b2\.4 Nested RPCs are Wider than Deep\s*"),
        "### 2.4 Nested RPCs are Wider than Deep\n\n",
        1,
    ),
    (re.compile(r"\b2\.5 RPC Size Matters\s*"), "### 2.5 RPC Size Matters\n\n", 1),
    (
        re.compile(r"\b2\.6 Storage RPCs are Important\s*"),
        "### 2.6 Storage RPCs are Important\n\n",
        1,
    ),
    (
        re.compile(r"\bRPC Latency\s+The previous section showed"),
        "## 3 RPC Latency\n\nThe previous section showed",
        1,
    ),
    (re.compile(r"\b3\.1 RPC Components\s*"), "### 3.1 RPC Components\n\n", 1),
    (
        re.compile(r"\b3\.2 Fleet-Wide Latency Variation\s*"),
        "### 3.2 Fleet-Wide Latency Variation\n\n",
        1,
    ),
    (
        re.compile(r"\b3\.3 Service-Specific Latency Variation\s*"),
        "### 3.3 Service-Specific Latency Variation\n\n",
        1,
    ),
    (
        re.compile(r"\b3\.3\.1 Latency Variation Within a Cluster\.\s*"),
        "#### 3.3.1 Latency Variation Within a Cluster\n\n",
        1,
    ),
    (
        re.compile(r"\b3\.3\.2 Component Impact on Tail Latency\.\s*"),
        "#### 3.3.2 Component Impact on Tail Latency\n\n",
        1,
    ),
    (
        re.compile(r"\b3\.3\.3 Service Latency of Different Clusters\.\s*"),
        "#### 3.3.3 Service Latency of Different Clusters\n\n",
        1,
    ),
    (
        re.compile(r"\b3\.3\.4 Exogenous Variables Affecting Latency Variation\.\s*"),
        "#### 3.3.4 Exogenous Variables Affecting Latency Variation\n\n",
        1,
    ),
    (
        re.compile(r"\b3\.3\.5 Latency of Cross-Cluster RPCs\.\s*"),
        "#### 3.3.5 Latency of Cross-Cluster RPCs\n\n",
        1,
    ),
    (
        re.compile(r"(?:\ba\s+)?Resource Utilization of RPCs\s+This section studies"),
        "## 4 Resource Utilization of RPCs\n\nThis section studies",
        1,
    ),
    (
        re.compile(r"\b4\.1 CPU Cycle Breakdown\s*"),
        "### 4.1 CPU Cycle Breakdown\n\n",
        1,
    ),
    (
        re.compile(r"\b4\.2 Fleet-Wide CPU Cycle Variation\s*"),
        "### 4.2 Fleet-Wide CPU Cycle Variation\n\n",
        1,
    ),
    (
        re.compile(r"\b4\.3 Load-Balancing Resources\s*"),
        "### 4.3 Load-Balancing Resources\n\n",
        1,
    ),
    (
        re.compile(r"\b4\.4 RPC Cancellations and Errors\s*"),
        "### 4.4 RPC Cancellations and Errors\n\n",
        1,
    ),
    (
        re.compile(r"\bImplications\s+This section briefly highlights"),
        "## 5 Implications\n\nThis section briefly highlights",
        1,
    ),
    (
        re.compile(r"\b5\.1 RPC Behavior and Problems\s*"),
        "### 5.1 RPC Behavior and Problems\n\n",
        1,
    ),
    (
        re.compile(r"\b5\.2 Software Optimizations\s*"),
        "### 5.2 Software Optimizations\n\n",
        1,
    ),
    (
        re.compile(r"\b5\.3 Hardware Optimizations\s*"),
        "### 5.3 Hardware Optimizations\n\n",
        1,
    ),
    (re.compile(r"\b5\.4 Limitations\s*"), "### 5.4 Limitations\n\n", 1),
    (re.compile(r"\bRelated Work\s+"), "## 6 Related Work\n\n", 1),
    (
        re.compile(r"\bConclusions\s+This paper presents"),
        "## 7 Conclusions\n\nThis paper presents",
        1,
    ),
    (re.compile(r"\bReferences\s+\[1\]\s*"), "## References\n\n[1] ", 1),
]


def sectionize_markdown(text: str) -> str:
    lines = text.splitlines()
    header: list[str] = []
    body: list[str] = []
    in_body = False
    for line in lines:
        if PAGE_RE.match(line):
            in_body = True
            continue
        if not in_body:
            header.append(line)
            continue
        body.append(line)

    body_text = "\n".join(body)
    body_text = re.sub(r"\n{3,}", "\n\n", body_text).strip()

    for pattern, replacement, count in REPLACEMENTS:
        body_text = pattern.sub(replacement, body_text, count=count)

    body_text = re.sub(r"\s+(#{2,4}\s)", r"\n\n\1", body_text)
    body_text = re.sub(r"(?m)^## Page \d+\n?", "", body_text)
    body_text = re.sub(r"\n{3,}", "\n\n", body_text).strip()

    header_text = "\n".join(header).rstrip()
    return header_text + "\n\n" + body_text + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_md", type=Path)
    parser.add_argument("output_md", nargs="?", type=Path)
    parser.add_argument("--in-place", action="store_true")
    args = parser.parse_args()

    if args.in_place:
        output_path = args.input_md
    elif args.output_md is not None:
        output_path = args.output_md
    else:
        raise SystemExit("provide OUTPUT_MD or use --in-place")

    text = args.input_md.read_text(encoding="utf-8")
    output_path.write_text(sectionize_markdown(text), encoding="utf-8")
    print(f"wrote {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
