---
name: google-search
description: Web discovery workflow for targeted Google Search queries using Lightpanda for search, navigation, and URL extraction. Use when you need low-volume web search, source discovery, or PDF/source URL discovery, then hand off the final URL to a separate downloader.
---

# Google Search Workflow

Use this skill for targeted web discovery when the task needs Google Search specifically. Treat this as a low-volume discovery workflow, not a scraping workflow. This is browser-assisted discovery, not an official Google Search API workflow.

## When to Use

- Searching for an exact paper title, project page, PDF URL, DOI landing page, dataset page, or official documentation.
- Refining a web query with `site:`, exact phrases, exclusions, or filetype targeting.
- Discovering candidate PDF URLs or source landing pages before acquisition.

## When Not to Use

- Bulk result collection.
- High-concurrency scraping.
- Any workflow that depends on bypassing consent screens, CAPTCHAs, or unusual-traffic protections.
- Any workflow that assumes Lightpanda directly downloads and saves PDFs.

## Grounded Capability Boundary

Lightpanda is for **search, navigation, page interaction, and URL extraction**.

- Supported patterns from Lightpanda docs and examples:
  - `fetch` for page retrieval and DOM dump when interaction is not needed
  - CDP server mode for Puppeteer/Playwright-style automation
  - MCP or browser interaction primitives for `goto`, `fill`, `click`, waiting, and link extraction
- Do **not** describe Lightpanda as the downloader. Current grounded guidance is: use Lightpanda to reveal the final URL, then use a **separate downloader** to fetch the asset and a **separate PDF validation** step to confirm the response is really a PDF.

## Query Construction

Prefer a few high-quality queries over deep paging.

- Exact title: `"paper title"`
- Domain restriction: `site:arxiv.org`, `site:usenix.org`, `site:dl.acm.org`, `site:ieeexplore.ieee.org`
- Exclusions: `-slides -poster -syllabus`
- PDF targeting when appropriate: `filetype:pdf`
- Combine exact phrase plus domain first, then relax constraints if needed.

## Workflow

1. **Start with the best narrow query**.
   - Prefer exact title or exact phrase plus `site:` restriction.
2. **Use Lightpanda to search**.
   - Open Google Search.
   - If a normal first-party consent screen appears, proceed only through standard user-directed acceptance; if it blocks results, stop and report.
   - Fill the query and submit.
   - Wait for result selectors before extracting links.
3. **Extract candidate results**.
   - Capture title, snippet if useful, landing URL, and any visible PDF/source URL.
4. **Open the most promising result pages**.
   - Prefer official project pages, DOI landing pages, publisher pages, arXiv, institutional repositories, or conference pages.
5. **Extract the final acquisition target**.
   - Best case: a direct PDF URL.
   - Otherwise: a DOI URL, publisher landing page, repository page, or source page that can lead to the PDF.
6. **Hand off to a separate downloader**.
   - Use a non-Lightpanda downloader to fetch the final URL.
   - Record the final resolved URL after redirects.
7. **Validate the file separately**.
   - Check response headers and/or PDF signature before treating the file as an acquired paper.

## Output Contract

Return structured notes with:

- query used
- result title
- landing URL
- extracted PDF URL if present
- fallback source URL if no direct PDF exists
- notes on access restrictions, consent screens, or ambiguity

## Stop Conditions

Stop and report instead of pushing through when you hit:

- CAPTCHA or unusual-traffic pages
- consent or login walls that block results
- unstable repeated failures loading Google Search
- ambiguous results where a human choice is needed

Do **not** add retry storms, proxy rotation, or bypass logic.

## Fallbacks

If Google Search blocks or fails:

1. Return the exact query you used.
2. Return any partial results already extracted.
3. Pivot to source-native discovery:
   - DOI resolver
   - publisher page
   - arXiv or repository search
   - exact-title search on known scholarly hosts

If no direct PDF URL is visible:

1. Return the best landing/source URL.
2. Note whether the next step is DOI resolution, repository navigation, or publisher download.
3. Do not claim the paper was acquired until the separate downloader succeeds and the file validates as a PDF.
