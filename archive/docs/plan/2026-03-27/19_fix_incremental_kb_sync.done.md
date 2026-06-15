# Fix incremental KB sync

## Goal

Make `sync-kb` work on `tursodb v0.6.0-pre.11` without relying on unsupported `TEMPORARY` tables.

## Approach

- inspect the current sync script
- replace the temp-table reconciliation path with a compatible SQL flow
- rerun full sync and targeted sync

## Result

- removed `TEMPORARY` table usage from `sync-kb`
- switched full reconciliation to an inline `NOT IN (...)` path list
- verified both `sync-kb` and `sync-kb docs/plan/2026-03-27/18_add_incremental_kb_sync.done.md`
