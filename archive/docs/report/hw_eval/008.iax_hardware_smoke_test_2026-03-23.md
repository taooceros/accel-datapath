# `hw-eval` IAX hardware smoke test (2026-03-23)

## Summary

`hw-eval` rebuilt successfully from the current worktree, but the minimal
hardware-backed IAX smoke test could not open `/dev/iax/wq1.0`.

The observed runtime failure is:

```text
Failed to open /dev/iax/wq1.0: Permission denied (os error 13) (need CAP_SYS_RAWIO or run via dsa_launcher)
```

## Commands run

Build:

```bash
cd hw-eval
cargo build --release
```

Result:

```text
Finished `release` profile [optimized] target(s) in 0.08s
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

Result:

```text
Failed to open /dev/iax/wq1.0: Permission denied (os error 13) (need CAP_SYS_RAWIO or run via dsa_launcher)
```

## Environment observations

Visible IAX work queues:

```text
/dev/iax/wq1.0
/dev/iax/wq3.0
```

Device-node permissions for the queue used in the test:

```text
crw------- 1 root dsa ... /dev/iax/wq1.0
```

Current user:

```text
uid=1001(hongtao) gid=1001(hongtao) groups=1001(hongtao),27(sudo),30001(dsa)
```

Non-interactive sudo was not available:

```text
sudo: a password is required
```

## Conclusion

The current worktree builds, but I could not complete a real hardware-backed
IAX benchmark run from this session because opening `/dev/iax/wq1.0` failed
with `EACCES`.

The immediate blocker is environment permission, not a compile failure in
`hw-eval`.
