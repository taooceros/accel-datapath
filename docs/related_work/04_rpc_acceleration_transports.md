# RPC acceleration and transports

This note covers the best top-tier RPC papers to position `accel-rpc`. The literature is thinner for direct gRPC-plus-accelerator designs than for RPC or SmartNIC systems more broadly, so the note separates direct matches from nearby baselines.

## Direct matches

- **Datacenter RPCs can be General and Fast (eRPC)** — 2019, **NSDI**. Best fast-RPC baseline and the clearest reference for a high-performance but still relatively general software RPC stack.
- **RpcNIC: Enabling Efficient Datacenter RPC Offloading on PCIe-attached SmartNICs** — 2025, **HPCA**. Most direct accelerator-assisted RPC paper in the preferred venue set.
- **R2P2: Making RPCs first-class datacenter citizens** — 2019, **USENIX ATC**. Strong paper for treating RPC as the primitive that transport and scheduling should optimize around.
- **Turbo: SmartNIC-enabled Dynamic Load Balancing of µs-scale RPCs** — 2023, **HPCA**. Useful direct SmartNIC comparison point for microsecond-scale RPC services.
- **FaSST: Fast, Scalable and Simple Distributed Transactions with Two-Sided (RDMA) Datagram RPCs** — 2016, **OSDI**. Best RDMA-backed RPC systems reference in the preferred venue set.

## Adjacent but important baselines

- **Shenango** — 2019, **NSDI**. Helps separate runtime wins from true datapath wins.
- **Caladan** — 2020, **OSDI**. Same role, with stronger interference/isolation framing.
- **Rethinking RPC Communication for Microservices-based Applications** — 2025, **HotOS**. Good motivation for why current microservice RPC stacks mismatch modern hardware.

## Why these matter here

`accel-rpc` is trying to answer a question the literature has not fully resolved: can a drop-in, semantics-preserving RPC stack use on-die accelerators for copy, checksum, and compression while still fitting an async runtime model? eRPC and R2P2 give the software RPC baseline, RpcNIC and Turbo give accelerator-aware or SmartNIC-aware comparisons, and FaSST grounds the RDMA transport side.

## Gap relative to this repo

There is still little strong top-tier work on **gRPC or tonic with integrated on-die accelerator offload**. That gap is important enough to state explicitly in any repo-level related-work discussion.
