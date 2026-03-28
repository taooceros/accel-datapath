# Batching and submission regime

This note covers the papers most relevant to the repository's core thesis: batching changes the regime by making submission and completion policy, rather than raw device latency, the dominant cost.

## Direct matches

- **MegaPipe: A New Programming Interface for Scalable Network I/O** — 2012, **OSDI**. The clearest classic support for the idea that API shape, batching, and event aggregation strongly affect scalability.
- **IX: A Protected Dataplane Operating System for High Throughput and Low Latency** — 2014, **OSDI**. Strong support for poll-mode, bounded batching, and direct fast-path control.
- **mTCP: a Highly Scalable User-level TCP Stack for Multicore Systems** — 2014, **NSDI**. Useful evidence that once software stays on the fast path, aggregation and batching policy dominate small-message throughput.
- **Arrakis: The Operating System is the Control Plane** — 2014, **OSDI**. Important for the broader argument that removing mediation from the fast path exposes the remaining software costs.

## Intel- and movement-adjacent support

- **How to Copy Memory? Coordinated Asynchronous Copy as a First-Class OS Service** — 2025, **SOSP**. Very strong adjacent paper for the repo because it promotes copy to a managed async service and makes coordination/completion overhead first-class.
- **DaeMon: Architectural Support for Efficient Data Movement in Disaggregated Systems** — 2022, **SIGMETRICS**. Useful architectural evidence that data movement itself often dominates system design.

## Why these matter here

The repository's batching argument is not just “batching helps throughput.” The stronger claim is that batching can lower hardware cost enough that submission/completion software dominates. MegaPipe, IX, mTCP, and Arrakis support that shape of argument from the network and OS side, while Copier and DaeMon strengthen the movement-specific side.

## Gap relative to this repo

These papers do not directly measure the cost of modern composable async frameworks such as stdexec in the nanosecond regime. They motivate the regime; the repository contributes the layer-removal measurement angle.
