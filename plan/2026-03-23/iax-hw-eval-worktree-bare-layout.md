# Migrate to bare repo + worktree layout

## Goals
- Provide a clean workspace layout with a single bare git repository and branch-isolated working trees.
- Keep an `main` working tree and an `iax-hw-eval` working tree for hardware work.

## Current state
- Existing non-bare worktree: `/home/hongtao/accel-datapath` (branch `main`).
- Existing worktrees: `.wt-iax`, `/tmp/accel-datapath-iax`, and `/home/hongtao/accel-datapath/iax-hw-eval-worktree`.
- Existing local branches: `iax-hw-eval`, `iax-hw-eval-alt`, `iax-hw-eval-worktree`.

## Plan
1. Create a bare repository mirror from current source:
   - `/home/hongtao/accel-datapath.git`.
2. Create dedicated worktrees from the bare repository:
   - `/home/hongtao/accel-datapath-main` -> branch `main`
   - `/home/hongtao/accel-datapath-iax` -> branch `iax-hw-eval`.
3. Verify git directory metadata and branches in new worktrees.
4. Keep current repo as an optional legacy checkout and remove or prune temporary worktrees as needed.
