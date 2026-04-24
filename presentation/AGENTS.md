# presentation AGENTS

Inherits `../AGENTS.md`.

## OVERVIEW
Typst slide decks and presentation-only artifacts live here. This subtree is for concise, audience-facing outputs derived from repo-grounded plans, reports, specs, and code.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Current deck | `YYYY-MM-DD/*.typ` | Keep each presentation in a dated directory. |
| Supporting evidence | `../docs/plan/`, `../docs/report/`, `../docs/specs/` | Slides should summarize these sources, not replace them. |
| Typst workflow | `../presentation/<date>/` | Keep deck-local helpers close to the deck unless reused broadly. |

## CONVENTIONS
- Use dated directories: `presentation/YYYY-MM-DD/`.
- Keep one primary `.typ` entry file per deck, named by topic or meeting.
- Prefer plain Typst with lightweight local helpers over introducing a large slide framework unless the deck clearly needs it.
- Keep claims in slides traceable to repo sources; if a finding matters for later reuse, write it in `docs/report/` or `remark/` as well.
- When editing a deck, preserve readability on a 16:9 presentation page and favor concise speaker-facing structure over dense prose.
- Compile the deck after non-trivial changes to catch Typst errors.

## DESIGN PRINCIPLES
- Prefer one main idea per slide. If a slide has multiple independent takeaways, split it.
- Prefer diagrams, pipelines, or charts plus short interpretation over prose-heavy panels.
- Use progressive disclosure for dense technical material: establish the question first, then mechanism, then evidence, then implication.
- Reserve dense tables or detail-heavy quantitative artifacts for the few slides where exact values matter most.
- Make result slides answer one explicit question and foreground the answer visually before caveats or nuance.
- Keep limitations visible but visually subordinate to the main takeaway; caveats should refine the claim, not overwhelm it.
- Use consistent visual roles across the deck so framing, evidence, and caveat elements are easy to distinguish.

### REVIEW CHECKLIST
- [ ] Each slide has one main takeaway and no buried secondary message.
- [ ] The slide's visual plus short text communicates the idea before any dense prose.
- [ ] For complex material, the order is question → mechanism → evidence → implication.
- [ ] Dense tables appear only where exact values are essential and are clearly highlighted as evidence-heavy slides.
- [ ] Results slides state the answer explicitly before caveats, and caveats are clearly secondary.
- [ ] Limitations are visible, but they do not obscure the slide's primary conclusion.
- [ ] Framing, evidence, and caveat elements are visually consistent across the deck.

## SLIDE-PLAN REQUIREMENTS
- Before creating or substantially revising a deck, write or update a plan under `../docs/plan/YYYY-MM-DD/`.
- The slide plan should be detailed enough that someone could implement the deck structure from the plan alone without rereading the whole conversation.
- The plan is not just a topic list; it should capture the intended audience-facing story and the concrete slide components.
- Every element that will appear on a slide should be described in the plan first; do not add deck content that has no plan-side description.

### What a slide plan must include
1. **Goal and audience**
   - what the talk is for (interview, project meeting, advisor update, defense dry run, etc.)
   - target duration
   - desired tone (conversational, persuasive, technical, status-update, etc.)
2. **Source grounding**
   - exact reports / plans / specs / older decks being summarized
   - any claims or numbers that must remain traceable to repo artifacts
3. **Story spine**
   - the high-level narrative arc in order
   - what question the talk opens with, what insight changes the story, and what closing message the audience should remember
4. **Slide-by-slide design**
   - every planned slide should have:
     - working title
     - purpose of the slide
     - key message / takeaway sentence
     - concrete content components to show
     - visual guidance when relevant
5. **Component-level detail**
   - specify which concrete components belong on a slide, such as:
     - title / subtitle / metadata
     - callouts / notes / cards / tables
     - diagrams or pipeline stages
     - explicit system components, APIs, or modules to mention
     - exact benchmark variants or measured results that appear
   - for every planned component, say what it is doing on the slide and what concrete content it contains
6. **Delivery guidance**
   - expected time per section or per slide when useful
   - transitions or phrasing cues if the deck should sound conversational

### Expected level of detail for slide plans
- Do not stop at “slide 3: background” or “slide 5: results”.
- Include which *components* appear on the slide and which *system pieces* they refer to.
- The plan should be detailed enough that a reviewer can map each slide element in the final deck back to a specific line in the plan.
- If something appears in the slide but is not described in the plan, the plan is incomplete.
- For systems talks, explicitly name the important software and hardware components when they matter to the story, for example:
  - APIs or abstractions (`async/await`, callbacks, sender/receiver, futures)
  - framework layers (`scope.nest`, `then`, `connect`, `start`)
  - hardware examples (RDMA MMIO, Intel DSA, Intel IAX)
  - datapath stages (codec, copy/CRC, compression, framing, runtime)
- If one slide contains the central quantitative evidence, say exactly which numbers, variants, or comparisons it should show.

### Preferred slide-plan structure
- `## Goal`
- `## Sources`
- `## Intended output`
- `## Design goals`
- `## Story spine`
- `## Planned slide sequence`
- `## Slide-by-slide content components`
- `## Visual guidance`
- `## Delivery guidance`
- `## Steps`

### Presentation-specific planning guidance
- Prefer one main idea per slide.
- Reserve dense tables or detailed quantitative artifacts for the one or two slides where they matter most.
- Use the plan to decide which material stays in the deck and which material remains only in reports.
- When writing the plan, explicitly list all slide-visible content: labels, callouts, diagrams, tables, result numbers, and caveats that will appear.
- Write the plan progressively as well: complete one section or module in enough detail before moving to the next, rather than leaving multiple shallow placeholder sections.
- If a deck is conversational, say so explicitly in the plan and specify where simple visuals should replace dense text.
- If reusing an older deck, the plan should say what is reused, what is reframed, and what is newly added.

## ANTI-PATTERNS
- Do not store the only copy of important analysis in slides.
- Do not create undated or ambiguously named presentation directories.
- Do not turn presentation files into a general knowledge dump; keep reusable knowledge in tracked docs.
- Do not write slide plans that are only a vague list of topics without slide purposes and components.
- Do not omit the key system components or measurements that the slide is supposed to mention.
- Do not add slide elements during implementation that were never mapped in the plan.
- Do not leave large stretches of the plan as shallow placeholders while continuing to later sections.
