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

## Session State
- The orchestrator should pass explorer session state for resumed work.
- Treat that state as execution memory only, not repo knowledge.
- Expected fields: task id, objective, scope, rounds used, candidates found, rejected leads, unknowns, files already read, stop conditions seen, compaction count.
- Use the session state to avoid re-reading and to detect prior compaction or resumed-with-missing-context runs.
- Missing prior context means the resumed task lacks task id, rounds used, shortlist/rejections/unknowns, or files already read.

## Stop Conditions
Stop and return when:
- 3 strong candidates exist
- search budget is exhausted
- candidate set is expanding instead of shrinking
- task requires synthesis across subsystems
- likely context blowup is forming
- session state shows prior compaction or resumed work with missing prior context

## Compaction Response
If prior compaction or resumed-with-missing-context is detected:
- do not reopen broad search
- do not try to reconstruct all prior reasoning
- return immediately with the current shortlist and state gaps
- suggest 2-4 parallel sub-searches split by subtree, module, or subquestion

## Output Format
- Likely candidates
- Rejected leads
- Unknowns
- Status
- Recommended next step

Each candidate should include:
- path/symbol
- 1-line why relevant
- confidence
- next action

Status should include:
- rounds used / budget
- candidates found
- compaction count
- stop condition hit

Prefer path:line refs over snippets.

## Anti-patterns
- “Understand everything about X”
- “Find all files related to Y and explain architecture”
- reading broadly before narrowing
- retaining raw evidence instead of returning a shortlist
- continuing broad discovery after compaction
- using session memory as justification for final synthesis
