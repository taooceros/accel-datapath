# Add repo-local google search skills

## Goal

Add repo-local `google-search` and `google-scholar` skills that encode a safe browser-assisted discovery workflow using Lightpanda for search and navigation, while keeping PDF acquisition as a separate downloader step.

## Scope

- add `.claude/skills/google-search/SKILL.md`
- add `.agents/skills/google-search/SKILL.md`
- add `.claude/skills/google-scholar/SKILL.md`
- add `.agents/skills/google-scholar/SKILL.md`
- keep the `.agents` copies mirrored from `.claude`
- encode Lightpanda accurately as search/navigation/extraction tooling, not as a native PDF downloader

## Result

- added mirrored `google-search` and `google-scholar` repo-local skills
- documented low-volume Google and Google Scholar discovery flows with explicit stop conditions
- required downloader handoff and PDF validation instead of claiming Lightpanda directly downloads papers
