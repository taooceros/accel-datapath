# Improve local KB search ranking and binary embeddings

## Goal

Make `search-kb` prefer real FTS matches and improve the deterministic binary embedding used for vector fallback.

## Plan

1. Revise the binary embedding generator from simple bit-setting to a signed token-hash sketch.
2. Update `search-kb` to return only FTS hits when any exist, with vector distance as a secondary ranking signal.
3. Keep the rebuild path compatible with the current preview `tursodb` shell.
