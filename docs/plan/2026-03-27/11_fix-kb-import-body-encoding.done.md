# Fix local KB import body encoding

## Goal

Avoid preview `tursodb` aborts when importing large markdown files into the local KB.

## Plan

1. Replace the line-by-line SQL concatenation for document bodies with a single hex-encoded text literal.
2. Keep the FTS and binary-vector behavior unchanged.
3. Retry the rebuild and search flow after the importer is simplified.
