# Add repo-local tursodb KB skill

## Goal

Add a dedicated repo-local skill for the `tursodb` knowledge-base workflow so agents are guided to use the local KB through a first-class skill instead of treating `search-kb*` as undocumented shell trivia.

## Scope

- add `.agents/skills/tursodb-kb/SKILL.md`
- document when to use hybrid, FTS-only, and vector-only KB search
- capture the markdown-only ingestion constraints for `rebuild-kb` and `sync-kb`
- update repo guidance to point to the skill as the preferred KB workflow entry point
- update the literature workflow to refer to the new skill rather than only raw commands

## Result

- repo gains a dedicated `tursodb-kb` skill for local knowledge-base lookup and maintenance
- root workflow docs point agents to the skill while still naming the underlying commands
- literature-review guidance now treats KB retrieval as a reusable repo workflow instead of ad hoc shell usage
