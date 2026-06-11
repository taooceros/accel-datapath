---
name: no-dsa-staging-buffer
description: "Do not implement DSA copies through an intermediate staging buffer"
condition: ["\\bstag(?:e|ed|ing)\\b[\\s\\S]{0,160}\\b(?:DSA|dsa)\\b[\\s\\S]{0,160}\\b(?:destination|buffer|output|BytesMut)\\b", "\\bdst\\s*:\\s*BytesMut\\b", "BytesMut::with_capacity\\(payload_len\\)"]
scope: ["tool:write(*.rs)", "tool:edit(*.rs)"]
---

Do not add a staging buffer for DSA byte copies. The point of the DSA path is to write into the final encode buffer. Assume the encode buffer remains stable across `Poll::Pending`; keep the buffer storage alive and commit/advance it after completion instead of copying from a temporary `BytesMut`.