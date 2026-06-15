# Hermes: Enhancing Layer-7 Cloud Load Balancers with Userspace-Directed I/O Event Notification

## Paper identity

- **Canonical title**: Hermes: Enhancing Layer-7 Cloud Load Balancers with Userspace-Directed I/O Event Notification
- **Venue / year**: SIGCOMM 2025
- **Authors noted in local artifacts**: Tian Pan et al.
- **Source URL**: https://ng-95.github.io/files/Hermes_SIGCOMM25.pdf
- **Repo pass confidence**: strong for the event-notification mechanism, worker-state-aware dispatch policy, and the main production deployment outcomes used in this repo

## Why it matters here

Hermes is a strong recent paper for the repo's claim that scheduling and event-notification policy can become first-class bottlenecks even when the kernel data path itself is no longer the dominant CPU consumer. It turns userspace worker state into an explicit control input for kernel dispatch instead of treating epoll or reuseport behavior as a fixed substrate.

## Problem

The paper asks how a multicore Layer-7 cloud load balancer should dispatch new connections when existing Linux mechanisms expose an unpleasant tradeoff. `epoll` can avoid some overload cases but suffers from unfair wakeups, while `SO_REUSEPORT` hashes more evenly but has no visibility into whether a userspace worker is already overloaded, hung, or handling disproportionately expensive traffic.

## Method / mechanism

1. **Userspace-directed I/O event notification**: treat userspace worker status as a first-class input to connection dispatch.
2. **Worker Status Table (WST)**: keep per-worker timestamps, pending-event counts, and accumulated connection counts in shared memory.
3. **Worker-triggered distributed scheduling**: run a cascading filter in userspace to choose acceptable workers before new connections arrive.
4. **eBPF-assisted kernel dispatch**: override default `SO_REUSEPORT` socket selection with a userspace-supplied worker set via eBPF maps.

## Evaluation setup

- production deployment in Alibaba Cloud L7 load balancers
- deployed at `O(100K)` CPU cores and `O(10M)` RPS according to the abstract-level summary in the local artifact
- paper evaluation describes a single 32-core / 128 GB Linux 4.19 LB within a larger eight-LB cluster serving about 1,500 tenants for controlled comparisons against `epoll exclusive` and `reuseport`

## Key findings

1. Hermes reduces daily worker hangs by `99.8%` in the reported deployment.
2. Hermes lowers normalized unit infrastructure cost for the L7 LB fleet by up to `18.9%`.
3. The closed-loop design is more robust than either `epoll exclusive` or `reuseport` alone across diverse traffic regimes, especially when workers become busy or hung.
4. The paper argues that userspace-visible metrics such as pending events and accumulated connections are more informative for L7 scheduling than kernel-visible packet counts alone.

## Limits and confidence

- strong source for userspace-guided event notification and worker-aware dispatch policy
- not a direct Tonic, gRPC, or on-die accelerator paper
- grounded in Linux `epoll`, `SO_REUSEPORT`, shared memory, and eBPF rather than a Rust async runtime
- multi-tenant L7 load balancer assumptions do not transfer directly to this repo's accelerator datapath, so the paper is best used as a control-path and scheduling analogue

## Repo takeaways

- treat event notification and completion policy as measurable design variables, not merely fixed kernel behavior
- a hot-path / control-path split can let userspace own the richer scheduling logic while the kernel executes a constrained fast-path decision
- lock-free shared state plus eBPF map synchronization is a useful pattern for low-overhead cross-boundary control

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../003.async_runtime_2026-03-28.md`](../../003.async_runtime_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../related_work/03_async_framework_completion_overhead.md`](../../../../related_work/03_async_framework_completion_overhead.md)
- [`../../../../../current.md`](../../../../../current.md)
