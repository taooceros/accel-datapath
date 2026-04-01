Goal: locate {thing}

Scope:
- Only inspect {subtrees/modules}
- Do not inspect {excluded areas} unless needed for disambiguation

Allowed sources:
- {search/index tools}
- Read only top-level or obviously relevant files if needed

Session state:
- Task id: {task_id}
- Objective: {objective}
- Scope: {scope}
- Rounds already used: {rounds_used}
- Prior shortlist/rejections/unknowns: {memory_summary}
- Files already read: {files_read}
- Stop conditions already seen: {stop_conditions_seen}
- Compaction count: {compaction_count}

Budget:
- Max {N} search rounds
- Max {M} candidates
- Max {K} reads per candidate

Hard stop when:
- {P} plausible candidates found, or
- budget exhausted
- candidate set is expanding instead of shrinking
- prior compaction or resumed-with-missing-context is detected

Missing prior context means the resumed task lacks task id, rounds used, shortlist/rejections/unknowns, or files already read.

If prior compaction or resumed-with-missing-context is detected:
- stop immediately
- do not reopen broad discovery
- return current findings and state gaps under Unknowns
- propose 2-4 parallel sub-searches under Recommended next step

Do not do final synthesis or implementation.

Return exactly:
- Likely candidates
- Rejected leads
- Unknowns
- Status
- Recommended next step

Status:
- rounds_used={n}/{N}
- candidates_found={n}
- compaction_count={n}
- stop_condition={condition}
