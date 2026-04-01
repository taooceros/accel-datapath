Goal: locate {thing}

Scope:
- Only inspect {subtrees/modules}
- Do not inspect {excluded areas} unless needed for disambiguation

Allowed sources:
- {search/index tools}
- Read only top-level or obviously relevant files if needed

Budget:
- Max {N} search rounds
- Max {M} candidates
- Max {K} reads per candidate

Stop when:
- {P} plausible candidates found, or
- budget exhausted

Return:
- Likely candidates
- Rejected leads
- Unknowns
- Recommended next step
