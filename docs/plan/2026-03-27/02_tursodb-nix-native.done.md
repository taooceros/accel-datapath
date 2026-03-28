# Rewrite nested `tursodb` setup to a Nix-native package

## Goal

Replace the imperative preview fetch script in `/home/hongtao/accel-datapath/agent-env-wt/tursodb` with a pinned Nix package so the nested `devenv` is self-contained and reproducible, while keeping the actual database file at `/home/hongtao/accel-datapath/knowledge.db`.

## Plan

1. Package the pinned preview `tursodb` binary directly in `devenv.nix` using a fixed-output fetch.
2. Remove the local fetch script and mutable version file from the nested setup.
3. Update the local README to describe the new `devenv shell` workflow and the parent-level database path.
