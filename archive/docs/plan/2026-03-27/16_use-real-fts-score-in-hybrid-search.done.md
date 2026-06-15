# Use real `fts_score` in hybrid local KB search

## Goal

Replace the hand-rolled lexical scoring in `search-kb` with real `fts_score(...)` contributions while keeping the search hybrid.

## Plan

1. Build one supported FTS subquery per normalized query term and join those scores back to the document rows.
2. Sum the per-term BM25 scores, add a small phrase bonus, and keep vector distance as the semantic signal.
3. Preserve the existing hybrid rank fusion output shape.
