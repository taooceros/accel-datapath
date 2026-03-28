# `dsa_launcher` permission failure analysis

## Summary

`tools/build/dsa_launcher` fails at:

```text
prctl PR_CAP_AMBIENT_RAISE: Operation not permitted
```

This still happens after setting the launcher file capability to
`cap_sys_rawio=eip`.

The immediate problem is not device-node permission and not the child binary.
The failure happens inside the launcher itself before `execvp()`.

## What I checked

### Intended design

From `tools/README.md` and `tools/dsa_launcher.c`, the design is:

1. Grant `CAP_SYS_RAWIO` to the launcher binary with file capabilities.
2. Have the launcher raise `CAP_SYS_RAWIO` into the ambient set.
3. `execvp()` the target program so the child inherits that ambient capability.

### Current launcher code

`tools/dsa_launcher.c` does only this before `execvp()`:

```c
prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_RAISE, CAP_SYS_RAWIO, 0, 0)
```

It does **not** add `CAP_SYS_RAWIO` to the process inheritable set first.

### Current binary state

Observed capability on the built launcher:

```text
tools/build/dsa_launcher cap_sys_rawio=eip
```

### Reproduction

Minimal reproduction:

```bash
./tools/build/dsa_launcher /usr/bin/true
```

Observed result:

```text
prctl PR_CAP_AMBIENT_RAISE: Operation not permitted
Hint: launcher must have file capability cap_sys_rawio+eip
```

## Root cause

`PR_CAP_AMBIENT_RAISE` requires the capability to already be present in both:

- the process permitted set
- the process inheritable set

The launcher gets `CAP_SYS_RAWIO` in its permitted/effective sets from the file
capability, but it never places that capability into its **process inheritable
set**.

That means the `prctl(... PR_CAP_AMBIENT_RAISE ...)` call fails with `EPERM`,
even when the file itself is marked `eip`.

## Important implication

The current hint in `tools/dsa_launcher.c` is misleading:

```text
Hint: launcher must have file capability cap_sys_rawio+eip
```

Adding `i` on the **file capability** is not sufficient by itself for this
launcher implementation. The launcher must explicitly update its process
capability sets before trying to raise the ambient capability.

## Code-level fix

Before calling `PR_CAP_AMBIENT_RAISE`, the launcher should:

1. read its current process capabilities
2. add `CAP_SYS_RAWIO` to the inheritable set
3. apply the updated capability set to itself
4. then call `PR_CAP_AMBIENT_RAISE`

In practice this means using libcap APIs such as:

- `cap_get_proc()`
- `cap_set_flag(..., CAP_INHERITABLE, ...)`
- `cap_set_proc()`
- `cap_free()`

Only after that should the launcher call:

```c
prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_RAISE, CAP_SYS_RAWIO, 0, 0)
```

## Minimal conclusion

The launcher currently fails because it tries to raise an ambient capability
without first putting `CAP_SYS_RAWIO` into its process inheritable set.

`setcap cap_sys_rawio+eip tools/build/dsa_launcher` is necessary for the
intended design, but it is not sufficient for the current implementation.
