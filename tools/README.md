# tools/

## dsa_launcher.c

`dsa_launcher` is a C11 utility that launches child processes with the
`CAP_SYS_RAWIO` capability required for user-space Intel DSA/IAX access.

### How It Works

DSA submission instructions (`_movdir64b`, `_enqcmd`) require `CAP_SYS_RAWIO`.
Rather than applying `setcap` to every built binary, `dsa_launcher` is granted
the capability once and passes it to child processes via **Linux ambient
capabilities** (`prctl PR_CAP_AMBIENT`). The child inherits the capability
without needing its own `setcap`.

### Build

```bash
gcc -o dsa_launcher dsa_launcher.c -lcap
sudo setcap cap_sys_rawio+eip dsa_launcher
```

### Usage

```bash
# Via devenv script (recommended — auto-builds and locates the binary):
launch ./target/release/hw-eval
launch ./dsa-stdexec/build/linux/x86_64/release/dsa_benchmark

# Direct invocation:
./tools/build/dsa_launcher <program> [args...]
```

### The `launch` Script

The `launch` devenv script automates building `dsa_launcher`, and invokes it
to wrap any command with `CAP_SYS_RAWIO`. Works from any directory in the repo.
