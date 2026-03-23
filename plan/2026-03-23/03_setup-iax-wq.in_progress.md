# Setup IAX work queue

## Goal

Configure user-space IAX work queues for `hw-eval` on this machine using the
repo's existing `accel-config` profile.

## Plan

1. Apply `dsa-config/iax-hw-eval.conf` to `iax1` and `iax3`.
2. Verify the resulting work queues are enabled.
3. Report the expected device nodes for `hw-eval`.
