# Setup codemogger with devenv

## Goal

Add `codemogger` as a repo-local tool managed through `devenv`.

## Scope

- inspect the upstream package metadata
- add a `codemogger/` module
- import it from the root `devenv`
- keep local codemogger state out of git

## Result

- added a repo-local `codemogger/` devenv module
- imported it from the root `devenv`
- vendored `codemogger/package-lock.json` to make the Nix build reproducible
- pinned the npm tarball and npm dependency hash in `codemogger/devenv.nix`
- configured the package build to skip npm install scripts so it does not fetch `onnxruntime` during the Nix build
- gitignored `.codemogger/` for local codemogger state
