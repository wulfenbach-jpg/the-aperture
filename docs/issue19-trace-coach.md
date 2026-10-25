# Issue19 — trace-backed teaching on the native table

Every coaching statement on the native table is derived from one of three
sources: the deterministic engine state, the generated legal-action list, or the
pinned Comprehensive Rules notes (revision 2026-09-25). Nothing is generated
prose and nothing is invented by a model.

## What the coach shows

Each coaching line carries a category badge:

- **[RULE] (TEXT color)** — a fact derived from the engine state or the pinned
  rule notes, with CR citations. Topics: `priority`, `pass round`,
  `response window`, `stack`, `pending choice`, `state-based actions`, `board`,
  `legal actions`, and per-card `… unavailable` legality reasons.
- **[SIMPLIFICATION] (WARN color)** — an engine behavior that deliberately
  simplifies a rule, taught as the simplification, never as the rule. Today
  that is automatic lethal-first combat damage assignment (CR 510.1c is the
  controller's choice; typed division is not modeled).
- **[STRATEGY] (BRASS color)** — strategy is never derived from engine data in
  this slice, so the panel renders a single explicit absence record and no
  win probabilities or advice styled as rules.

## Pass round / response window / APNAP

- The *pass round* record lists the seats that have passed consecutively in the
  current priority round (CR 117.3d) and names what resolves next once every
  remaining seat passes: the top stack object, or the end of the step/phase
  when the stack is empty (CR 117.4).
- The *response window* record appears while an object sits on the stack and
  states that any player may act first.
- APNAP pending choices (Crack the Earth sacrifices, commander returns) state
  that seats answer one at a time in turn order (CR 101.4 paraphrase). A seat
  that is not answering is told the ordering but never the private choice label.

Colors and category labels, as well as the coaching body text, are rendered on
the frame but every fact string is computed synchronously from the state — no
IO and no hidden opponent hand/library data.

## How to try it on the real table

1. Run `LAUNCH.cmd` from the released package (or `launch.cmd` from source).
2. Press `C` to show/hide the coaching panel (shown by default).
3. Cast a spell or play a land: the response window and pass-round records
   appear; press `P` to pass and watch consecutive passes accumulate until the
   top object resolves or the step ends.
4. In a Crack the Earth game, an APNAP pending-choice record explains the
   turn-order sequencing.

## Scope and limits (unchanged, disclosed)

28-card practice pool; one supported commander per seat; 20-life and bounded
40-life profiles only. Not arbitrary 100-card Commander, partners, the legend
rule, all mechanics, a trainer or v1. Automatic combat-damage assignment remains
a disclosed simplification. Older saves are refused explicitly; save/replay
digests are unchanged by these additive explanations.
