// Discussion deck: AI assistance and coding skill formation.
// Reader: CSE 599H / human-AI interaction classmates.
// Claim boundary: discussion framing of Shen & Tamkin 2026; no long-term retention claim.
// Sources:
// - docs/plan/2026-05-27/04.ai-coding-skill-formation-discussion.done.md
// - docs/plan/2026-05-27/10.ai-coding-deck-personal-debugging-examples.done.md
// - docs/plan/2026-05-27/11.ai-coding-deck-coroutine-directed-debugging.done.md
// - Anthropic Research, "How AI assistance impacts the formation of coding skills", 2026.
// - Shen & Tamkin, "How AI Impacts Skill Formation", arXiv:2601.20245, 2026.

// - Storey, "How Generative and Agentic AI Shift Concern from Technical Debt to Cognitive Debt", 2026.
// - Sarkar, "Intention Is All You Need", arXiv:2410.18851, 2024.
#import "@preview/touying:0.6.3": *
#let paper = rgb("#f8f0e6")
#let ink = rgb("#11253f")
#let muted = rgb("#756f66")
#let blue = rgb("#315d87")
#let blue-soft = rgb("#e5ecf1")
#let coral = rgb("#c75d51")
#let coral-soft = rgb("#f1ded9")
#let green = rgb("#287d5d")
#let green-soft = rgb("#deeee5")
#let amber = rgb("#c6922e")
#let amber-soft = rgb("#f3e7cf")
#let fog = rgb("#eee7db")

#let fs-caption = 12pt
#let fs-small = 14pt
#let fs-cover-subtitle = 18pt
#let fs-body = 19pt
#let fs-slide-subtitle = 20pt
#let fs-prompt = 23pt
#let fs-step-index = 25pt
#let fs-score-value = 26pt
#let fs-question = 25pt
#let fs-accent-heading = 38pt
#let fs-fact = 42pt
#let fs-arrow-large = 48pt
#let fs-cover-title = 35pt
#let fs-metric = 52pt
#let fs-activity-index = 56pt

#show: touying-slides.with(
  config-page(width: 16in, height: 9in, margin: (x: 68pt, y: 46pt), fill: paper),
)
#set text(font: "Latin Modern Sans", size: fs-body, fill: ink)
#set par(leading: 0.86em, spacing: 0.52em)

#let plain(body, font-size, fill: ink) = text(size: font-size, fill: fill)[#body]
#let strong(body, font-size, fill: ink) = text(size: font-size, weight: "bold", fill: fill)[#body]
#let small(body) = plain(body, fs-small, fill: muted)
#let tiny(body) = plain(body, fs-caption, fill: muted)
#let slide-number() = context tiny[#utils.slide-counter.display() / #utils.last-slide-number]
#let cap(body, accent: blue) = strong(body, fs-caption, fill: accent)
#let emph(body, accent: blue) = text(weight: "bold", fill: accent)[#body]
#let cover-title(body) = strong(body, fs-cover-title)
#let cover-subtitle(body) = plain(body, fs-cover-subtitle, fill: muted)
#let cover-prompt(body) = strong(body, fs-body, fill: coral)
#let slide-title-text(body) = strong(body, fs-cover-title, fill: ink)
#let slide-subtitle(body) = strong(body, fs-slide-subtitle)
#let prompt-text(body) = strong(body, fs-prompt)
#let prompt-quote(body) = strong(body, fs-prompt)
#let list-emphasis(body, accent: ink) = plain(body, fs-step-index, fill: accent)
#let lane-title(body, accent: blue) = strong(body, fs-body, fill: accent)
#let divider-text(body) = plain(body, fs-prompt, fill: muted)
#let divider-large(body) = strong(body, fs-arrow-large, fill: amber)
#let step-index(body, accent: blue) = strong(body, fs-step-index, fill: accent)
#let task-title(body, accent: blue) = strong(body, fs-question, fill: accent)
#let fact-number(body, accent: blue) = strong(body, fs-fact, fill: accent)
#let score-label(body) = strong(body, fs-cover-subtitle)
#let score-value(body, accent: blue) = strong(body, fs-score-value, fill: accent)
#let panel-title(body) = strong(body, fs-step-index)
#let panel-callout(body, accent: coral) = strong(body, fs-slide-subtitle, fill: accent)
#let accent-heading(body, accent: coral) = strong(body, fs-accent-heading, fill: accent)
#let statement-text(body) = strong(body, fs-score-value)
#let metric-number(body, accent: blue) = strong(body, fs-metric, fill: accent)
#let activity-index(body, accent: blue) = strong(body, fs-activity-index, fill: accent)
#let discussion-question(body, accent: ink) = strong(body, fs-question, fill: accent)
#let closing-thesis(body, accent: green) = strong(body, fs-question, fill: accent)
#let hair(accent: blue, width: 80pt) = rect(width: width, height: 1.5pt, fill: accent)

