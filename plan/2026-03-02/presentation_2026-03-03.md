# Plan: Meeting Presentation 2026-03-03

**Date**: 2026-03-02
**Goal**: One-file Typst slide deck for tomorrow's advisor meeting.

## Meeting agenda
1. Brief intro — undergraduate delegation's dialogue project
2. Concurrency strategy landscape in the DSA benchmark suite
3. Auto-batching as the central thesis: transparent, framework-layer batching

## Slide outline

| # | Title | Key content |
|---|-------|-------------|
| 0 | Title | "stdexec + DSA: Progress Update · Mar 3, 2026" |
| 1 | Agenda | Three-item agenda for today's meeting |
| 2 | Undergrad project: Delegation's Dialogue | Brief — dialogue-driven delegation model |
| 3 | What We Built | Benchmark suite overview, 9 strategies × 5 submitters |
| 4 | Concurrency Strategy Landscape | Three families (sliding window, batch, scoped workers) + two polling modes |
| 5 | The Cost of Composability | Layer-removal result: noalloc→direct→reusable, measured ns/op deltas |
| 6 | The MMIO Doorbell Problem | Why unbatched submission caps at ~6 Mpps |
| 7 | Auto-Batching: Transparent by Design | Scheduling layer is oblivious; same code, 3–6× higher throughput |
| 8 | How the MirroredRing Works | memfd + dual mmap, eliminates wrap-around |
| 9 | Results | 1.2–2× speedup across all 8 DSA ops; table from batching.typ |
| 10 | The General Principle | Batching shifts bottleneck from HW to SW — not DSA-specific |
| 11 | What's Next | Indexed queue, bistable regime, multi-device |

## Style
- Match `presentation/2026-02-23/progress_2026-02-23.typ` exactly (font, margins, slide-title helper).
- No external package dependencies (avoid cetz for simplicity in this talk).
- Plain tables for data; inline code with backtick blocks where needed.
