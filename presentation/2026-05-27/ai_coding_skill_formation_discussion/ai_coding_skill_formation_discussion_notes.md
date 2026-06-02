Here is the revised presentation script. It integrates the exact terminology, empirical findings, and data from the paper *“How AI Impacts Skill Formation”* by Shen & Tamkin (2026) to ground the script perfectly with your slides.

---

# Presentation Script: The Illusion of Competence

## Slide 1: Title Slide (CSE 599H Human-AI Interaction)

**Visual:** Slide showing the two paths: AI path vs. Human path.

**Script:**
"Welcome, everyone, to CSE 599H. Today, we are diving into a crucial dynamic in human-AI interaction: **The Illusion of Competence**. We are discussing the paper *'How AI Impacts Skill Formation'* by Shen & Tamkin (2026), originally released through the Anthropic Fellows program.

Look at these two paths on the slide. The **AI path** is a smooth transaction: ask the AI, get the code, the task passes. The **Human path** is a rigorous cognitive cycle: predict, try, fail, debug, explain, and *retain*. As we review today's data, ask yourself: Which path did you choose the last time you used an LLM assistant?"

---

## Slide 2: Hook (01 HOOK)

**Visual:** "The visible win hides the invisible cost." Organizational gains vs. what may be missing.

**Script:**
"The core tension of deploying generative AI tools at scale can be summarized simply: **The visible win hides the invisible cost**.

Organizations are easily enticed by what is visible—faster draft generation, tickets closed, and a drastic drop in syntax errors. But what is hidden from the ledger is a form of **cognitive debt**. We are seeing compromised skill acquisition, stunted debugging mechanics, and a passive acceptance of plausible-sounding but fundamentally incorrect code. It forces us to confront an uncomfortable reality: *How many of us have approved or shipped code this week that we could not fully explain line by line?*"

---

## Slide 3: Article Question (01 ARTICLE QUESTION)

**Visual:** Core Question: Does AI provide a shortcut to both efficiency and skill development?

**Script:**
"This brings us directly to the core objective of the Shen & Tamkin study. We have all seen the *prior productivity story*—massive headline figures showing that AI can accelerate familiar, well-rehearsed tasks.

But this study asks an essential question: What happens when developers utilize AI while attempting to master a completely unfamiliar framework? To isolate this, the researchers chose **Trio**, an asynchronous programming library in Python that is highly conceptual and intentionally outside the everyday experience of most developers. Does AI act as a shortcut to both immediate completion and long-term competence, or does it bypass the learning loop entirely?"

---

## Slide 4: The Experiment (02 THE EXPERIMENT)

**Visual:** Experimental timeline: 1 Recruit $\rightarrow$ 2 Calibrate $\rightarrow$ 3 Learn $\rightarrow$ 4 Evaluate.

**Script:**
"To untangle task completion from skill retention, the researchers designed a randomized controlled experiment. They recruited 52 Python developers with no prior exposure to Trio.

The workflow was strictly controlled. First, a 10-minute unassisted warm-up calibrated baseline development speed. Next, the 35-minute learning phase kicked off. Participants were randomly split into two streams to complete two Trio tasks: the **AI Condition** provided a chat-based assistant in a sidebar that could ingest their current code and generate active solutions; the **No-AI Condition** provided identical reading materials but mandated entirely manual coding. Finally, everyone sat for a 25-minute evaluation quiz with **zero AI access**."

---

## Slide 5: What Exactly Did They Ask Developers to Learn?

**Visual:** Breakdown of Task 1 (Timer/Nurseries) and Task 2 (Record retrieval/Error handling).

**Script:**
"The tasks were meticulously selected to simulate real-world software engineering onboarding. This wasn't a standard syntax check; it was an architectural test.

Task 1 required building a timer that prints every second concurrently alongside other functions, forcing developers to implement Trio’s core concurrency abstraction: **nurseries**. Task 2 scaled up the difficulty, requiring a record retrieval system with strict error handling, testing their grip on **memory channels** and complex async result flows. This layout perfectly mirrors joining a new team: you parse some brief docs, look at starter code, and are expected to ship a minor feature."

---

## Slide 6: The Evaluation Quiz

**Visual:** Quiz breakdown: 14 questions, 27 points, 4 assessment types.

**Script:**
"The evaluation quiz was built to evaluate independent AI supervision. If your assistant vanishes, can you still read, audit, and fix the codebase?

The 14-question, 27-point evaluation targeted four distinct dimensions derived from computer science education literature: **Debugging** (diagnosing hidden faults), **Code Reading** (verifying correctness line-by-line), **Code Writing** (architecting an independent approach), and **Conceptual Inquiry** (explaining the underlying engine mechanics)."

---

## Slide 7 & 8: Results & The Retention Crash (03 RESULTS)

**Visual:** Completion Time ($p=0.391$) vs. Quiz Score ($p=0.010$) graphs.

**Script:**
"When the data came in, it revealed what the paper frames as a highly significant **learning penalty**.

First, look at the productivity outcome. On average, the AI-assisted group finished the tasks only about two minutes faster than the manual coders. Statistically, this time difference was **not significant** ($p=0.391$).

