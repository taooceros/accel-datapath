# Set up local `tursodb` devenv wrapper

## Goal

Create a self-contained `devenv` setup under `/home/hongtao/accel-datapath/agent-env-wt/tursodb` for the preview `tursodb` CLI, while keeping the actual database file in the parent directory at `/home/hongtao/accel-datapath/knowledge.db`.

## Plan

1. Add a standalone `devenv.nix` and `devenv.yaml` inside `tursodb/` so the nested environment does not depend on the repo root configuration.
2. Pin the current preview version, add a small fetch script, and download the Linux `x86_64` preview archive into the nested setup folder.
3. Create the parent-level database file placeholder and document the open flow for `tursodb`.
