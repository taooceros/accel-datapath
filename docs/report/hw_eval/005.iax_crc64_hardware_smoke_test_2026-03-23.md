# `hw-eval` IAX CRC64 hardware smoke test (2026-03-23)

## Summary

The spec-aligned `CRC64` IAX path completed a minimal hardware-backed smoke
test successfully on `/dev/iax/wq1.0`.

## Preconditions

- Visible IAX work queues: `/dev/iax/wq1.0`, `/dev/iax/wq3.0`
- Device-node mode on both queues: `crw-rw---- root:dsa`
- Current user is in the `dsa` group
- `tools/build/dsa_launcher` has `cap_sys_rawio=eip`

## Commands run

Release build:

```bash
devenv shell -- bash -lc 'cd hw-eval && cargo build --release'
```

Hardware smoke test:

```bash
timeout 30s ./tools/build/dsa_launcher \
  ./hw-eval/target/release/hw-eval \
  --accel iax \
  --device /dev/iax/wq1.0 \
  --iterations 10 \
  --sizes 64 \
  --max-concurrency 4 \
  --json
```

## Result

The run exited successfully and returned JSON with valid `noop`, `crc64`, and
`burst_crc64` measurements.

Key observations from the result:

- `metadata.device`: `/dev/iax/wq1.0`
- `metadata.wq_dedicated`: `true`
- `noop` median latency: `607 ns`
- `crc64` median latency at 64 B: `1051 ns`
- `burst_crc64` throughput at concurrency 4: `1.90 Mops/s`
- `crc64` sliding-window throughput at concurrency 4: `2.80 Mops/s`

## Conclusion

The current `hw-eval` IAX backend now passes both hardware-independent unit
tests and a real hardware-backed CRC64 smoke test. The previous `memmove`
descriptor failure is no longer on the active path.
