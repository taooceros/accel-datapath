---
name: rust-no-boxed-async-in-unboxed-prost-path
description: "Do not switch Prost async encode APIs to boxed futures in an unboxed codebase"
condition: "Now changed to boxed future:|Message::encode_raw_async[\\s\\S]{0,200}Box<dyn core::future::Future"
scope: "text"
---

Do not introduce `Box`, `Pin<Box<dyn Future>>`, or `Box::pin` to make Prost/Tonic async Rust compile when the surrounding codebase is unboxed. Preserve the zero-allocation style: use `async fn`/RPITIT where accepted, an associated future type, or redesign the trait boundary. If boxing is truly necessary, stop and justify the allocation and dynamic dispatch before changing code.