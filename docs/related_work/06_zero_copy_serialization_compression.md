# Zero-copy, serialization, and compression

This topic is the closest bridge from the mature DSA/stdexec results to the future `accel-rpc` agenda. The best papers here are often adjacent rather than exact matches, but they help define where copy, serialization, and compression costs enter the RPC path.

## Strong adjacent papers

- **A Hardware Accelerator for Protocol Buffers** — 2021, **MICRO**. Best direct citation for protobuf serialization/deserialization as a first-class cost center. Paper folder: [`../report/literature/papers/hardware-accelerator-for-protocol-buffers/README.md`](../report/literature/papers/hardware-accelerator-for-protocol-buffers/README.md)
- **Cornflakes** — 2023, **SOSP**. Important for zero-copy serialization and scatter-gather-aware RPC design. Paper folder: [`../report/literature/papers/cornflakes-zero-copy-serialization-for-microsecond-scale-networking/README.md`](../report/literature/papers/cornflakes-zero-copy-serialization-for-microsecond-scale-networking/README.md)
- **SerDes-free State Transfer in Serverless Workflows** — 2024, **EuroSys**. Useful evidence that serialization/deserialization itself can be a dominant host-side cost.
- **Achieving Zero-copy Serialization for Datacenter RPC** — 2023, IPCCC. Not top-tier by the repo's preferred venue filter, but directly relevant enough to keep as adjacent support.

## Direct gRPC-oriented measurement support

- **Designing a Micro-Benchmark Suite to Evaluate gRPC for TensorFlow: Early Experiences** — 2018, BPOE / ASPLOS workshop. Useful because it explicitly separates serialized and non-serialized gRPC modes and varies payload structure. Paper folder: [`../report/literature/papers/designing-a-micro-benchmark-suite-to-evaluate-grpc-for-tensorflow-early-experiences/README.md`](../report/literature/papers/designing-a-micro-benchmark-suite-to-evaluate-grpc-for-tensorflow-early-experiences/README.md)

## Compression and checksum relevance

For compression and checksum offload, the strongest immediate grounding is still partly implementation-oriented rather than paper-oriented:

- Intel QPL and Intel QATzip for compression paths
- ISA-L for fast CRC/deflate baselines
- gRPC compression guide and tonic compression hooks for wire semantics and integration points

## Why these matter here

`accel-rpc` is not just a transport project. Its planned value is to decompose end-to-end RPC cost and selectively offload the expensive path elements. That makes zero-copy, serialization, and compression prior work directly relevant even when the papers are not accelerator-specific. The strongest new additions here are the protobuf-focused MICRO 2021 paper and the gRPC microbenchmark methodology paper, which together justify treating serialization and buffer-shape effects as separate buckets rather than burying them inside generic transport cost. For paper-oriented detail, use the linked paper folders under `docs/report/literature/papers/`.

## Gap relative to this repo

There is still no strong top-tier cluster combining **gRPC semantics, Rust async integration, and on-die accelerator offload for copy/checksum/compression**. This remains one of the clearest opportunities identified by the repo's research plan.
