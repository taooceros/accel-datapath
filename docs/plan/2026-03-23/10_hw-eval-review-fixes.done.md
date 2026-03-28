# Plan: hw-eval review fixes

## Goal

Address the two review findings from the latest `hw-eval` IAX backend commit:

1. Select the default work-queue device based on `--accel`.
2. Ensure sliding-window throughput benchmarks only report work that was
   actually issued.

## Scope

- `hw-eval/src/main.rs`
- `hw-eval/README.md`

## Notes

- Keep the change set narrow.
- Do not run validation in this step.
