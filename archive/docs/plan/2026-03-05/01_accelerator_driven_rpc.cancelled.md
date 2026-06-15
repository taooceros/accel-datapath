# Accelerator-Driven gRPC: Architecture & Experiment Plan

**Date**: 2026-03-05
**Platform**: Tonic (Rust gRPC) + Intel DSA + Intel IAX
**Goal**: Offload the gRPC data path to hardware accelerators, maximizing message throughput and freeing CPU for irreducible computation (serialization/deserialization).
**Status**: cancelled on 2026-04-15


Historical note:
Relationship: Preserved as a historical planning record; future work should start from a fresh plan instead of reviving this stale in-progress file.

---

## 1. Hardware Inventory

### Machine: Saturn (2-socket Sapphire Rapids)

| Resource | Spec | NUMA 0 | NUMA 1 |
|----------|------|--------|--------|
| **CPU** | Xeon Gold 6438M, 32c/64t per socket | cores 0-31, 64-95 | cores 32-63, 96-127 |
| **DSA** | 4 engines, 8 WQs, 128 WQ depth each | **dsa0** (wq0.0 dedicated, active) | **dsa2** (present, unconfigured) |
| **IAX** | 8 engines, 8 WQs, 128 WQ depth each | **iax1** (enabled, **WQs unconfigured**) | **iax3** (enabled, **WQs unconfigured**) |
| **Network** | Intel I350 1GbE (igb driver) | enp23s0f0 | — |
| **DLB** | Not present | — | — |
| **Kernel** | 6.17.7, PREEMPT_DYNAMIC | — | — |

### Accelerator Capabilities

**DSA** — Data movement and integrity:
- `data_move` (memcpy), `copy_crc` (fused copy + CRC-32C), `crc_gen`, `compare`, `compare_value`, `dualcast`, `mem_fill`, `cache_flush`
- Max transfer: 2 MiB, max batch: 1024 descriptors
- Submission: `movdir64b` (dedicated WQ) or `enqcmd` (shared WQ)
- Completion: inline polling on completion record (no interrupts)

