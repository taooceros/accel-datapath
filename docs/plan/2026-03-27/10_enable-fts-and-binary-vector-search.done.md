# Enable Turso FTS and exact binary vector search for the local KB

## Goal

Revise the local Turso KB setup so it uses Turso FTS correctly and stores deterministic binary embeddings for exact vector search without relying on ANN indexing.

## Plan

1. Update the schema to create a Turso FTS index and store a binary vector embedding per document.
2. Revise the rebuild script to recreate the database, build the FTS index under the experimental flag, and populate deterministic `vector1bit` embeddings.
3. Add a small query helper and update the docs for the new local search flow.
