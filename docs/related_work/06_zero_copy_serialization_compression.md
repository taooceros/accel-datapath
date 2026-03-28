# Zero-copy, serialization, and compression

This topic is the closest bridge from the mature DSA/stdexec results to the future `accel-rpc` agenda. The best papers here are often adjacent rather than exact matches, but they help define where copy, serialization, and compression costs enter the RPC path.

## Strong adjacent papers

- **Cornflakes** — 2023, **SOSP**. Important for zero-copy serialization and scatter-gather-aware RPC design.
- **SerDes-free State Transfer in Serverless Workflows** — 2024, **EuroSys**. Useful evidence that serialization/deserialization itself can be a dominant host-side cost.
- **Achieving Zero-copy Serialization for Datacenter RPC** — 2023, IPCCC. Not top-tier by the repo's preferred venue filter, but directly relevant enough to keep as adjacent support.

## Compression and checksum relevance

For compression and checksum offload, the strongest immediate grounding is still partly implementation-oriented rather than paper-oriented:

- Intel QPL and Intel QATzip for compression paths
- ISA-L for fast CRC/deflate baselines
- gRPC compression guide and tonic compression hooks for wire semantics and integration points

## Why these matter here

`accel-rpc` is not just a transport project. Its planned value is to decompose end-to-end RPC cost and selectively offload the expensive path elements. That makes zero-copy, serialization, and compression prior work directly relevant even when the papers are not accelerator-specific.

## Gap relative to this repo

There is still no strong top-tier cluster combining **gRPC semantics, Rust async integration, and on-die accelerator offload for copy/checksum/compression**. This remains one of the clearest opportunities identified by the repo's research plan.
