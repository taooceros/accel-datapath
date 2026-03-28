# Fix codemogger runtime cache path

## Goal

Make the packaged `codemogger` CLI use writable repo-local cache directories instead of attempting to write into the Nix store at runtime.

## Scope

- inspect the current `codemogger` devenv module
- set runtime cache environment variables for the shell
- keep codemogger and model cache state under `.codemogger/`

## Result

- added a wrapper around the packaged `codemogger` binary
- preload a tiny module via `NODE_OPTIONS --import=...` to set `@huggingface/transformers` `env.cacheDir`
- redirected codemogger and model cache state to repo-local `.codemogger/`
- avoided patching upstream `dist/cli.mjs`, so version bumps should mainly be hash updates
