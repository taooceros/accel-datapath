---
name: google-scholar
description: Low-volume scholarly discovery workflow using Google Scholar with Lightpanda for search, navigation, and result extraction. Use when you need paper discovery, citation metadata, visible PDF/source links, or source landing pages, then hand off acquisition to a separate downloader.
---

# Google Scholar Workflow

Use this skill for focused scholarly discovery, not bulk collection. Treat Google Scholar as a fragile, policy-sensitive surface.

## When to Use

- Looking up a known paper title.
- Finding a paper's visible PDF link, DOI landing page, publisher page, or repository mirror.
- Gathering citation metadata such as title, authors, venue, year, and related versions.
- Finding review papers or newer follow-up work with a narrow scholarly query.

## When Not to Use

- Bulk harvesting.
- Exhaustive pagination across many result pages.
- Any workflow that assumes Google Scholar has a stable official automation API.
- Any workflow that assumes Lightpanda directly downloads and stores PDFs.

## Policy and Capability Boundary

Ground the skill in these constraints:

- Google Scholar does **not** provide a stable official automation API for this workflow.
- Official Scholar guidance discourages bulk automated access and asks automated software to respect robots guidance.
- Use Google Scholar only for low-volume, user-directed discovery.
- Lightpanda is for **search, navigation, page interaction, and result extraction**, not native paper download.
- PDF acquisition must be a **separate downloader** step followed by **separate PDF validation**.

## Query Construction

Prefer precise Scholar-style queries:

- exact paper title in quotes
- title-focused search when you know the exact name
- author filter when the title is ambiguous
- publication or venue filter when useful
- year bounds when narrowing follow-up work

Refine the query instead of paging deeply.

## Workflow

1. **Start with the narrowest plausible scholarly query**.
   - Exact title first when known.
2. **Use Lightpanda to query Scholar cautiously**.
   - Open Google Scholar.
   - If a normal first-party consent screen appears, proceed only through standard user-directed acceptance; if it blocks results, stop and report.
   - Fill the query and submit.
   - Wait for result selectors before extraction.
3. **Extract result metadata**.
   - Capture title, authors, year, venue if visible, Scholar result URL, and snippet text if useful.
4. **Inspect acquisition affordances in order**.
   - visible `[PDF]` or `[HTML]` links
   - `All versions`
   - primary result landing page
   - DOI or publisher/source page if visible
5. **Extract the final acquisition target**.
   - Best case: direct PDF URL.
   - Otherwise: repository page, publisher page, DOI resolver, or source landing page.
6. **Hand off to a separate downloader**.
   - Fetch the final URL outside Lightpanda.
   - Record the final resolved URL after redirects.
7. **Validate the file separately**.
   - Confirm content type and/or PDF signature before treating the file as acquired.

## Output Contract

Return structured notes with:

- query used
- title
- authors if visible
- year or venue if visible
- Scholar result URL
- extracted PDF URL if present
- fallback DOI/source/publisher URL if no direct PDF exists
- notes on rate limits, consent, access restrictions, or uncertainty

## Stop Conditions

Stop and report instead of pushing further when you hit:

- CAPTCHA, unusual-traffic pages, or blocking consent flows
- obvious rate limiting
- repeated Scholar failures suggesting the workflow is no longer safe or stable
- ambiguous matches that require a human choice

Do **not** add retry storms, proxy rotation, or bypass logic.

## Fallbacks

If Scholar blocks or becomes unstable:

1. Return the exact query and any partial metadata already extracted.
2. Pivot to other discovery methods:
   - exact-title Google Search
   - DOI resolution
   - publisher or conference site
   - repository search such as arXiv or institutional mirrors

If no direct PDF is visible:

1. Return the best DOI/source/publisher URL.
2. Note whether `All versions`, repository search, or publisher navigation is the next step.
3. Do not claim the paper was acquired until the separate downloader succeeds and the file validates as a PDF.
