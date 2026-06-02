---
name: no-hotpath-trace-vec-allocation
description: "Do not allocate trace vectors inside submit-occupancy measurement paths"
condition: "let\\s+mut\\s+iteration_trace\\s*=\\s*Vec::new\\s*\\(\\)"
scope: "tool:read(*experiment_1_submit_occupancy.rs)"
---

No allocation or dynamic trace-vector creation in benchmark measurement paths. If you see `let mut iteration_trace = Vec::new()` inside the submit-occupancy per-iteration loop, stop and refactor before proceeding: pre-allocate trace/result storage before the loop and push/write directly into reserved storage outside the timed submit path.