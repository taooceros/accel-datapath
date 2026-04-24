# papers AGENTS

Inherits `../../../AGENTS.md` and `../../AGENTS.md`.

## OVERVIEW
This folder holds per-paper literature artifacts. Each paper directory should keep the original `paper.pdf` plus KB-searchable extracted text that is good enough for repo search, quoting, and downstream synthesis.

## REQUIRED ARTIFACTS PER PAPER FOLDER
- `paper.pdf` — the original local PDF
- `README.md` — curated paper summary, confidence, and repo relevance
- `paper.md` — raw but searchable extracted text in Markdown
- `paper.txt` — optional compatibility artifact when already present or when a plain-text fallback is useful

## EXTRACTION PIPELINE
Use this pipeline when adding or refreshing paper text.

1. **Preserve the source PDF**
   - Never treat the PDF itself as the searchable knowledge artifact.
   - Keep `paper.pdf` unchanged and generate derived text next to it.

2. **Default target: `paper.md`**
   - Prefer Markdown over plain text so the extracted content is easier to search, inspect, and cite in the repo knowledge flow.
   - Start the file with a short provenance note that names the extraction method and warns that minor PDF artifacts may remain.
   - When section detection is reliable, prefer a section-based `paper.md` over a page-block dump.

3. **Extraction method order**
   - **Reusable workflow entrypoint:** `tools/paper-text/extract_paper_text.py`.
   - **First choice for simple one-column PDFs:** `pdftotext` without `-layout`.
   - **First choice for multi-column papers:** column-aware extraction, usually page-by-page left column then right column, then write the result into Markdown sections such as `## Page N` before sectionization.
   - **Avoid defaulting to `pdftotext -layout` for two-column academic papers.** It often preserves visual placement instead of reading order and can splice unrelated column fragments into broken sentences.
   - If auto column detection looks wrong, rerun the extractor with an explicit override such as `--columns two`.
   - **Fallbacks:** `pdfplumber` / `pypdf` extraction, or OCR only when the PDF is image-based or text extraction is clearly failing.

4. **Sectionization stage**
   - After extraction, use `tools/paper-text/sectionize_markdown.py` to convert page-based markdown into section-based markdown when headings are recoverable from the extracted text.
   - The sectionizer should be conservative: promote only text-supported headings such as `Abstract`, numbered sections/subsections, `Related Work`, `Conclusions`, and `References`.
   - Keep figure-caption cleanup out of this pass unless the task explicitly requests it.

5. **Normalization expectations**
   - Repair line wrapping into readable paragraphs.
   - Join hyphenated line breaks when they are clearly word continuations.
   - Remove obvious page-number and publication boilerplate when it pollutes the text.
   - Keep figure captions and section headings when they remain readable; this is a raw knowledge artifact, not a polished edition.

6. **Verification checklist**
   - Run `tools/paper-text/verify_paper_text.py` on the final `paper.md`.
   - Spot-check the abstract and first section for sentence order.
   - For two-column papers, confirm the text does **not** interleave left- and right-column content.
   - Check at least one later page with figures or section transitions.
   - If the result is still degraded, document the limitation explicitly in the paper folder or `README.md` rather than silently keeping a bad extraction.

7. **Paper-folder updates**
   - Link `paper.md` from the paper folder `README.md` under local artifacts.
   - Record meaningful extraction-method changes in the active literature canonical thread file under `.agents/state/threads/`, then refresh the matching `current.md` dashboard/index entry.

## RECOMMENDED SHAPE FOR `paper.md`
```md
# <Paper title>

> Extracted from `paper.pdf` with <method>. Raw KB-searchable text; minor PDF artifacts may remain.

## Page 1

...
```

Use page-based sections as the safe intermediate extraction form, then prefer section-based `paper.md` when the sectionizer can recover headings cleanly.

## ANTI-PATTERNS
- Do not rely on `paper.pdf` alone as the knowledge artifact.
- Do not overwrite curated interpretation in `README.md` with raw extraction dumps.
- Do not assume `pdftotext -layout` is safer just because it looks closer to the PDF page.
- Do not force auto column detection when a manual `--columns two` rerun clearly fixes the text order.
- Do not hide known extraction failures; document them.
