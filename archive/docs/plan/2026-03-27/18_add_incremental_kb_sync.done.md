# Add incremental local KB sync

## Goal

Provide an incremental way to add or update docs/plan/docs/report/spec files in the local Turso database without rebuilding everything every time.

## Plan

1. Extract the KB document encoding and SQL upsert logic into a shared helper.
2. Add a `sync-kb` utility that can upsert specific files or reconcile the tracked source sets incrementally.
3. Expose the utility through `devenv` and document the workflow.
