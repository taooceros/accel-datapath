# Fix root `devenv shell` failure from imported `tursodb` module

## Goal

Restore the root-level `devenv shell` by fixing the imported `tursodb` module's shell-string escaping.

## Plan

1. Escape shell `${...}` expansions inside the `devenv.nix` multiline strings so Nix stops interpreting them as Nix interpolation.
2. Keep the dynamic repo-root and `.turso/knowledge.db` behavior unchanged.
3. Leave validation to an explicit follow-up command.
