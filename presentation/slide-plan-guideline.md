# Slide Plan Guideline

A slide plan is an implementation blueprint, not a text inventory. It must define the story, the exact authored words, where those words go, and how the audience should read each page before Typst work begins.

## Required deck-level sections

1. **Context**
   - Goal of the deck.
   - Audience and assumed background.
   - Source grounding: exact files, reports, commits, measurements, or artifacts the deck is based on.

2. **Story spine**
   - The narrative arc in order.
   - The opening question or tension.
   - The mechanism being taught.
   - The evidence or design result.
   - The final reusable implication.

3. **Visual language**
   - Color meanings.
   - Repeated components.
   - Diagram conventions.
   - Any constraints that must remain consistent across pages.

4. **Slide-by-slide blueprint**
   - One section per slide using the template below.

## Required per-slide template

```md
### Slide N — Title

**Teaching purpose**
What this page teaches in one sentence.

**Audience takeaway**
The exact idea the audience should remember after this page.

**Layout**
Describe where content goes: top/middle/bottom regions, left/right columns, main callout, diagram, code block, and speaker-message strip.

**Exact visible text**
List every authored word grouped by placement. Do not use a flat inventory.

**Visual treatment**
State hierarchy and emphasis: primary vs secondary text, dominant visual object, color meaning, arrows/relationships, and what should feel subordinate.

**Speaker message**
The exact bottom-strip teaching sentence.

**Transition**
How this slide creates the need for the next slide.
```

## Good plan example

```md
### Slide 2 — Sync Encode: Borrow, Write, Return

**Teaching purpose**
Show the original sync contract: encode finishes payload bytes before Tonic framing begins.

**Audience takeaway**
Sync encode is safe because the borrowed buffer is only used during one stack-local completed-buffer call.

**Layout**
Use two columns at the top and one wide flow card below.

Left top card:
- Title: `The API shape`
- Code block:

  ```rust
  fn encode(&mut self, item, dst: &mut EncodeBuf) -> Result<()>
  ```

- Under the code block, place a green thesis callout.

Right top card:
- Title: `Why the completed-buffer contract works`
- Four bullets.

Middle wide card:
- Title: `Completed-before-framing flow`
- Horizontal flow diagram.
- Caption centered below the flow.

Bottom strip:
- Speaker message.

**Exact visible text**
Left top card:
- `The API shape`
- `fn encode(&mut self, item, dst: &mut EncodeBuf) -> Result<()>`
- `The caller sees completed payload bytes before framing begins.`

Right top card:
- `Why the completed-buffer contract works`
- `Encoder borrows the output buffer: dst: &mut EncodeBuf.`
- `CPU writes the payload immediately, before any return.`
- `No encode state survives the call.`
- `gRPC framing can measure and prefix completed bytes.`

Middle wide card:
- `Completed-before-framing flow`
- `message`
- `encode call`
- `borrowed EncodeBuf`
- `completed payload bytes`
- `gRPC frame`
- `Once payload bytes are complete, framing can measure, prefix, and hand off the buffer.`

Bottom strip:
- `Sync encode works because it is a completed-buffer contract: by the time framing runs, payload bytes already exist.`

**Visual treatment**
The green thesis callout is the main sentence. The flow diagram should be wider and more visually dominant than the bullet card, because the next slide breaks this completed-buffer flow. The code block is evidence, not the main visual object.

**Speaker message**
`Sync encode works because it is a completed-buffer contract: by the time framing runs, payload bytes already exist.`

**Transition**
Next slide asks what breaks if encode can suspend before the buffer is complete.
```

## Anti-patterns

- A flat list of strings with no placement.
- A vague slide title plus bullets like “show results” or “explain architecture.”
- Adding visual elements during Typst implementation that were not planned.
- Treating exact text as sufficient without visual hierarchy or transition logic.
- Letting a slide teach multiple unrelated ideas.
