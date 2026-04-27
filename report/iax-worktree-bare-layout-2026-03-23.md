# IAX hw-eval worktree migration report (2026-03-23)

## Goal
Set up a bare repository workflow for `hw-eval` with a clean, non-dot, branch-oriented directory layout.

## Executed steps
1. Created plan: `plan/2026-03-23/iax-hw-eval-worktree-bare-layout.md`.
2. Removed previous temporary worktrees:
   - `/home/hongtao/accel-datapath/.wt-iax`
   - `/home/hongtao/accel-datapath/iax-hw-eval-worktree`
3. Created bare repository mirror at `/tmp/accel-datapath.git`.
4. Added worktrees from bare repo:
   - `/tmp/accel-datapath-main` (branch `main`)
   - `/tmp/accel-datapath-iax` (branch `iax-hw-eval`)

## Result
- Bare repo exists and is usable:
  - `git --git-dir=/tmp/accel-datapath.git worktree list`
- Both new worktrees are clean checks.

## Notes
- Existing main worktree remains at `/home/hongtao/accel-datapath`.
- Because of sandbox path permissions, the new bare repo/layout is under `/tmp` instead of a peer of `/home/hongtao`.