#let panel(body, fill: fog, inset: (x: 18pt, y: 14pt), radius: 18pt, height: auto) = block(
  width: 100%,
  height: height,
  inset: inset,
  radius: radius,
  fill: fill,
)[#body]

#let prompt-band(body, accent: amber) = block(
  width: 100%,
  inset: (x: 20pt, y: 13pt),
  stroke: (left: 3pt + accent),
)[#body]

#let pill(body, accent: blue, fill: fog) = block(
  radius: 999pt,
  inset: (x: 13pt, y: 7pt),
  fill: fill,
  stroke: 0.45pt + accent,
)[#strong(body, fs-small, fill: accent)]

#let header(section, accent: blue) = [
  #grid(
    columns: (1fr, auto),
    [#cap(section, accent: accent)], [#slide-number()],
  )
  #v(0.42em)
  #hair(accent: accent)
]

#let story-slide(section, title, body, accent: blue) = [
  #header(section, accent: accent)
  #v(0.55em)
  #slide-title-text(title)
  #v(0.75em)
  #body
]

#let big-line(body, accent: amber) = [
  #slide-subtitle(body)
  #v(0.28em)
  #hair(accent: accent, width: 94pt)
]

#let split-lanes(left-title, left-body, right-title, right-body) = [
  #grid(
    columns: (1fr, 0.08fr, 1fr),
    gutter: 18pt,
    [
      #panel(fill: blue-soft, radius: 0pt)[
        #lane-title(left-title, accent: blue)
        #v(0.5em)
        #left-body
      ]
    ],
    [#align(center + horizon)[#divider-text[vs.]]],
    [
      #panel(fill: green-soft, radius: 0pt)[
        #lane-title(right-title, accent: green)
        #v(0.5em)
        #right-body
      ]
    ],
  )
]

#let step-num(n, title, body, accent: blue, fill: fog, height: auto) = [
  #panel(fill: fill, inset: (x: 15pt, y: 13pt), height: height)[
    #step-index(n, accent: accent)
    #h(0.5em)
    #text(weight: "bold", fill: accent)[#title]
    #v(0.45em)
    #body
  ]
]

#align(center + horizon)[
  #cap([CSE 599H · HUMAN–AI INTERACTION], accent: coral)
  #v(0.65em)
  #cover-title[The Illusion of Competence]
  #v(0.55em)
  #cover-subtitle[AI assistance, coding skills, and the hidden cost of easy answers]
  #v(0.85em)
  #hair(accent: coral, width: 136pt)
  #v(1.05em)
  #split-lanes(
    [artifact path],
    [ask AI → get code → task passes],
    [learning path],
    [predict → try → fail → debug → explain → retain],
  )
  #v(0.9em)
  #cover-prompt[Which path did you use the last time AI helped you code?]
  #v(0.35em)
  #tiny[Shen & Tamkin, Anthropic Research, 2026]
]

#pagebreak()

