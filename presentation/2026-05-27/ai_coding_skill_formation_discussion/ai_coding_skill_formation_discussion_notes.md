# Presenter notes: The Illusion of Competence

## 1. The Illusion of Competence

- Open with the contrast between a completed artifact and retained skill.
- Do not claim the paper proves that AI is harmful. The point is narrower: a passing AI-assisted task is not evidence that the human learned.
- Ask the audience to remember their most recent AI-assisted coding session.

## 2. The visible win hides the invisible cost

- Let the discussion-starter question sit for a moment.
- Frame this as a measurement problem: organizations can count tickets and drafts, but rarely measure retained skill.
- Avoid guilt framing. The point is that normal incentives push people toward the artifact path.

## 3. Oversight is not a job title. It is a skill.

- Connect this to HAI: as AI writes more code, humans are increasingly positioned as reviewers and supervisors.
- Oversight depends on debugging, code reading, and conceptual understanding.
- Transition: the paper asks whether AI-assisted learning strengthens or bypasses these oversight skills.

## 4. The experiment as a pipeline

- Trio matters because it is unfamiliar but realistic: a new async library a developer might learn on the job.
- The randomized setup matters, but do not get stuck in methods details.
- Stress the final unassisted quiz: this is what separates task completion from retained mastery.

## 5. What exactly did they ask developers to learn?

- The tasks were small but concept-bearing: concurrent timers, nurseries, missing-record handling, channels, and result flow.
- The point is workplace onboarding, not a toy syntax exercise.
- Keep the explanation brief; the next slide explains how mastery was measured.

## 6. How did they measure learning?

- The quiz is not just "do you remember syntax?"
- Debugging is the important one for oversight because AI-generated code can be plausible and wrong.
- Low-level code writing may become less central, but reading, debugging, and conceptual judgment remain central.

## 7. How did they conclude there was a learning penalty?

- Say "17 percentage points" when comparing 50% and 67%.
- The article describes the gap as 17% lower and nearly two letter grades; the slide uses percentage points for clarity.
- The speed result was about two minutes faster for AI on average, but not statistically significant.
- Do not imply every AI user performed poorly; the next section explains interaction-pattern variation.

## 8. The productivity story breaks

- This is the visual headline slide for the main result.
- Keep the interpretation simple: the task artifact was not enough evidence of retained mastery.
- Do not overclaim permanent degradation; the measured outcome was immediate unassisted mastery.

## 9. Debugging is where the gap bites

- The paper reports the largest score gap on debugging questions.
- The proposed mechanism is that no-AI learners encountered and repaired more errors, so they practiced debugging.
- Mark the mechanism as plausible, not proven causal mediation.

## 10. Where did the time savings go?

- AI generation is not the whole workflow.
- Some participants spent substantial time composing and iterating on prompts.
- Use this slide to pivot from "AI or no AI" to "what kind of AI interaction?"

## 11. Not all AI use is the same

- Do not moralize the personas. These are behavior patterns under time pressure.
- The important contrast is whether the user keeps doing prediction, explanation, and repair.
- The qualitative clusters are descriptive; do not present them as causal proof.

## 12. Same assistant. Different practice loop.

- Keep this concrete and quick.
- The traces are intentionally minimal, not realistic transcripts.
- Ask the room which trace feels closer to their own use.

## 13. Which persona was your last AI session?

- This is a short pair-share, about one or two minutes.
- Ask participants to choose a label, not justify themselves.
- If the room is quiet, answer the three diagnostic questions yourself first.

## 14. Generator or coach?

- Move from individual responsibility to interface design.
- Make the tradeoff explicit: friction can harm throughput but help learning.
- Ask which interventions belong in onboarding or unfamiliar-library work, not necessarily in every production task.


## 15. When delegation felt productive but did not teach enough

- Use the Tonic profiling example as the personal version of the artifact-path warning.
- Be concrete: the agent could produce runs, tables, and dashboards, but the useful understanding came from checking whether the comparisons were valid.
- Tie the lesson to method: matched regimes, instrumentation-off throughput, and perf evidence turned the artifact into learning.

## 16. The coroutine bug worked because I steered the debugger

- Frame this as learning-oriented delegation, not solo hero debugging.
- Say that the useful move was giving Claude the suspected boundary: DSA descriptor/completion alignment interacting with coroutine-frame storage.
- Claude helped inspect layout and challenge the naive `alignas()` assumption, but the human kept the hypothesis, evidence standard, and final judgment in the loop.

## 17. One question to end on

- Use this as the single closing discussion: what undergraduate CS education should protect when AI can produce working artifacts.
- Ask the class to choose one practice loop: prediction, debugging, explanation, or design judgment.
- Push for one concrete course policy that preserves the chosen practice loop without pretending students will not use AI.

## 18. Bibliography

- Show briefly or leave as a reference slide.
- The primary grounding is Shen and Tamkin plus the Anthropic research article.
- Storey's cognitive-debt article and Sarkar's intention paper provide the broader framing for shared understanding and intentional programming.
- The reference video can be shared as a pre-watch or follow-up link.