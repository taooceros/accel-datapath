# Fix `rebuild-kb` for preview `tursodb`

## Goal

Make the local KB rebuild work against the current preview `tursodb` shell.

## Plan

1. Remove the `FTS5` dependency from the first-pass schema because the preview build does not expose that module.
2. Change the importer to emit a safe SQL body expression instead of raw multiline SQL string literals.
3. Make the rebuild script fail if the `tursodb` shell prints parse errors.