#story-slide(
  [01 · HOOK],
  [The visible win hides the invisible cost],
  [
    #grid(
      columns: (0.33fr, 0.40fr, 0.33fr),
      gutter: 24pt,
      [
        #cap([WHAT ORGANIZATIONS SEE], accent: green)
        #v(0.5em)
        #list-emphasis[faster drafts]
        #v(0.2em)
        #list-emphasis[completed tickets]
        #v(0.2em)
        #list-emphasis[fewer syntax errors]
      ],
      [
        #align(center + horizon)[
          #divider-large[→]
        ]
      ],
      [
        #cap([WHAT MAY BE MISSING], accent: coral)
        #v(0.5em)
        #list-emphasis([weaker mental model], accent: coral)
        #v(0.2em)
        #list-emphasis([less debugging practice], accent: coral)
        #v(0.2em)
        #list-emphasis([less rejection of plausible wrong code], accent: coral)
      ],
    )

    #v(0.95em)
    #prompt-band(accent: amber)[
      #prompt-quote[“How many of us have approved or shipped code this week that we could not fully explain line by line?”]
    ]
  ],
  accent: coral,
)

#pagebreak()

#story-slide(
  [01 · HOOK],
  [Oversight is not a job title. It is a skill.],
  [
    #v(0.15em)
    #big-line[If AI writes more code, human review becomes more important, not less.]
    #v(0.85em)

    #grid(
      columns: (1fr, 0.12fr, 1fr, 0.12fr, 1fr),
      gutter: 10pt,
      [#panel(fill: blue-soft)[#align(center)[#emph([AI generates], accent: blue)]]],
      [#align(center + horizon)[#divider-text[→]]],
      [#panel(fill: amber-soft)[#align(center)[#emph([human reviews], accent: amber)]]],
      [#align(center + horizon)[#divider-text[→]]],
      [#panel(fill: green-soft)[#align(center)[#emph([system ships], accent: green)]]],
    )

    #v(0.9em)
    #grid(
      columns: (0.36fr, 0.64fr),
      gutter: 34pt,
      [
        #cap([REVIEW REQUIRES], accent: blue)
        #v(0.45em)
        detect structural errors \
        read unfamiliar code \
        understand library semantics \
        say no to plausible output
      ],
      [
        #prompt-band(accent: coral)[
          The paper asks whether AI-assisted work builds the review skill or bypasses the practice loop.
        ]
      ],
    )
  ],
  accent: coral,
)

#pagebreak()

#story-slide(
  [02 · THE EXPERIMENT],
  [The experiment as a pipeline],
  [
    #big-line[The design separates “finished the task” from “retained the skill.”]
    #v(0.8em)

    #grid(
      columns: (1fr, 1fr, 1fr, 1fr),
      gutter: 14pt,
      [#step-num([1], [Recruit], [52 Python developers; Trio unfamiliar], accent: blue, height: 108pt)],
      [#step-num([2], [Calibrate], [warm-up task without AI], accent: blue, fill: blue-soft, height: 108pt)],
      [#step-num(
        [3],
        [Randomize],
        [two Trio tasks; AI sidebar or no AI],
        accent: amber,
        fill: amber-soft,
        height: 108pt,
      )],
      [#step-num(
        [4],
        [Evaluate],
        [final quiz with #emph([no AI], accent: green)],
        accent: green,
        fill: green-soft,
        height: 108pt,
      )],
    )

    #v(0.8em)
    #grid(
      columns: (1fr, 1fr),
      gutter: 24pt,
      [
        #cap([AI CONDITION], accent: green)
        #v(0.25em)
        chat-based coding assistant in the sidebar; it could see current code and produce correct code
      ],
      [
        #cap([NO-AI CONDITION], accent: blue)
        #v(0.25em)
        same learning materials and coding tasks, but no assistant during the Trio stage
      ],
    )
  ],
  accent: blue,
)

#pagebreak()

