# Make local KB search robust against preview FTS quirks

## Goal

Avoid relying on preview `tursodb` FTS features that behave inconsistently, while keeping search quality reasonable.

## Plan

1. Replace `fts_match(...)` and built-in FTS score usage with term-by-term `MATCH` predicates that work in the pinned preview.
2. Rank FTS hits with a simple lexical score derived from title/body term and phrase presence.
3. Keep vector distance as the fallback ordering when the FTS filter finds no hits.