**IAX** — Compression and analytics:
- Deflate compress/decompress (RFC 1951 — the core of gzip/zlib)
- CRC-32 (different polynomial from DSA's CRC-32C)
- Scan, select, extract (analytics filters — less relevant for RPC)
- Same submission/completion model as DSA (ENQCMD + polling)

**Not available**: DLB (hardware load balancing), RDMA/RoCE NIC, SmartNIC/DPU.

### Configuration TODO

IAX hardware is present but needs work queue setup before use:
```bash
# Example: configure iax1 with a dedicated WQ
accel-config config-wq iax1/wq1.0 --mode dedicated --type user --name rpc_compress \
    --priority 10 --size 128 --group-id 0
accel-config config-engine iax1/engine1.0 --group-id 0
accel-config enable-device iax1
accel-config enable-wq iax1/wq1.0
```

---

## 2. Tonic Architecture

### Crate Layering

```
tonic-build              codegen (.proto → Rust stubs)
  └─▸ tonic              core: Codec, Request/Response, Status, metadata, streaming
        └─▸ tonic-transport   Channel (client) / Server, TLS, HTTP/2, connection mgmt
              └─▸ hyper       HTTP/2 implementation
                    └─▸ h2    HTTP/2 protocol state machine
                          └─▸ tokio   async runtime, epoll reactor, TCP
```

### Key Traits and Injection Points

| Trait / Abstraction | Location | What It Controls | Extensibility |
|---------------------|----------|------------------|---------------|
| `Codec` | `tonic/src/codec/` | Serialization ↔ bytes conversion | **Public trait** — implement your own, no fork |
| `Encoder` / `Decoder` | `tonic/src/codec/` | Streaming encode/decode with `Buf` | Part of `Codec` trait |
| `bytes::Buf` / `BufMut` | `bytes` crate | Buffer abstraction (read/write views) | **Trait-based** — custom buffer types possible |
| `CompressionEncoding` | `tonic/src/codec/compression.rs` | gzip/zstd compression | Enum-based; extending requires fork or middleware |
| Tower `Layer` / `Service` | `tower` crate | Request/response middleware pipeline | **Fully composable** — add layers without forking |
| `Connected` | `tonic-transport` | Transport trait for custom connections | Can implement custom transport |

### Data Path: Send Side (Detailed)

```
1. User creates T: prost::Message
2. Codec::encoder().encode(T, &mut BytesMut)
   └── prost::Message::encode(&self, &mut BytesMut)     // serialize
3. encode_client()/encode_server():
   ├── if compression: compress(&mut BytesMut, encoding)  // gzip/zstd
   ├── prepend: [1 byte compress flag][4 byte length]     // gRPC frame
   └── yield EncodedBytes as http_body::Frame
4. Tower service layers process the request/response
5. hyper encodes HTTP/2 HEADERS + DATA frames
6. h2 applies flow control, writes to TLS stream
7. tokio writes to TCP socket
```

### Data Path: Receive Side (Detailed)

```
1. tokio reads from TCP socket
2. TLS decrypt (rustls or native-tls)
3. h2 decodes HTTP/2 frames
4. hyper yields body chunks
5. Streaming::decode():
   ├── accumulate bytes until gRPC frame complete
   ├── read [compress flag][length]
   ├── if compressed: decompress(BytesMut, encoding)
   └── Codec::decoder().decode(&mut BytesMut) → T
6. User receives T: prost::Message
```

---

## 3. The Accelerator-Driven Pipeline

### Full Pipeline with Offload Points

```
                          ┌──────────┐
                          │   APP    │  Rust struct (T: prost::Message)
                          └────┬─────┘
                               │
  ┌────────────────────────────▼────────────────────────────┐
  │  STAGE 1: Serialization                    [CPU only]   │
  │  prost::Message::encode() → bytes in Buffer A           │
  │                                                         │
  │  Irreducible: data-dependent branching, varint encoding │
  │  Cannot be offloaded to DSA or IAX                      │
  └────────────────────────────┬────────────────────────────┘
                               │ serialized bytes in Buffer A (pool-allocated)
  ┌────────────────────────────▼────────────────────────────┐
  │  STAGE 2: Compression                      [IAX]  ★★★  │
  │  IAX deflate compress(A → B)                            │
  │                                                         │
  │  Typically the #1 CPU consumer in compressed gRPC.      │
  │  IAX does RFC 1951 deflate in hardware.                 │
  │  Fallback: software gzip on CPU.                        │
  └────────────────────────────┬────────────────────────────┘
                               │ compressed bytes in Buffer B
  ┌────────────────────────────▼────────────────────────────┐
  │  STAGE 3: Copy + Integrity                 [DSA]  ★★   │
  │  DSA copy_crc(B → C, seed) → CRC-32C                   │
  │                                                         │
  │  Single hardware op: copies to transport buffer AND     │
  │  computes CRC-32C. Integrity check is "free".           │
  │  CRC can be sent as gRPC trailing metadata.             │
  └────────────────────────────┬────────────────────────────┘
                               │ transport-ready bytes in Buffer C + CRC value
  ┌────────────────────────────▼────────────────────────────┐
  │  STAGE 4: Framing                          [CPU]        │
  │  gRPC: prepend [compress_flag | length] (5 bytes)       │
  │  HTTP/2: HEADERS + DATA frames, HPACK                   │
  │  Trivial CPU cost (~ns). HPACK is stateful.             │
  └────────────────────────────┬────────────────────────────┘
                               │
  ┌────────────────────────────▼────────────────────────────┐
  │  STAGE 5: Encryption                       [CPU/AES-NI] │
  │  TLS 1.3 AES-256-GCM via AES-NI instructions           │
  │  CPU has hardware AES; not worth offloading separately  │
  └────────────────────────────┬────────────────────────────┘
                               │
  ┌────────────────────────────▼────────────────────────────┐
  │  STAGE 6: Network I/O                      [Kernel]     │
  │  TCP send via kernel socket (1GbE)                      │
  │  Bottleneck at ~125 MB/s; io_uring for async submit     │
  └────────────────────────────────────────────────────────────┘

  ═══════════════════ RECEIVE PATH (reverse) ══════════════════

  ┌─────────────────────────────────────────────────────────┐
  │  Kernel TCP recv → TLS decrypt → HTTP/2 deframe         │
  │  → gRPC deframe (strip 5-byte header)                   │
  └────────────────────────────┬────────────────────────────┘
                               │ compressed payload in wire buffer
  ┌────────────────────────────▼────────────────────────────┐
  │  STAGE R1: Verify + Copy                   [DSA]        │
  │  DSA copy_crc(wire_buf → D) → verify against sent CRC  │
  └────────────────────────────┬────────────────────────────┘
                               │
  ┌────────────────────────────▼────────────────────────────┐
  │  STAGE R2: Decompression                   [IAX]  ★★★  │
  │  IAX deflate decompress(D → E)                          │
  └────────────────────────────┬────────────────────────────┘
                               │ decompressed serialized bytes
  ┌────────────────────────────▼────────────────────────────┐
  │  STAGE R3: Deserialization                 [CPU only]   │
  │  prost::Message::decode() → Rust struct                 │
  └────────────────────────────┬────────────────────────────┘
                               │
                          ┌────▼─────┐
                          │   APP    │
                          └──────────┘
```

### Pipeline Parallelism: The Core Insight

The real throughput win is not just offloading individual stages — it's overlapping CPU and accelerator work across messages:

```
Time ──────────────────────────────────────────────────────────────►

CPU:   │ser(0)│ser(1)│ser(2)│ser(3)│ser(4)│ser(5)│  ← always serializing next msg
IAX:   │      │cmp(0)│cmp(1)│cmp(2)│cmp(3)│cmp(4)│  ← compressing prev msg
DSA:   │      │      │cpy(0)│cpy(1)│cpy(2)│cpy(3)│  ← copy+CRC two msgs back

Steady state: 3 messages in flight, throughput = max(ser, compress, copy) ≈ ser time
              CPU never blocks on compression or copy — they happen in background
```

This maps directly to the **sliding window** pattern from the dsa-stdexec benchmark suite (`benchmark/dsa/strategies/sliding_window/`), where maintaining K operations in-flight maximizes hardware utilization.

### What Each Accelerator Buys

| Accelerator | Operation | gRPC Stage | CPU Savings | Throughput Impact |
|-------------|-----------|------------|-------------|-------------------|
| **IAX** | deflate compress | Send compression | **High** — compression is CPU-dominant | Removes #1 bottleneck for compressed gRPC |
| **IAX** | deflate decompress | Recv decompression | **High** | Same |
| **DSA** | copy_crc | Buffer transfer + integrity | **Medium** — memcpy + CRC fused | "Free" integrity; frees CPU for ser/deser |
| **DSA** | data_move | Buffer management | **Low-Medium** | Frees CPU cache for hot data |
| **DSA** | dualcast | Response + audit log | **Low** | Enables zero-cost observability |
| **DSA** | mem_fill | Buffer zeroing | **Low** | Security: clear sensitive data between RPCs |
| **DSA** | compare | Cache validation | **Niche** | Skip deser if payload unchanged |
| **DSA** | batch | Multi-op submission | **Amortization** | 1 MMIO doorbell for N operations |

### What Stays on CPU (Irreducible)

| Stage | Why It Can't Be Offloaded |
|-------|---------------------------|
| Protobuf serialization | Data-dependent branching, varint encoding, field traversal |
| Protobuf deserialization | Same — structured data transformation |
| HPACK (HTTP/2 headers) | Stateful compression with dynamic table |
| gRPC framing | 5 bytes — too trivial to offload |
| TLS (AES-GCM) | AES-NI already hardware-accelerated on CPU |
| Flow control / protocol state | Complex state machines, must be CPU |

---

## 4. Experiment Design

### Phase 0: IAX Configuration & Baseline (Days 1-3)

**Configure IAX work queues** — hardware is present but idle:
1. Set up `iax1/wq1.0` (NUMA 0) as dedicated WQ for compression experiments
2. Verify with `accel-config list`
3. Test with QPL (Query Processing Library) or raw IAX descriptors

**Establish Tonic baseline**:
1. Build and run Tonic helloworld example
2. Set up a benchmark harness: unary RPC, varying message sizes (64B → 1MB)
3. Profile with `perf record` / flamegraph
4. Measure: messages/sec, p50/p99 latency, CPU utilization breakdown

**Deliverable**: Baseline numbers + flamegraph showing CPU time distribution across pipeline stages.

### Phase 1: Buffer Infrastructure (Days 4-8)

Replace Tonic's default allocation with accelerator-friendly buffers.

**Buffer Pool Design**:
```rust
/// Pre-allocated, pre-faulted buffer pool for zero-allocation RPC
struct BufferPool {
    slabs: Vec<Slab>,           // large contiguous allocations (huge-page backed)
    free_list: Mutex<Vec<BufHandle>>,  // available buffer slots
}

/// A buffer handle that implements bytes::Buf and bytes::BufMut
struct PoolBuf {
    ptr: *mut u8,               // pointer into slab
    len: usize,
    capacity: usize,
    pool: Arc<BufferPool>,      // return to pool on drop
}

unsafe impl bytes::Buf for PoolBuf { /* ... */ }
unsafe impl bytes::BufMut for PoolBuf { /* ... */ }
```

Buffer requirements:
- **Pre-faulted**: avoid DSA page fault retry path
- **Huge-page backed**: reduce TLB pressure for large messages
- **Pool-managed**: O(1) acquire/release, no malloc in hot path
- **Aligned**: 64-byte alignment for DSA descriptor compatibility (data buffers are flexible)

**Custom Codec**:
```rust
struct AccelCodec<T> {
    pool: Arc<BufferPool>,
    _phantom: PhantomData<T>,
}

impl<T: prost::Message + Default> Codec for AccelCodec<T> {
    type Encode = T;
    type Decode = T;
    type Encoder = AccelEncoder<T>;
    type Decoder = AccelDecoder<T>;

    fn encoder(&mut self) -> Self::Encoder {
        AccelEncoder { pool: self.pool.clone(), _phantom: PhantomData }
    }
    fn decoder(&mut self) -> Self::Decoder {
        AccelDecoder { pool: self.pool.clone(), _phantom: PhantomData }
    }
}
```

**Deliverable**: Custom codec with pooled buffers, benchmarked against stock ProstCodec. Expect: reduced allocation jitter, similar throughput.

### Phase 2: DSA Integration via FFI (Days 9-15)

Bridge Rust to DSA hardware. Two options:

**Option A: Thin C FFI to existing codebase**
```
Rust (tonic) ──FFI──▸ libdsa_ffi.so (C ABI) ──▸ DSA hardware
                       Wraps our C++ DsaEngine
```

```c
// dsa_ffi.h — C ABI for Rust FFI
typedef struct dsa_context dsa_context_t;

dsa_context_t* dsa_init(const char* wq_path);
int dsa_submit_copy_crc(dsa_context_t* ctx, void* dst, const void* src,
                        size_t len, uint32_t seed, uint64_t* completion_id);
int dsa_poll(dsa_context_t* ctx, uint64_t completion_id, uint32_t* crc_out);
void dsa_destroy(dsa_context_t* ctx);
```

**Option B: Native Rust DSA via raw ENQCMD/MOVDIR64B**
```rust
// Direct hardware access — no FFI overhead
unsafe fn submit_descriptor(wq_portal: *mut u8, desc: &DsaDescriptor) {
    // movdir64b instruction via inline asm
    core::arch::asm!(
        "movdir64b {desc}, [{portal}]",
        desc = in(reg) desc as *const _,
        portal = in(reg) wq_portal,
    );
}
```

**Recommendation**: Start with **Option A** (FFI) for rapid iteration since our C++ DSA code handles page fault retry, WQ backpressure, and alignment. Move to Option B later if FFI overhead matters (unlikely — DSA ops are ~100ns+, FFI is ~5ns).

**Rust Future wrapping DSA**:
```rust
struct DsaFuture {
    ctx: Arc<DsaContext>,
    completion_id: u64,
}

impl Future for DsaFuture {
    type Output = Result<u32, DsaError>;  // CRC result or error

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.ctx.poll(self.completion_id) {
            Some(crc) => Poll::Ready(Ok(crc)),
            None => {
                // Register waker for re-polling
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}
```

**Async integration options** (from simple to sophisticated):

| Approach | How | Latency | CPU Efficiency |
|----------|-----|---------|----------------|
| `spawn_blocking` | Offload submit+poll to blocking thread pool | ~μs (thread switch) | Poor — wastes a thread spinning |
| Busy-poll future | `DsaFuture` re-polls every time tokio wakes it | ~100ns (completion) | Medium — waker churn |
| Custom tokio driver | Hook into tokio reactor to poll DSA alongside epoll | Optimal | Best — single poll loop |
| Dedicated DSA thread | Thread runs PollingRunLoop, sends results via channel | ~μs (channel) | Good — mirrors our C++ model |

**Recommendation**: Start with **busy-poll future** (simplest), measure overhead, move to dedicated DSA thread if polling churn is excessive.

**Deliverable**: `dsa-ffi` crate with working copy_crc, memcpy, and poll operations. Benchmarked raw DSA throughput from Rust.

### Phase 3: CRC Middleware (Days 16-20)

Tower middleware layer that adds CRC-32C integrity to every RPC:

```rust
/// Tower layer that computes CRC-32C on outgoing response bodies
struct DsaCrcLayer {
    dsa: Arc<DsaContext>,
}

impl<S> tower::Layer<S> for DsaCrcLayer {
    type Service = DsaCrcService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DsaCrcService { inner, dsa: self.dsa.clone() }
    }
}

impl<S, ReqBody> tower::Service<http::Request<ReqBody>> for DsaCrcService<S>
where
    S: tower::Service<http::Request<ReqBody>, Response = http::Response<BoxBody>>,
{
    // After inner service produces response:
    // 1. DSA copy_crc(response_body → transport_buffer)
    // 2. Attach CRC as gRPC trailing metadata: "x-crc32c: <value>"
    // 3. Receiver verifies CRC after decompression
}
```

Three backends for comparison:
1. **No CRC** (baseline)
2. **Software CRC** (`crc32c` crate, uses SSE4.2 `crc32` instruction)
3. **DSA CRC** (`dsa_copy_crc` — fused copy + CRC in hardware)

**Key question**: For small messages (<4KB), software CRC via SSE4.2 may be faster than DSA submission + polling overhead. The crossover point is important data.

**Deliverable**: CRC middleware with all three backends. Latency vs message size comparison.

### Phase 4: IAX Compression (Days 21-28)

Replace Tonic's software gzip with IAX hardware deflate.

**IAX FFI** (extend the C FFI layer):
```c
typedef struct iax_context iax_context_t;

iax_context_t* iax_init(const char* wq_path);
int iax_submit_compress(iax_context_t* ctx, void* dst, size_t dst_cap,
                        const void* src, size_t src_len, uint64_t* completion_id);
int iax_submit_decompress(iax_context_t* ctx, void* dst, size_t dst_cap,
                          const void* src, size_t src_len, uint64_t* completion_id);
int iax_poll(iax_context_t* ctx, uint64_t completion_id,
             size_t* output_len);  // actual compressed/decompressed size
void iax_destroy(iax_context_t* ctx);
```

**Integration into Tonic**: Two approaches:

**Approach A: Replace compression in Codec** (modify Tonic fork)
```rust
// In tonic/src/codec/compression.rs
fn compress(buf: &mut BytesMut, encoding: CompressionEncoding) -> Result<(), Status> {
    match encoding {
        CompressionEncoding::Gzip => software_gzip_compress(buf),
        CompressionEncoding::IaxDeflate => iax_hardware_compress(buf),  // new
    }
}
```

**Approach B: Compression as Tower middleware** (no fork needed)
```rust
struct IaxCompressionLayer { iax: Arc<IaxContext> }

// Intercepts outgoing body, compresses via IAX before reaching Tonic's transport
```

**Recommendation**: Approach A is cleaner since compression is tightly coupled with gRPC framing (the compress flag byte). This is where our Tonic fork pays off.

**Deliverable**: IAX-accelerated compression in the gRPC path. Benchmark: throughput and CPU utilization vs software gzip, across message sizes.

### Phase 5: Full Pipeline Integration (Days 29-35)

Combine all pieces into the complete accelerator-driven pipeline:

```
              ┌────────────────────────────────────────────────────┐
              │              Accelerator-Driven Tonic Server        │
              │                                                    │
  Request ──▸ │  [DSA copy_crc] ──▸ [IAX decompress] ──▸ [CPU deser] ──▸ Handler
              │                                                    │
  Response ◂──│  [CPU ser] ──▸ [IAX compress] ──▸ [DSA copy_crc]  │ ◂── Handler
              │                                                    │
              │  Buffer Pool: pre-faulted, huge-page, NUMA-local   │
              │  Per-thread: 1 DSA engine + 1 IAX engine           │
              └────────────────────────────────────────────────────┘
```

**Sliding window across accelerators**:
```rust
/// Per-worker-thread accelerator context
struct AccelWorker {
    dsa: DsaContext,        // dedicated WQ on local NUMA node
    iax: IaxContext,        // dedicated WQ on local NUMA node
    pool: BufferPool,       // NUMA-local buffer pool
    inflight: VecDeque<PipelineSlot>,  // sliding window of in-flight operations
}

struct PipelineSlot {
    stage: PipelineStage,
    buf_a: PoolBuf,         // serialization output / decompression output
    buf_b: PoolBuf,         // compression output / transport copy
    iax_completion: Option<u64>,
    dsa_completion: Option<u64>,
    crc: Option<u32>,
}

enum PipelineStage {
    Serializing,            // CPU working on prost::encode
    Compressing,            // IAX compress submitted, awaiting completion
    CopyingWithCrc,         // DSA copy_crc submitted, awaiting completion
    Ready,                  // all stages complete, ready for transport
}
```

**NUMA-aware resource binding**:
```
NUMA 0: tokio workers 0-N  →  dsa0/wq0.x + iax1/wq1.x + local buffer pool
NUMA 1: tokio workers N-M  →  dsa2/wq2.x + iax3/wq3.x + local buffer pool
```

**Deliverable**: End-to-end accelerator-driven gRPC server with full pipeline.

### Phase 6: Benchmarks & Analysis (Days 36-42)

**Workloads**:
- Unary RPC: request-response, varying message sizes (64B, 256B, 1KB, 4KB, 16KB, 64KB, 256KB, 1MB)
- Server streaming: continuous response stream
- Bidirectional streaming: echo server
- Batch: N requests pipelined on one HTTP/2 connection

**Configurations to compare**:

| Config | Codec | Compression | Copy | CRC |
|--------|-------|-------------|------|-----|
| **Baseline** | ProstCodec | Software gzip | implicit (BytesMut) | None |
| **Pool only** | AccelCodec (pool bufs) | Software gzip | implicit (pool) | None |
| **DSA copy+CRC** | AccelCodec | Software gzip | DSA data_move | DSA copy_crc |
| **IAX compress** | AccelCodec | IAX deflate | implicit | None |
| **Full accel** | AccelCodec | IAX deflate | DSA copy_crc | DSA copy_crc |
| **No compression** | AccelCodec | None | DSA copy_crc | DSA copy_crc |

**Metrics**:
- Throughput: messages/sec, MB/sec
- Latency: p50, p99, p99.9
- CPU utilization: per-core, user vs system, instructions retired
- Accelerator utilization: DSA/IAX engine busy time
- Cache behavior: LLC miss rate (accelerators bypass cache)
- Crossover points: message size where hardware offload beats software

**Deliverable**: `docs/report/accelerator_rpc_results.md` with data, flamegraphs, and analysis.

---

## 5. Design Decisions

### Why Tonic over gRPC C++

| Factor | Tonic | gRPC C++ |
|--------|-------|----------|
| Codec extensibility | Public `Codec` trait | Internal, requires deep fork |
| Buffer abstraction | `Buf`/`BufMut` traits | `SliceBuffer`, limited |
| Middleware | Tower layers, composable | Interceptors (limited) |
| Codebase | ~15K lines | ~2M+ lines |
| Build | Cargo | Bazel/CMake |
| Compression hookpoint | `compression.rs`, ~200 lines | Spread across core lib |
| Time to first experiment | Days | Weeks |

### FFI vs Native Rust DSA/IAX

Start with FFI, native later:
- Our C++ DSA code handles page fault retry, WQ backpressure, batch submission
- FFI overhead (~5ns) is negligible vs hardware op latency (~100ns+)
- Lets us start experimenting in days, not weeks
- Native Rust DSA/IAX becomes follow-up if results are promising

### Async Bridging Strategy

| Phase | Strategy | Rationale |
|-------|----------|-----------|
| Initial | Busy-poll `Future` | Simplest, good enough for prototyping |
| If overhead high | Dedicated DSA/IAX poll thread + channel | Mirrors our C++ PollingRunLoop |
| Optimal (later) | Custom tokio runtime driver | Polls DSA/IAX completions in reactor loop |

### Buffer Ownership

```
BufferPool (NUMA-local, huge-page, pre-faulted)
  │
  ├──▸ Codec::encode() borrows buf, serializes into it     (Stage 1, CPU)
  │
  ├──▸ IAX compress reads buf A, writes to buf B            (Stage 2, IAX)
  │
  ├──▸ DSA copy_crc reads buf B, writes to buf C + CRC     (Stage 3, DSA)
  │
  ├──▸ hyper/h2 reads buf C for HTTP/2 DATA frame          (Stage 4, CPU)
  │
  └──▸ bufs A, B returned to pool; C returned after send   (recycle)
```

### CRC-32C as Application-Level Integrity

gRPC relies on TLS for integrity. We add CRC-32C as an **application-level** check:
- Transmitted as gRPC trailing metadata: `x-crc32c: <base64>`
- Computed "for free" via DSA `copy_crc` (piggybacks on the buffer copy)
- Catches: memory corruption, accelerator errors, software bugs below TLS
- Cost: effectively zero (fused with a copy that was happening anyway)

---

## 6. Open Questions & Future Directions

### Near-term Questions

1. **IAX vs software compression crossover**: At what message size does IAX outperform software deflate? Expect: IAX wins for >1KB, software wins for <256B.

2. **DSA vs SSE4.2 CRC crossover**: SSE4.2 `crc32` instruction is fast for small data. At what size does DSA `copy_crc` (which also copies) beat `memcpy + sse4.2_crc`?

3. **tokio integration overhead**: How much overhead does the Future-based DSA/IAX polling add vs our C++ PollingRunLoop? Is a custom tokio driver worth building?

4. **Multiple WQ strategy**: Should each tokio worker thread get a dedicated WQ, or share WQs? Dedicated avoids contention but limits total WQ count.

### Future Directions

1. **Shared-memory transport**: For same-machine RPC, bypass HTTP/2 + TCP entirely. Use DSA `copy_crc` to transfer messages through shared memory. This would be an alternative `tonic-transport` backend.

2. **FlatBuffers + DSA**: FlatBuffers are zero-copy (no serialization step). Pairing FlatBuffers with DSA `copy_crc` eliminates Stage 1 entirely — the "serialized" buffer IS the application struct. Reduces pipeline to: `[app struct] → [DSA copy_crc] → [IAX compress] → [transport]`.

3. **Batch RPC with hardware batch descriptors**: For streaming RPCs, accumulate N messages and submit all DSA/IAX operations as a single hardware batch (1 MMIO doorbell for N ops). Amortization from the benchmark suite shows significant per-op overhead reduction.

4. **Multi-device scheduling**: Use dsa0 (NUMA 0) + dsa2 (NUMA 1) simultaneously. Route RPCs to the NUMA-local accelerator based on which tokio worker thread handles them.

5. **io_uring for network I/O**: Replace tokio's epoll-based reactor with io_uring for the network path. Combined with DSA/IAX for the data path, this would make the entire pipeline submission-based.

6. **Programmable accelerators**: DSA/IAX are fixed-function. For true serialization offload, a programmable accelerator (FPGA, GPU, or future Intel IPU) could handle protobuf encode/decode.

---

## 7. Repository Structure

```
tonic/                              # Git submodule (our fork)
experiments/
  accel-rpc/
    Cargo.toml                      # Workspace root
    proto/
      benchmark.proto               # Test message definitions
    accel-codec/
      src/lib.rs                    # AccelCodec, AccelEncoder, AccelDecoder
      src/buffer_pool.rs            # Pre-faulted, huge-page buffer pool
    accel-middleware/
      src/lib.rs                    # Tower CRC middleware
      src/crc_layer.rs              # DsaCrcLayer / DsaCrcService
      src/compression_layer.rs      # IaxCompressionLayer (if using middleware approach)
    dsa-ffi/
      src/lib.rs                    # Rust bindings to DSA hardware
      src/future.rs                 # DsaFuture — async wrapper
      build.rs                      # Link to libdsa_ffi.so
      cbindgen.toml                 # C header generation
    iax-ffi/
      src/lib.rs                    # Rust bindings to IAX hardware
      src/future.rs                 # IaxFuture — async wrapper
    accel-bench/
      src/main.rs                   # Benchmark harness
      benches/
        throughput.rs               # Criterion: messages/sec vs msg size
        latency.rs                  # Criterion: p50/p99 latency
        cpu_utilization.rs          # perf-stat integration
    examples/
      baseline_server.rs            # Stock Tonic (control)
      accel_server.rs               # Full accelerator pipeline
      baseline_client.rs            # Load generator
src/dsa/                            # Existing C++ DSA code
  ffi/                              # NEW: C wrapper for Rust FFI
    dsa_ffi.h
    dsa_ffi.cpp
    iax_ffi.h
    iax_ffi.cpp
```

---

## 8. Timeline Summary

| Days | Phase | Key Output |
|------|-------|------------|
| 1-3 | **Phase 0**: IAX config + Tonic baseline | Baseline numbers, flamegraph, IAX WQs configured |
| 4-8 | **Phase 1**: Buffer infrastructure | AccelCodec with pooled buffers, benchmarked |
| 9-15 | **Phase 2**: DSA FFI integration | `dsa-ffi` crate, DsaFuture, raw DSA from Rust |
| 16-20 | **Phase 3**: CRC middleware | Tower CRC layer, software vs DSA comparison |
| 21-28 | **Phase 4**: IAX compression | IAX deflate in gRPC path, vs software gzip |
| 29-35 | **Phase 5**: Full pipeline | End-to-end accelerator-driven server |
| 36-42 | **Phase 6**: Benchmarks & report | Comprehensive comparison, `docs/report/accelerator_rpc_results.md` |
