# Make `tursodb` database path dynamic

## Goal

Remove the hardcoded database path from the nested `tursodb` devenv module so the database location follows the current checkout path.

## Plan

1. Compute the repository root dynamically when entering the shell.
2. Export `TURSODB_DB_PATH` as `parent-of-repo/knowledge.db`.
3. Update the nested `tursodb` README to describe the dynamic path behavior.