#story-slide(
  [02 · THE EXPERIMENT],
  [What exactly did they ask developers to learn?],
  [
    #big-line[Trio was a deliberately unfamiliar async library, not a Python syntax test.]
    #v(0.75em)

    #grid(
      columns: (1fr, 1fr),
      rows: (auto, auto),
      gutter: 28pt,
      [
        #task-title([Task 1], accent: blue)

        Write a timer that prints each passing second while other functions run.
      ],
      [
        #task-title([Task 2], accent: green)

        Implement record retrieval that handles missing-record errors.

      ],

      [
        #cap([TARGET CONCEPTS], accent: blue)

        nurseries · starting tasks · concurrent functions
      ],
      [
        #cap([TARGET CONCEPTS], accent: green)

        error handling · memory channels · result flow
      ],
    )

    #v(0.85em)
    #prompt-band(accent: amber)[
      The task simulates workplace onboarding: a short tutorial, starter code, then a small feature using a new library.
    ]
  ],
  accent: blue,
)

#pagebreak()

#story-slide(
  [02 · THE EXPERIMENT],
  [How did they measure learning?],
  [
    #big-line[The quiz measured whether participants could supervise code without the assistant.]
    #v(0.75em)

    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 20pt,
      [
        #fact-number([14], accent: coral)
        #v(0.15em)
        questions
        #v(0.35em)
        #small[27 total points]
      ],
      [
        #fact-number([7], accent: blue)
        #v(0.15em)
        Trio concepts
        #v(0.35em)
        #small[covered by the two tasks]
      ],
      [
        #fact-number([0], accent: green)
        #v(0.15em)
        AI access
        #v(0.35em)
        #small[during the quiz]
      ],
    )

    #v(0.9em)
    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 20pt,
      [
        #cap([DEBUGGING], accent: coral)
        #v(0.25em)
        identify why code fails
      ],
      [
        #cap([CODE READING], accent: blue)
        #v(0.25em)
        understand unfamiliar code
      ],
      [
        #cap([CONCEPTUAL], accent: green)
        #v(0.25em)
        know the library model
      ],
    )

    #v(0.7em)
    #tiny[They excluded code-writing questions to reduce Python syntax confounds; the quiz rubric was submitted in preregistration.]
  ],
  accent: blue,
)

#pagebreak()

#story-slide(
  [02 · THE EXPERIMENT],
  [How did they conclude there was a learning penalty?],
  [
    #big-line[Random assignment makes the comparison: same task, different access to AI.]
    #v(0.7em)

    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 22pt,
      [
        #cap([GROUPS], accent: blue)
        #v(0.3em)
        26 AI participants \
        26 no-AI participants
      ],
      [
        #cap([PRODUCTIVITY OUTCOME], accent: amber)
        #v(0.3em)
        completion time difference: about two minutes \
        #emph([not significant], accent: amber) (`p=0.391`)
      ],
      [
        #cap([LEARNING OUTCOME], accent: coral)
        #v(0.3em)
        quiz gap: 4.15 points on a 27-point quiz
        #emph([significant], accent: coral) (`d=0.738`, `p=0.010`)
      ],
    )

    #v(0.9em)
    #prompt-band(accent: coral)[
      Their claim is not “AI users failed the task.” It is: after the same learning task, AI access predicted lower unassisted mastery.
    ]
  ],
  accent: blue,
)

#pagebreak()

#story-slide(
  [03 · PRODUCTIVITY PARADOX],
  [The productivity story breaks],
  [
    #big-line[Lower scores, no reliable speed win.]
    #v(0.78em)

    #let score-row(label, pct, accent, filled, empty) = grid(
      columns: (300pt, 400pt, auto),
      gutter: 18pt,
      align: horizon,
      [#score-label(label)],
      [
        #grid(
          columns: (filled, empty),
          gutter: 0pt,
          [#rect(width: filled, height: 17pt, fill: accent)],
          [#rect(width: empty, height: 17pt, fill: paper, stroke: 0.8pt + accent)],
        )
      ],
      [#score-value(pct, accent: accent)],
    )

    #align(center)[
      #block(width: 88%, inset: (x: 28pt, y: 22pt), radius: 20pt, fill: fog)[
        #align(center)[#panel-title[THE RETENTION CRASH]]
        #v(0.9em)
        #score-row([Hand-coding group score:], [67%], green, 268pt, 132pt)
        #v(0.55em)
        #score-row([AI-assisted group score:], [50%], coral, 200pt, 200pt)
        #v(0.85em)
        #align(center)[#panel-callout([Statistically significant 17 pp learning drop], accent: coral)]
        #v(0.2em)
        #align(center)[#small[Average time gain: about two minutes, not statistically significant.]]
      ]
    ]

    #v(0.72em)
    #prompt-band(accent: amber)[
      #prompt-text[A finished task did not translate into retained mastery.]
    ]
  ],
  accent: coral,
)

