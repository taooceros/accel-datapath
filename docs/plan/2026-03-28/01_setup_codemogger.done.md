# Setup codemogger in repo devenv

## Goal

Add `codemogger` as a repo-local tool managed through `devenv`, similar to the existing local Turso tooling.

## Scope

- create a `codemogger/` module for the CLI
- wire it into the root `devenv`
- keep codemogger state repo-local and gitignored
