# Validate KB end to end

## Goal

Exercise the local KB flows until incremental sync and search utilities behave correctly on the pinned `tursodb` preview.

## Scope

- validate targeted sync
- validate hybrid, FTS-only, and vector-only search commands
- validate deletion of a tracked source from the database
- fix any preview-specific incompatibilities found during testing

## Result

- verified targeted `sync-kb` on an existing tracked file
- verified `search-kb`, `search-kb-fts`, and `search-kb-vector`
- verified tracked-file deletion by removing a probe file and syncing its old path
- verified full `sync-kb` removes stale rows from renamed plan files