#pagebreak()

#story-slide(
  [03 · PRODUCTIVITY PARADOX],
  [Debugging is where the gap bites],
  [
    #accent-heading([debugging questions], accent: coral)
    #v(0.2em)
    #statement-text[were the largest group gap.]
    #v(0.85em)

    #grid(
      columns: (1fr, 0.12fr, 1fr, 0.12fr, 1fr),
      gutter: 10pt,
      [#panel(fill: coral-soft)[#align(center)[No-AI learners hit errors]]],
      [#align(center + horizon)[#divider-text[→]]],
      [#panel(fill: amber-soft)[#align(center)[they repaired those errors]]],
      [#align(center + horizon)[#divider-text[→]]],
      [#panel(fill: green-soft)[#align(center)[repair trained judgment]]],
    )

    #v(0.85em)
    #grid(
      columns: (0.5fr, 0.5fr),
      gutter: 30pt,
      [
        #cap([OBSERVED PATTERN], accent: coral)
        #v(0.3em)
        Largest gap: debugging. \
        Smallest gap: code reading.
      ],
      [
        #cap([CAVEAT], accent: blue)
        #v(0.3em)
        Plausible mechanism, not proven mediation.
      ],
    )
  ],
  accent: coral,
)

#pagebreak()

#story-slide(
  [03 · PRODUCTIVITY PARADOX],
  [Where did the time savings go?],
  [
    #big-line[Generation is fast. The workflow is not.]
    #v(0.75em)

    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 22pt,
      [
        #metric-number([11], accent: amber)
        #v(-0.1em)
        #cap([MINUTES], accent: amber)
        #v(0.3em)
        some participants spent composing queries
      ],
      [
        #metric-number([15], accent: blue)
        #v(-0.1em)
        #cap([QUESTIONS], accent: blue)
        #v(0.3em)
        some participants asked the assistant
      ],
      [
        #metric-number([✓], accent: green)
        #v(-0.1em)
        #cap([CHECKING], accent: green)
        #v(0.3em)
        the workflow still included validation and fixing
      ],
    )

    #v(0.95em)
    #align(center)[
      #discussion-question[The model may be working;]
      #h(0.25em)
      #discussion-question([the learner may not be practicing.], accent: coral)
    ]
  ],
  accent: coral,
)

#pagebreak()

#story-slide(
  [04 · BEHAVIORAL PERSONAS],
  [Not all AI use is the same],
  [
    #big-line[The difference is whether AI removes practice or scaffolds it.]
    #v(0.75em)

    #grid(
      columns: (1fr, 1fr),
      gutter: 28pt,
      [
        #panel(fill: coral-soft, height: 202pt)[
          #cap([OFFLOADING PATTERNS], accent: coral)
          #v(0.55em)
          - delegation
          - progressive reliance
          - AI debugging
          #v(0.85em)
          asks the model to write, diagnose, or finish the hard part
        ]
      ],
      [
        #panel(fill: green-soft, height: 202pt)[
          #cap([LEARNING PATTERNS], accent: green)
          #v(0.55em)
          - conceptual inquiry
          - code + explanation
          - self-checking
          #v(0.85em)
          keeps prediction, explanation, or judgment in the human loop
        ]
      ],
    )

    #v(0.75em)
    #tiny[The interaction clusters are descriptive; they do not by themselves prove causal effects.]
  ],
  accent: green,
)

#pagebreak()

