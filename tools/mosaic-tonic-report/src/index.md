---
title: Tonic bounded-matrix dashboard
sql:
  matrix: ./data/matrix_summary.csv
---

# Tonic bounded-matrix dashboard

This Observable Framework rewrite keeps the dashboard lightweight: page-first markdown, a repo-fed CSV attachment, a few useful global filters, and Mosaic views over the bounded tonic matrix.

> Data source: `results/tonic/2026-04-01-loop2/matrix_summary.csv`

```js
const globalFilters = vg.Selection.intersect();

const payloadSizeMenu = vg.menu({label: "Payload size (bytes)", from: "matrix", column: "payload_size", as: globalFilters});
const payloadKindMenu = vg.menu({label: "Payload kind", from: "matrix", column: "payload_kind", as: globalFilters});
const compressionMenu = vg.menu({label: "Compression", from: "matrix", column: "compression", as: globalFilters});
const runtimeMenu = vg.menu({label: "Runtime", from: "matrix", column: "runtime", as: globalFilters});

vg.hconcat(payloadSizeMenu, payloadKindMenu, compressionMenu, runtimeMenu)
```

## Throughput and latency views

```js
vg.vconcat(
  vg.plot(
    vg.lineY(vg.from("matrix", {filterBy: globalFilters}), {
      x: "concurrency",
      y: "throughput_rps_mean",
      stroke: "payload_size",
      z: "config"
    }),
    vg.dot(vg.from("matrix", {filterBy: globalFilters}), {
      x: "concurrency",
      y: "throughput_rps_mean",
      fill: "payload_size",
      symbol: "payload_kind",
      stroke: "compression"
    }),
    vg.xLabel("Concurrency"),
    vg.yLabel("Throughput (req/s)"),
    vg.width(760),
    vg.height(280)
  ),
  vg.plot(
    vg.lineY(vg.from("matrix", {filterBy: globalFilters}), {
      x: "concurrency",
      y: "latency_us_p99_mean",
      stroke: "runtime",
      z: "config"
    }),
    vg.dot(vg.from("matrix", {filterBy: globalFilters}), {
      x: "concurrency",
      y: "latency_us_p99_mean",
      fill: "runtime",
      symbol: "payload_kind",
      stroke: "compression"
    }),
    vg.xLabel("Concurrency"),
    vg.yLabel("p99 latency (us)"),
    vg.width(760),
    vg.height(280)
  )
)
```

## Regime comparison

```js
vg.plot(
  vg.dot(vg.from("matrix", {filterBy: globalFilters}), {
    x: "throughput_mib_s_mean",
    y: "latency_us_p99_mean",
    fill: "payload_size",
    symbol: "runtime",
    stroke: "compression",
    r: 6,
    title: "config"
  }),
  vg.xLabel("Throughput (MiB/s)"),
  vg.yLabel("p99 latency (us)"),
  vg.width(760),
  vg.height(360)
)
```

## Detail table

```js
vg.table({
  from: "matrix",
  filterBy: globalFilters,
  columns: [
    "config",
    "payload_size",
    "payload_kind",
    "compression",
    "runtime",
    "concurrency",
    "throughput_rps_mean",
    "throughput_mib_s_mean",
    "latency_us_p99_mean",
    "requests_completed_mean"
  ],
  width: 960,
  height: 420
})
```

## Notes

- This baseline intentionally drops the old localStorage persistence and per-chart brush isolation.
- The CSV is imported through a Framework data loader, so production builds copy a repo-local snapshot into the site output.
- If the upstream CSV changes, rerun the build so the generated attachment under `dist/` and the published artifact stay in sync.
