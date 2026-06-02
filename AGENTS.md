# PROJECT KNOWLEDGE BASE

Research monorepo for Intel DSA/IAX data-path work.

## CONVENTIONS

- Keep durable detail in plans, reports, and remarks rather than transient notes.
- Keep commit headlines short and consistent with current style.
- Write commits in focused, reviewable increments, but not so small that they lose a coherent unit of work.
- Write a human project plan in `docs/plan/YYYY-MM-DD/NN.<topic>.<state>.md` before non-trivial project changes. State the goal, scope, intended changes, verification, and completion notes in plain language.
- Always write agent execution/workflow plans in `agents/plan/YYYY-MM-DD/NN.<topic>.<state>.md`.
- Write findings to `docs/report/<topic>/NNN.<descriptor>.<ext>`.
- Write single-point insights to `remark/NNN_<topic>.md`.
- Match code to specs, not specs to code, unless explicitly told otherwise.
- **Code Elegance & Lean Design:** Keep code lean. Do not repeat duplicate logic; abstract if possible, or keep it perfectly clean if unavoidable. If code looks heavyweight with excessive conditionals, stop and design a more elegant approach.
- Keep child `AGENTS.md` files lean and local; do not repeat parent guidance within them.

## DO NOT
- Guess DSA/IAX behavior if `docs/specs/*.md` or `docs/report/architecture/001.design_decisions.md` already cover it.
- Run hardware-facing binaries directly when the documented flow requires `launch` or `dsa_launcher`.

## HARDWARE BATCHING TERMINOLOGY
- Use `hw-eval/` as the hardware potential baseline when comparing accelerator submission strategies.
- Keep `batch` and `concurrency` separate in plans, reports, and code names.
- **Batch size (`batch_n`):** The number of logical operations submitted to hardware through one MMIO submission. Batch size 1 means one operation per MMIO submission.
- **Concurrency:** The maximum number of logical operations outstanding at once, independent of how those operations are grouped into MMIO submissions.
- If a benchmark reports outstanding submission slots instead of logical operations, convert before comparing:
  $$logical\_concurrency = batch\_size \times outstanding\_submission\_slots$$
- Avoid ambiguous phrases like “batch size 1” without specifying whether it means the no-batch/direct-descriptor baseline or a DSA `BATCH` descriptor containing one sub-descriptor.
- Keep hw-eval-specific strategy names and command-line trigger details in `hw-eval/AGENTS.md`.

## REPO MAP
```text
dsa-stdexec/  C++ stdexec sender/receiver framework
accel-rpc/    Rust accelerator-aware RPC workspace
hw-eval/      Hardware potential baseline benchmark harnesses
docs/         Plans, reports, specs, related work
tools/        Launcher behavior
.agents/      Hidden configuration directory for agent tooling templates/workflows