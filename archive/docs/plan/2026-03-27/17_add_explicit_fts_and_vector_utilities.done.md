# Add explicit FTS and vector search utilities

## Goal

Provide separate local KB utilities for FTS-only and vector-only retrieval so the retrieval source is explicit.

## Plan

1. Add dedicated scripts for FTS-only and vector-only search.
2. Expose those scripts through the `tursodb` devenv module.
3. Update docs so the retrieval mode is explicit at call time.