#story-slide(
  [04 · BEHAVIORAL PERSONAS],
  [Same assistant. Different practice loop.],
  [
    #split-lanes(
      [artifact path],
      [
        #cap([DELEGATION], accent: blue)
        1. User: write the solution
        2. AI: here is code
        3. User: paste / pass

        #v(1em)

        #emph([Outcome: cannot debug variant], accent: coral)
      ],
      [learning path],
      [
        #cap([LEARNING], accent: green)
        #v(0.35em)
        1. User: asks concept \
        2. User: predicts behavior \
        3. User: tries, hits error \

        #v(1em)

        #emph([Outcome: can debug variant], accent: green)
      ],
    )
  ],
  accent: green,
)

#pagebreak()

#story-slide(
  [04 · BEHAVIORAL PERSONAS],
  [Which persona was your last AI session?],
  [
    #big-line[Think of the last time AI helped you code.]
    #v(0.8em)

    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 28pt,
      [
        #activity-index([1], accent: blue)
        #v(0.15em)
        Did I predict before asking?
      ],
      [
        #activity-index([2], accent: amber)
        #v(0.15em)
        Did I debug anything myself?
      ],
      [
        #activity-index([3], accent: green)
        #v(0.15em)
        If the model disappeared, what could I still explain?
      ],
    )

    #v(0.75em)
    #small[#emph([Pair-share:], accent: green) delegation, debugging-by-AI, or learning-oriented?]
  ],
  accent: green,
)

#pagebreak()

#story-slide(
  [05 · STRATEGIC PIVOT],
  [Generator or coach?],
  [
    #big-line[AI tools can preserve effort instead of bypassing it.]
    #v(0.75em)

    #grid(
      columns: (1fr, 1fr),
      gutter: 20pt,
      [
        #panel(fill: blue-soft)[#emph([Ask-before-answer], accent: blue) \
          elicit a hypothesis first]
      ],
      [
        #panel(fill: blue-soft)[#emph([Hint-before-fix], accent: blue) \
          support debugging without taking it over]
      ],

      [
        #panel(fill: green-soft)[#emph([Explain-after-generation], accent: green) \
          require comprehension]
      ],
      [
        #panel(fill: green-soft)[#emph([Mode clarity], accent: green) \
          ship mode versus learn mode]
      ],
    )

    #v(0.75em)
    #prompt-band(accent: amber)[
      #prompt-text[Which lever would annoy you in production but help you while learning?]
    ]
  ],
  accent: amber,
)

#pagebreak()

#story-slide(
  [06 · PERSONAL TRACE],
  [When delegation felt productive but did not teach enough],
  [
    #big-line[My Tonic profiling struggle looked like the paper's warning sign.]
    #v(0.72em)

    #grid(
      columns: (1fr, 0.08fr, 1fr, 0.08fr, 1fr),
      gutter: 10pt,
      [
        #panel(fill: blue-soft, height: 154pt)[
          #cap([ASK], accent: blue)
          #v(0.45em)
          AI help produced runs, tables, and dashboards.
        ]
      ],
      [#align(center + horizon)[#divider-text[→]]],
      [
        #panel(fill: coral-soft, height: 154pt)[
          #cap([INSPECT], accent: coral)
          #v(0.45em)
          Comparisons were confounded; hot-path timers changed the system.
        ]
      ],
      [#align(center + horizon)[#divider-text[→]]],
      [
        #panel(fill: green-soft, height: 154pt)[
          #cap([REFRAME], accent: green)
          #v(0.45em)
          Matched regimes; instrumentation-off throughput; perf evidence.
        ]
      ],
    )

    #v(0.75em)
    #grid(
      columns: (0.38fr, 0.62fr),
      gutter: 28pt,
      [
        #cap([WHAT FINALLY HELPED], accent: green)
        #v(0.3em)
        bounded matrix \
        bucketed timers as diagnostic mode \
        buffer-policy controls \
        frame-pointer / perf reruns
      ],
      [
        #prompt-band(accent: coral)[
          #prompt-text[The learning happened when I challenged the artifact, not when the artifact appeared.]
        ]
      ],
    )
  ],
  accent: coral,
)

#pagebreak()

