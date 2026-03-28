# Reorganize planning and spec documents under docs

## Goal

Move `docs/plan/`, `docs/report/`, and `docs/specs/` under `docs/` so repository documentation lives in one top-level area.

## Scope

- move the directories under `docs/`
- update the local Turso KB scripts and workflow docs
- update top-level references that point at the old paths

## Result

- moved `plan/`, `report/`, and `specs/` to `docs/plan/`, `docs/report/`, and `docs/specs/`
- updated workflow and README references to the new paths
- updated local Turso KB tracking so it indexes `docs/plan`, `docs/report`, and `docs/specs`
- preserved `sync-kb plan/...`, `sync-kb report/...`, and `sync-kb specs/...` as compatibility aliases
