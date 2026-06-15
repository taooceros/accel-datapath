# PROJECT KNOWLEDGE BASE

Research monorepo for Intel DSA/IAX data-path work.

## DEFAULT MODE

Act first, document only when it helps the work.

- Start with concrete evidence: inspect code, run a focused command, edit the target file, or reproduce the failure.
- Write plans/reports/notes only when the user asks, or when the change affects research method, hardware behavior, public API shape, or cross-subsystem architecture.
- Keep replies short: changed files, verification, and any real blocker.
- Match code to specs, not specs to code.

## KNOWLEDGE LAYERS

- `AGENTS.md` — stable rules.
- `notes/now.md` — optional one-screen current-state snapshot.
- `notes/inbox.md` — optional messy capture buffer; clean it only on request.
- `docs/evidence/` — current-truth maps, one file per research thread.
- `docs/report/` — new evidence reports after results exist.
- `remark/NNN_<topic>.md` — durable single-point insights.
- `archive/docs/` — historical docs, specs, reports, plans, and related work.

## DOCUMENTATION RULES

- For small/local code changes, write code and run focused verification.
- For risky cross-subsystem or hardware-facing changes, write the shortest useful plan.
- For research-method or paper-facing changes, write a human-readable plan before changing the method.
- For new evidence, update the relevant `docs/evidence/<thread>.md` when the current conclusion changes.
- Use archive docs as reference material; curate them into active evidence only when needed.

A good evidence map contains: current conclusion, strongest evidence links, superseded/caution notes, and next reproducible action. It links raw data instead of copying it.

## CODE STYLE

- Keep code lean, explicit, and locally understandable.
- Remove weak abstractions.
- Share duplicate logic only when the abstraction makes reasoning easier.
- Keep unsafe/raw hardware boundaries narrow and out of high-level codec/application code.
- Keep child `AGENTS.md` files lean and local.

## HARDWARE BATCHING TERMINOLOGY

- Use `hw-eval/` as the hardware potential baseline when comparing accelerator submission strategies.
- Keep `batch` and `concurrency` separate in plans, reports, code names, and explanations.
- **Batch size (`batch_n`)**: number of logical operations submitted to hardware through one MMIO submission. Batch size 1 means one operation per MMIO submission.
- **Concurrency**: maximum number of logical operations outstanding at once, independent of how those operations are grouped into MMIO submissions.
- Convert outstanding submission slots before comparing: `logical_concurrency = batch_size * outstanding_submission_slots`.
- Say whether “batch size 1” means the no-batch/direct-descriptor baseline or a DSA `BATCH` descriptor containing one sub-descriptor.
- Keep hw-eval-specific strategy names and command-line trigger details in `hw-eval/AGENTS.md`.

## HARDWARE SAFETY

- Use `archive/docs/specs/*.md` and `archive/docs/report/architecture/001.design_decisions.md` before relying on DSA/IAX behavior.
- Use `launch` or `dsa_launcher` for hardware-facing binaries when the documented flow requires them.

## REPO MAP

```text
dsa-stdexec/   C++ stdexec sender/receiver framework
accel-rpc/     Rust accelerator-aware RPC workspace
hw-eval/       Hardware potential baseline benchmark harnesses
tools/         Launcher behavior
remark/        Durable single-point insights
archive/docs/  Historical docs tree, specs, reports, plans, related work
agents/        Agent operating material when explicitly needed
```

## COMMANDS

Use `rtk` for shell commands whenever available, including chained commands:

```bash
rtk cargo test
rtk cargo check
rtk git status
rtk git diff
rtk git add . && rtk git commit -m "message"
```

Run focused verification. Let the harness preserve full output artifacts.
