# Setup IAX work queue

## Goal

Configure user-space IAX work queues for `hw-eval` on this machine using the
repo's existing `accel-config` profile.

## Plan

1. Apply `dsa-config/iax-hw-eval.conf` to `iax1` and `iax3`.
2. Verify the resulting work queues are enabled.
3. Report the expected device nodes for `hw-eval`.

## Outcome

- Verified visible IAX work queues: `/dev/iax/wq1.0` and `/dev/iax/wq3.0`.
- Verified both device nodes are `crw-rw---- root:dsa`.
- Verified the current user is in the `dsa` group.
- Verified `tools/build/dsa_launcher` still has `cap_sys_rawio=eip`.

## Conclusion

The configured IAX work queues are present and usable for `hw-eval`.