#story-slide(
  [06 · PERSONAL TRACE],
  [The coroutine bug worked because I steered the debugger],
  [
    #big-line[I asked Claude to debug, but I supplied the hypothesis and boundary.]
    #v(0.74em)

    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 18pt,
      [
        #panel(fill: blue-soft, height: 174pt)[
          #cap([MY DIRECTION], accent: blue)
          #v(0.45em)
          Focus on DSA alignment inside coroutine-frame storage: \
          64-byte descriptor, 32-byte completion.
        ]
      ],
      [
        #panel(fill: amber-soft, height: 174pt)[
          #cap([CLAUDE'S WORK], accent: amber)
          #v(0.45em)
          inspect layout \
          challenge `alignas()` \
          trace where addresses are formed
        ]
      ],
      [
        #panel(fill: green-soft, height: 174pt)[
          #cap([FIX], accent: green)
          #v(0.45em)
          over-allocate \
          compute aligned pointers once \
          cache hot-path addresses
        ]
      ],
    )

    #v(0.72em)
    #grid(
      columns: (0.50fr, 0.50fr),
      gutter: 30pt,
      [
        #panel(fill: fog, inset: (x: 18pt, y: 15pt))[
          #cap([DEBUGGING MODEL], accent: amber)
          #v(0.3em)
          `(base + align - 1) & ~(align - 1)` preserved the hardware contract when coroutine storage made naive alignment suspect.
        ]
      ],
      [
        #prompt-band(accent: green)[
          Learning-oriented delegation: outsource the search, keep hypothesis, evidence, and judgment in the human loop.
        ]
      ],
    )
  ],
  accent: green,
)

#pagebreak()

#story-slide(
  [07 · DISCUSSION],
  [One question to end on],
  [
    #big-line[If AI can produce the artifact, what should undergraduate CS education protect?]
    #v(0.9em)

    #panel(fill: green-soft, inset: (x: 28pt, y: 28pt), height: 248pt)[
      #cap([DISCUSSION], accent: green)
      #v(0.55em)
      #discussion-question[Which practice loop should remain mostly human: prediction, debugging, explanation, or design judgment?]
      #v(0.75em)
      #small[Pick one, then propose a course policy that preserves it without pretending AI does not exist.]
    ]

    #v(0.8em)
    #prompt-band(accent: amber)[
      #prompt-text[What should we still make students struggle through—and why?]
    ]
  ],
  accent: green,
)

#pagebreak()

#story-slide(
  [REFERENCES],
  [AI-enhanced productivity is not a shortcut to competence],
  [
    #closing-thesis[If future oversight depends on human expertise, our tools and habits must preserve the practice loop.]
    #v(1.0em)
    #hair(accent: green, width: 110pt)
    #v(0.9em)

    #grid(
      columns: (0.24fr, 0.76fr),
      gutter: 18pt,
      [#cap([PRIMARY], accent: blue)],
      [#small[Shen, Judy Hanwen, and Alex Tamkin. “How AI Impacts Skill Formation.” arXiv:2601.20245, 2026.]],

      [#cap([ARTICLE], accent: blue)],
      [#small[Anthropic Research. “How AI assistance impacts the formation of coding skills.” 2026.]],

      [#cap([RELATED], accent: blue)],
      [#small[Storey, Margaret-Anne. “How Generative and Agentic AI Shift Concern from Technical Debt to Cognitive Debt.” 2026. margaretstorey.com/blog/2026/02/09/cognitive-debt/]],
      [#cap([RELATED], accent: blue)],
      [#small[Sarkar, Advait. “Intention Is All You Need.” arXiv:2410.18851, 2024. arxiv.org/abs/2410.18851]],
      [#cap([VIDEO], accent: blue)], [#small[Anthropic Research on AI and skill formation. YouTube reference: `bp8dwCJF6iw`.]],
      [#cap([CONTEXT], accent: blue)],
      [#small[Prior AI-productivity, critical-thinking, and cognitive-offloading work cited in the Anthropic article.]],
    )
  ],
  accent: blue,
)
