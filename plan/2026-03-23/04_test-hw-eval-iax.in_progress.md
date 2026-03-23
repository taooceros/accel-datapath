# Test hw-eval IAX path

## Goal

Run a minimal hardware-backed smoke test of `hw-eval` in `iax` mode against an
enabled IAX work queue on this machine.

## Plan

1. Confirm the release binary and launcher are present.
2. Run a small `hw-eval --accel iax` invocation on `/dev/iax/wq1.0`.
3. Record the outcome and any failures in a report.
