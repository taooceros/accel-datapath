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


<!-- rtk-instructions v2 -->
# RTK (Rust Token Killer) - Token-Optimized Commands

## Golden Rule

**Always prefix commands with `rtk`**. If RTK has a dedicated filter, it uses it. If not, it passes through unchanged. This means RTK is always safe to use.

**Important**: Even in command chains with `&&`, use `rtk`:
```bash
# ❌ Wrong
git add . && git commit -m "msg" && git push

# ✅ Correct
rtk git add . && rtk git commit -m "msg" && rtk git push
```

## RTK Commands by Workflow

### Build & Compile (80-90% savings)
```bash
rtk cargo build         # Cargo build output
rtk cargo check         # Cargo check output
rtk cargo clippy        # Clippy warnings grouped by file (80%)
rtk tsc                 # TypeScript errors grouped by file/code (83%)
rtk lint                # ESLint/Biome violations grouped (84%)
rtk prettier --check    # Files needing format only (70%)
rtk next build          # Next.js build with route metrics (87%)
```

### Test (60-99% savings)
```bash
rtk cargo test          # Cargo test failures only (90%)
rtk go test             # Go test failures only (90%)
rtk jest                # Jest failures only (99.5%)
rtk vitest              # Vitest failures only (99.5%)
rtk playwright test     # Playwright failures only (94%)
rtk pytest              # Python test failures only (90%)
rtk rake test           # Ruby test failures only (90%)
rtk rspec               # RSpec test failures only (60%)
rtk test <cmd>          # Generic test wrapper - failures only
```

### Git (59-80% savings)
```bash
rtk git status          # Compact status
rtk git log             # Compact log (works with all git flags)
rtk git diff            # Compact diff (80%)
rtk git show            # Compact show (80%)
rtk git add             # Ultra-compact confirmations (59%)
rtk git commit          # Ultra-compact confirmations (59%)
rtk git push            # Ultra-compact confirmations
rtk git pull            # Ultra-compact confirmations
rtk git branch          # Compact branch list
rtk git fetch           # Compact fetch
rtk git stash           # Compact stash
rtk git worktree        # Compact worktree
```

Note: Git passthrough works for ALL subcommands, even those not explicitly listed.

### GitHub (26-87% savings)
```bash
rtk gh pr view <num>    # Compact PR view (87%)
rtk gh pr checks        # Compact PR checks (79%)
rtk gh run list         # Compact workflow runs (82%)
rtk gh issue list       # Compact issue list (80%)
rtk gh api              # Compact API responses (26%)
```

### JavaScript/TypeScript Tooling (70-90% savings)
```bash
rtk pnpm list           # Compact dependency tree (70%)
rtk pnpm outdated       # Compact outdated packages (80%)
rtk pnpm install        # Compact install output (90%)
rtk npm run <script>    # Compact npm script output
rtk npx <cmd>           # Compact npx command output
rtk prisma              # Prisma without ASCII art (88%)
```

### Files & Search (60-75% savings)
```bash
rtk ls <path>           # Tree format, compact (65%)
rtk read <file>         # Code reading with filtering (60%)
rtk grep <pattern>      # Search grouped by file (75%). Format flags (-c, -l, -L, -o, -Z) run raw.
rtk find <pattern>      # Find grouped by directory (70%)
```

### Analysis & Debug (70-90% savings)
```bash
rtk err <cmd>           # Filter errors only from any command
rtk log <file>          # Deduplicated logs with counts
rtk json <file>         # JSON structure without values
rtk deps                # Dependency overview
rtk env                 # Environment variables compact
rtk summary <cmd>       # Smart summary of command output
rtk diff                # Ultra-compact diffs
```

### Infrastructure (85% savings)
```bash
rtk docker ps           # Compact container list
rtk docker images       # Compact image list
rtk docker logs <c>     # Deduplicated logs
rtk kubectl get         # Compact resource list
rtk kubectl logs        # Deduplicated pod logs
```

### Network (65-70% savings)
```bash
rtk curl <url>          # Compact HTTP responses (70%)
rtk wget <url>          # Compact download output (65%)
```

### Meta Commands
```bash
rtk gain                # View token savings statistics
rtk gain --history      # View command history with savings
rtk discover            # Analyze Claude Code sessions for missed RTK usage
rtk proxy <cmd>         # Run command without filtering (for debugging)
rtk init                # Add RTK instructions to CLAUDE.md
rtk init --global       # Add RTK to ~/.claude/CLAUDE.md
```

## Token Savings Overview

| Category | Commands | Typical Savings |
|----------|----------|-----------------|
| Tests | vitest, playwright, cargo test | 90-99% |
| Build | next, tsc, lint, prettier | 70-87% |
| Git | status, log, diff, add, commit | 59-80% |
| GitHub | gh pr, gh run, gh issue | 26-87% |
| Package Managers | pnpm, npm, npx | 70-90% |
| Files | ls, read, grep, find | 60-75% |
| Infrastructure | docker, kubectl | 85% |
| Network | curl, wget | 65-70% |

Overall average: **60-90% token reduction** on common development operations.
<!-- /rtk-instructions -->