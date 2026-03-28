# Move Turso DB to repo-local `.turso`

## Goal

Store the dynamically-created Turso database under the current repository at `.turso/knowledge.db` and keep it out of git.

## Plan

1. Update the nested `tursodb` devenv module to derive `TURSODB_DB_PATH` as `repo/.turso/knowledge.db`.
2. Create `.turso/` automatically when entering the shell.
3. Ignore `.turso/` at the repository root and update the nested `tursodb` README.
