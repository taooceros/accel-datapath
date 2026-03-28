# Make local KB search hybrid

## Goal

Change `search-kb` from FTS-first fallback behavior to a true hybrid search that combines lexical and vector signals for every document.

## Plan

1. Compute lexical and vector scores for all documents in one query.
2. Derive simple lexical and vector ranks in SQL and fuse them into a hybrid score.
3. Update the docs and test the search helper against the current local KB.
