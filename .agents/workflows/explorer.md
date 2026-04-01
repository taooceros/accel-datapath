# Explorer Workflow

## Purpose
Explorer is for fast, bounded discovery under uncertainty.

## Default Mode
- discover candidates
- prune lightly
- hand back early

Do not use explorer for end-to-end understanding by default.

## Allowed Task Shapes
1. Discovery-only
2. Shortlist comparison
3. Single ambiguity resolution

## Standard Budget
- Search rounds: 2
- Domains: 2
- Candidates: 8
- Reads per candidate: 1-2

## Stop Conditions
Stop and return when:
- 3 strong candidates exist
- search budget is exhausted
- candidate set is expanding instead of shrinking
- task requires synthesis across subsystems
- likely context blowup is forming

## Output Format
- Likely candidates
- Rejected leads
- Unknowns
- Recommended next step

Each candidate should include:
- path/symbol
- 1-line why relevant
- confidence
- next action

Prefer path:line refs over snippets.

## Anti-patterns
- “Understand everything about X”
- “Find all files related to Y and explain architecture”
- reading broadly before narrowing
- retaining raw evidence instead of returning a shortlist
