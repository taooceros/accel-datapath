# Tighten local KB FTS querying

## Goal

Improve practical local KB retrieval by making FTS queries more selective and optimizing the index after bulk rebuilds.

## Plan

1. Optimize the FTS index after rebuild.
2. Normalize multi-term search queries into `AND`-joined FTS queries.
3. Keep vector fallback behavior unchanged when FTS has no hits.
