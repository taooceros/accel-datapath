# Mosaic tonic report

Observable Framework + Mosaic dashboard for exploring the bounded tonic profiling matrix.

## Package manager

This app is managed with Bun.

## Commands

```bash
bun install
bun run dev
bun run build
bun run build:artifact
```

- `bun run dev` starts the Observable Framework preview server for `tools/mosaic-tonic-report/src/` on `http://127.0.0.1:4173/`.
- `bun run build` writes the local static site to `dist/`.
- `bun run build:artifact` refreshes `docs/report/artifacts/003.tonic_bounded_matrix_mosaic/` from `dist/`.

The production build is emitted to:

- `docs/report/artifacts/003.tonic_bounded_matrix_mosaic/`

The app expects the tonic CSV at:

- `results/tonic/2026-04-01-loop2/matrix_summary.csv`

Observable Framework ingests that CSV through `src/data/matrix_summary.csv.js`, which copies the repo-local file into the generated site during build.

If `results/tonic/2026-04-01-loop2/matrix_summary.csv` changes, rerun the build; the built artifact contains a generated attachment snapshot rather than reading the CSV live at runtime.
