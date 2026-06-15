# Add repo-local literature-review skill

## Goal

Add a repo-local `literature-review` skill so the workflow lives with the repository instead of user-level OpenCode skills.

## Scope

- create a project-local skill entrypoint at `.claude/skills/literature-review/SKILL.md`
- preserve the requested literature-review workflow structure
- adapt path references to the files and folders that actually exist in this repository

## Result

- added `.claude/skills/literature-review/SKILL.md` as a repo-local skill entrypoint
- kept the requested discover → process → clarify → synthesize workflow shape
- rewrote path references to match current repo reality, especially `docs/related work/`, `docs/research_plan.md`, and the Turso KB helpers
