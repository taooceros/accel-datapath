# paper-text tools

Reusable helpers for turning local `paper.pdf` artifacts into KB-searchable Markdown.

## Scripts

- `extract_paper_text.py` — extract page-aware Markdown from a PDF, with one-column or two-column handling
- `sectionize_markdown.py` — convert page-based paper Markdown into more Markdown-native section headings
- `verify_paper_text.py` — run lightweight structural checks on extracted or sectionized paper Markdown

## Typical flow

```bash
python tools/paper-text/extract_paper_text.py \
  docs/report/literature/papers/<paper>/paper.pdf \
  docs/report/literature/papers/<paper>/paper.md

python tools/paper-text/sectionize_markdown.py \
  docs/report/literature/papers/<paper>/paper.md \
  --in-place

python tools/paper-text/verify_paper_text.py \
  docs/report/literature/papers/<paper>/paper.md
```

## Notes

- The extractor keeps page-level traceability because that is the safest intermediate representation.
- The sectionizer is conservative and only promotes headings that are text-supported by the extraction.
- Figure-caption cleanup is intentionally out of scope for this workflow.