But look at the quiz scores. The manual coding group averaged a solid **67%**, whereas the AI-assisted group dropped to a **50%** average. That is a massive 17% drop—nearly two full letter grades—and it is statistically highly significant with a large effect size ($d=0.738, p=0.010$). Shipping a completed task did not equate to retaining underlying knowledge."

---

## Slide 9 & 10: Skill Development by Experience & The Productivity Paradox

**Visual:** Data charts showing quiz scores and task times by years of coding experience.

**Script:**
"A common counter-argument is that this penalty only applies to junior engineers. However, when you segment the data by experience—1-3 years, 4-6 years, and 7+ years—the learning gap persists broadly across the board.

Why does this happen? The answer lies in the subarea breakdown: **debugging is where the gap bites hardest**. The manual group encountered significantly more errors during development, forcing them to manually diagnose, trace, and repair them. That exact friction is what trains engineering judgment. Because the AI group offloaded their troubleshooting, they skipped the cognitive struggle, leaving their mental models completely unverified."

---

## Slide 11 & 12: Behavioral Personas (04 BEHAVIORAL PERSONAS)

**Visual:** The 6 AI interaction modes categorized into Low skill development vs. High skill development interactions.

**Script:**
"The critical finding of Shen & Tamkin’s research is that 'AI use' is not a monolith; **the interaction pattern dictates the learning outcome**. They identified six qualitative behavioral personas.

Let's look at the **Low-Skill Development Interactions**, which pulled quiz scores below 40%:

* **AI Delegation**: Pure cognitive offloading. Asking the tool to write the full block and blindly pasting it.


* **Progressive AI Reliance**: Hand-coding the baseline task, hitting a wall on the second, and completely surrendering the architecture to the model.


* **Iterative AI Debugging**: Treating the AI as a rapid-fire trial-and-error crutch to fix runtime errors without processing *why* the failure occurred.



Now, look at the **High-Skill Development Interactions**, where scores remained at or above 65%:

* **Conceptual Inquiry**: Using the model purely to clarify documentation or library concepts while writing every line of code independently.


* **Hybrid Code-Explanation**: Demanding that code generation be accompanied by comprehensive structural explanations, taking time to read them before compiling.


* **Generation-then-comprehension**: Getting a snippet, but immediately engaging in active, focused follow-up queries to stress-test their own understanding."



---

## Slide 13: Caveat (05 CAVEAT)

**Visual:** Methodology note regarding qualitative clusters vs. controlled variables.

**Script:**
"As systems engineers and researchers, we must call out the methodological boundaries of these personas. These six interaction styles represent descriptive qualitative clusters drawn from small groups ($n=2$ to $7$).

They are not independently controlled, randomized conditions. We cannot definitively state whether delegation *caused* the lower quiz scores, or if developers with weaker baseline asynchronous programming skills simply fell back on delegation because they were severely stuck. It highlights a vital correlation: active cognitive friction matches better retention."

---

## Slide 14: Personal Trace: Delegation (06 PERSONAL TRACE)

**Visual:** Personal troubleshooting example with Tonic profiling. Ask $\rightarrow$ Inspect $\rightarrow$ Failure.

**Script:**
"Let me ground this paper's theory in a recent, real-world trace from my own systems engineering workflow. I was profiling a high-performance distributed network framework—specifically an asynchronous gRPC system using Tonic.

I fell straight into the **AI Delegation** trap. I asked the model to quickly generate a general code path for microsecond-level system profiling. When I inspected the code, the benchmark was totally corrupted; the hot-path timers it generated altered the cache lines and instrumentation overhead, fundamentally changing the system's behavior. It gave me highly plausible but structurally disastrous advice on a performance gap."

---

## Slide 15: Personal Trace: Directed Debugging (06 PERSONAL TRACE)

**Visual:** System architecture trace: Intel DSA 64-byte descriptor / 32-byte completion alignment logic.

**Script:**
"Compare that to when I consciously switched to a **Conceptual Inquiry** loop. I was debugging a low-level hardware-software co-design problem: alignment for Intel Data Streaming Accelerator (DSA) descriptors inside asynchronous coroutine-frame storage.

I owned the structural model. I forced the assistant to operate within a rigid hardware contract: a 64-byte descriptor and a 32-byte completion block. I directed Claude to explicitly inspect the alignment bounds, challenge standard compiler macro expansions, and track exactly how pointers were being evaluated.

Together, we derived the solution: over-allocate the heap buffer, compute the bitwise pointer adjustments once, and cache those hot addresses. The mathematical constraint—`(base + align - 1) & ~(align - 1)`—was driven by my direction. The assistant executed the search, but I maintained the oversight."

---

## Slide 16: Discussion (07 DISCUSSION / REFERENCES)

**Visual:** Final takeaway: "AI-enhanced productivity is not a shortcut to competence."

**Script:**
"We conclude with the definitive thesis statement of Shen & Tamkin's work: **AI-enhanced productivity is not a shortcut to competence**.

If engineering oversight, architectural safety, and systems verification depend on long-term human expertise, our tools, IDE configurations, and educational frameworks must protect the practice loop.

If we optimize our environments for zero friction, we risk optimizing our brains for zero retention. This leaves us with a critical question for discussion: As AI agents become more autonomous, what engineering difficulties must we intentionally design *back* into computer science education to ensure students actually learn?

Thank you, and let's open the floor."