# tools/

## dsa_launcher.c

`dsa_launcher` is a C11 utility that launches child processes with the
`CAP_SYS_RAWIO` capability required for user-space Intel DSA access.

### How It Works

DSA submission instructions (`_movdir64b`, `_enqcmd`) require `CAP_SYS_RAWIO`.
Rather than applying `setcap` to every built binary, `dsa_launcher` is granted
the capability once and passes it to child processes via **Linux ambient
capabilities** (`prctl PR_CAP_AMBIENT`). The child inherits the capability
without needing its own `setcap`.

### Build

```bash
gcc -o dsa_launcher dsa_launcher.c -lcap
```

### Setup

Apply the capability to the launcher binary once:

```bash
sudo setcap cap_sys_rawio+eip dsa_launcher
```

### Usage

```bash
./dsa_launcher ./build/linux/x86_64/release/dsa_benchmark [args...]
```

### The `run` Script

The `run` devenv script automates building `dsa_launcher` and the benchmark,
auto-detects the current xmake build mode, and invokes `dsa_launcher`
internally. In most cases, simply running `run` is sufficient.

See [CLAUDE.md](../CLAUDE.md) for full project documentation.
