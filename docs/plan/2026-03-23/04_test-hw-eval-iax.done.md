# Test hw-eval IAX path

## Goal

Run a minimal hardware-backed smoke test of `hw-eval` in `iax` mode against an
enabled IAX work queue on this machine.

## Plan

1. Confirm the release binary and launcher are present.
2. Run a small `hw-eval --accel iax` invocation on `/dev/iax/wq1.0`.
3. Record the outcome and any failures in a report.

## Outcome

- Confirmed `tools/build/dsa_launcher` is executable.
- Rebuilt `hw-eval` successfully with `cargo build --release` inside `devenv`.
- Ran the minimal hardware-backed IAX smoke test on `/dev/iax/wq1.0`.
- The run completed successfully and produced valid `noop` and `crc64` latency/throughput JSON.
- Recorded the successful result in `docs/report/hw_eval_iax_crc64_hardware_smoke_test_2026-03-23.md`.
