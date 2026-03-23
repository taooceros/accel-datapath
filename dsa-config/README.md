# dsa-config/

Pre-made `accel-config` JSON configuration files for Intel DSA devices.

## Files

| File | Description |
|------|-------------|
| `iax-hw-eval.conf` | IAX queue setup for hw-eval (`iax1/wq1.0` and `iax3/wq3.0`) |
| `1-engine.conf` | 1 engine per work queue |
| `2-engine.conf` | 2 engines per work queue |
| `3-engine.conf` | 3 engines per work queue |
| `4-engine.conf` | 4 engines per work queue |

The naming convention is `N-engine.conf`, where N is the number of engines
assigned to the work queue. More engines allow higher hardware parallelism.

## How to Apply

```bash
accel-config load-config iax-hw-eval.conf
```

This loads IAX queues for `iax1` and `iax3` in dedicated mode with one engine
per work queue for hw-eval testing.

```bash
accel-config load-config 2-engine.conf
```

This configures the DSA device and enables the work queue. The device must be
disabled before loading a new configuration.

## Key Configuration Fields

- **device**: Target DSA device (e.g., `dsa0`)
- **mode**: Work queue mode (`dedicated` for exclusive access)
- **size**: Work queue depth (number of descriptors the WQ can hold)
- **max_batch_size**: Maximum descriptors per hardware batch
- **max_transfer_size**: Maximum bytes per single descriptor transfer

The WQ depth (`size`) is particularly relevant for backpressure handling in
the benchmark -- see `DsaEngine::submit()` in `src/dsa/dsa.hpp`.

See [CLAUDE.md](../CLAUDE.md) for full project documentation.
