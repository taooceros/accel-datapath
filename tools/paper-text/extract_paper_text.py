#!/usr/bin/env python3

from __future__ import annotations

import argparse
import re
import subprocess
import tempfile
from pathlib import Path

BOILERPLATE_PATTERNS = [
    r"^This work is licensed under",
    r"^4\.0 License\.?$",
    r"^SOSP.? ?’23,",
    r"^© ?2023 Copyright held by the owner/author\(s\)\.?$",
    r"^ACM ISBN ",
    r"^https://doi\.org/",
    r"^\d+$",
]


def extract_crop(
    pdf_path: Path, page_num: int, x: int, y: int, width: int, height: int
) -> str:
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_output = Path(temp_dir) / "page.txt"
        subprocess.run(
            [
                "pdftotext",
                "-f",
                str(page_num),
                "-l",
                str(page_num),
                "-x",
                str(x),
                "-y",
                str(y),
                "-W",
                str(width),
                "-H",
                str(height),
                str(pdf_path),
                str(temp_output),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return temp_output.read_text(encoding="utf-8", errors="replace")


def detect_columns(pdf_path: Path) -> str:
    sample_text = extract_crop(pdf_path, 1, 0, 0, 5000, 5000)
    if (
        "Abstract" in sample_text
        and "Introduction" in sample_text
        and "in prior research." in sample_text
    ):
        return "two"
    return "one"


def pdfinfo_output(pdf_path: Path) -> str:
    proc = subprocess.run(
        ["pdfinfo", str(pdf_path)],
        check=True,
        capture_output=True,
        text=True,
    )
    return proc.stdout


def page_dimensions(pdf_path: Path, page_num: int) -> tuple[int, int]:
    info = pdfinfo_output(pdf_path)
    match = re.search(r"Page size:\s+([0-9.]+) x ([0-9.]+) pts", info)
    if not match:
        raise RuntimeError(f"could not determine page size for {pdf_path}")
    width = int(float(match.group(1)))
    height = int(float(match.group(2)))
    return width, height


def total_pages(pdf_path: Path) -> int:
    info = pdfinfo_output(pdf_path)
    match = re.search(r"Pages:\s+(\d+)", info)
    if not match:
        raise RuntimeError(f"could not determine page count for {pdf_path}")
    return int(match.group(1))


def normalize_block(text: str) -> str:
    lines = text.replace("\r\n", "\n").replace("\r", "\n").split("\n")
    cleaned: list[str] = []
    for raw_line in lines:
        line = raw_line.strip()
        if not line:
            cleaned.append("")
            continue
        if any(re.match(pattern, line) for pattern in BOILERPLATE_PATTERNS):
            continue
        cleaned.append(line)

    paragraphs: list[str] = []
    buffer = ""
    for line in cleaned:
        if not line:
            if buffer:
                paragraphs.append(buffer)
                buffer = ""
            continue
        if not buffer:
            buffer = line
            continue
        if buffer.endswith("-") and line[:1].islower():
            buffer = buffer[:-1] + line
        else:
            buffer += " " + line
    if buffer:
        paragraphs.append(buffer)

    text = "\n\n".join(paragraphs)
    text = re.sub(r"\s+", " ", text).strip()
    for before, after in ((" .", "."), (" ,", ","), (" ;", ";"), (" :", ":")):
        text = text.replace(before, after)
    return text


def extract_page_text(pdf_path: Path, page_num: int, mode: str) -> str:
    width, height = page_dimensions(pdf_path, page_num)

    if mode == "two":
        midpoint = width // 2
        overlap = 10
        left = extract_crop(pdf_path, page_num, 0, 0, midpoint + overlap, height)
        right = extract_crop(
            pdf_path,
            page_num,
            max(midpoint - overlap, 0),
            0,
            width - (midpoint - overlap),
            height,
        )
        if page_num == 1:
            abstract_index = left.find("Abstract")
            if abstract_index != -1:
                left = left[abstract_index:]
            for marker in ("in prior research.", "CCS Concepts:", "Introduction"):
                marker_index = right.find(marker)
                if marker_index != -1:
                    right = right[marker_index:]
                    break
        chunks = [normalize_block(chunk) for chunk in (left, right)]
        return "\n\n".join(chunk for chunk in chunks if chunk)

    page_text = extract_crop(pdf_path, page_num, 0, 0, width, height)
    return normalize_block(page_text)


def build_markdown(pdf_path: Path, title: str | None, mode: str) -> str:
    page_count = total_pages(pdf_path)
    heading = title or pdf_path.stem
    lines = [
        f"# {heading}",
        "",
        f"> Extracted from `paper.pdf` with {mode}-column-aware `pdftotext` processing. Raw KB-searchable text; minor PDF artifacts may remain.",
        "",
    ]
    for page_num in range(1, page_count + 1):
        lines.append(f"## Page {page_num}")
        lines.append("")
        lines.append(extract_page_text(pdf_path, page_num, mode))
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_pdf", type=Path)
    parser.add_argument("output_md", type=Path)
    parser.add_argument("--title")
    parser.add_argument("--columns", choices=["auto", "one", "two"], default="auto")
    args = parser.parse_args()

    mode = detect_columns(args.input_pdf) if args.columns == "auto" else args.columns
    markdown = build_markdown(args.input_pdf, args.title, mode)
    args.output_md.write_text(markdown, encoding="utf-8")
    print(f"wrote {args.output_md}")
    print(f"columns={mode}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
