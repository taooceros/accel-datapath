---
name: slides-no-unnecessary-cards
description: "Avoid decorative card containers in Typst slides when a simple graph and text will do."
condition: "#let\\s+path-card\\s*\\("
scope: "tool:write(*.typ)"
---

Do not introduce reusable card helpers or decorative card containers unless the slide story needs semantic grouping. For presentation revisions, prefer the simplest structure: Mermaid graph, direct labels, and concise explanatory text. Cards/blocks are allowed only when they clarify a real conceptual boundary.