# `hw-eval` IAX memmove failure: completion status `0x12` (2026-03-23)

## Summary

After fixing device-node permissions for `/dev/iax/wq1.0` and
`/dev/iax/wq3.0`, the hardware-backed IAX smoke test no longer fails at
`open(2)`. It now reaches the IAX execution path and fails reproducibly during
the `memmove` benchmark with completion status `0x12`.

This failure occurs on both visible IAX work queues:

- `/dev/iax/wq1.0`
- `/dev/iax/wq3.0`

## Commands run

Permission preflight:

```bash
ls -l /dev/iax/wq1.0 /dev/iax/wq3.0
id
getcap tools/build/dsa_launcher
```

Observed state:

```text
crw-rw---- 1 root dsa ... /dev/iax/wq1.0
crw-rw---- 1 root dsa ... /dev/iax/wq3.0
uid=1001(hongtao) gid=1001(hongtao) groups=1001(hongtao),27(sudo),30001(dsa)
tools/build/dsa_launcher cap_sys_rawio=eip
```

Smoke test on `wq1.0`:

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

Smoke test on `wq3.0`:

```bash
timeout 30s ./tools/build/dsa_launcher \
  ./hw-eval/target/release/hw-eval \
  --accel iax \
  --device /dev/iax/wq3.0 \
  --iterations 10 \
  --sizes 64 \
  --max-concurrency 4 \
  --json
```

Result on both queues:

```text
thread 'main' panicked at src/main.rs:1089:17:
IAX memmove failed: status=0x12 size=64
```

## Source-level observations

The failure is raised by the explicit IAX completion check in
`hw-eval/src/main.rs`:

```rust
} else if status != iax::IAX_COMP_SUCCESS {
    panic!("IAX memmove failed: status={:#x} size={}", status, size);
}
```

The IAX memmove descriptor builder in `hw-eval/src/iax.rs` sets:

```rust
self.src1_addr = src as u64;
self.dst_addr = dst as u64;
self.src1_size = size;
self.max_dst_size = size;
```

## Spec correlation

From `dsa_architecture_spec.md`:

- Completion status `0x12` means:
  - `Non-zero reserved field (other than a flag in the Flags field).`
- For the `Memory Move` operation, reserved descriptor fields are:
  - `Bytes 38-63`

## Interpretation

The hardware is accepting submissions and writing completions, but rejecting
the IAX memory-move descriptor as malformed.

The strongest current hypothesis is that `hw-eval/src/iax.rs` is populating a
field that is reserved for the `Memory Move` opcode. In particular,
`max_dst_size` is being set for `fill_memmove()`, while the spec says bytes
38-63 are reserved for `Memory Move`.

## Conclusion

The current blocker is no longer environment permission. The current worktree's
IAX `memmove` descriptor layout appears incompatible with the hardware/spec,
and that prevents the IAX benchmark suite from progressing beyond the first
data-moving operation.
