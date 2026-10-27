# Move review specification — information boundary, compute budget, ranking, presentation and honesty rules

Status: **authoritative design note for issue #28** (milestone M4, roadmap key `strategy-coach`).
It is the contract every other #28 child task builds against; where this note and an
implementation disagree, this note wins and the implementation is the defect.

| Field | Value |
| --- | --- |
| Spec version | 1.2 |
| Rules revision honoured | Comprehensive Rules effective **2026-09-25** (the pin already carried by `search.rs` and `PlayerView::rules_revision`) |
| Applies to | `crates/ai` (ranking layer + honesty guardrails), `crates/ui` (review presentation), `evidence/issue28` (validation) |
| Does not apply to | engine legality, catalog corpus, import, bundled decks, the optional local decision model |
| Normative words | MUST, MUST NOT, SHOULD, MAY are used in their RFC 2119 sense |

This note defines **no game rule**. Every statement about what is legal, what a card does, or
what a game state becomes is produced by the deterministic engine (`aperture_core`) and is
quoted by reference. Nothing in the review path is allowed to author a rule, infer one from
prose, or let a model supply one.

---

## 0. Definitions

- **Engine** — `aperture_core::Game`: the only authority for legality, state transitions, SBAs,
  pending choices and rule text.
- **Search** — `aperture_ai::search`: a budgeted information-set search over engine-submitted
  actions from the reviewing seat's masked observation.
- **Review** — the read-only artefact analysed in this note: a ranking of engine-offered
  alternatives at one decision point, plus the legal traces that support it.
- **Analysis** — one completed `aperture_ai::SearchOutcome` (decision + `SearchAnalysis`).
- **Candidate** — one engine-offered action at the reviewed decision point.
- **Line** — a sequence of engine-submitted actions recorded from one search rollout (the
  principal-variation analogue). Never synthesised, never model-generated.
- **Sample** — one rollout that reached an engine result. Its value is win `+1`, loss `−1` or
  draw `0`. A rollout that was capped, truncated or failed is a **rollout without result** and
  is neither a win nor a loss.
- **Traceable claim** — a sentence whose every factual assertion resolves to an evidence
  reference (`EvidenceRef`) present in the same payload.

### Decision registry

These are settled by this note. A child task MUST NOT re-decide them; if one is wrong, amend
this file in its own commit and say so in the handoff.

| id | Decision |
| --- | --- |
| D-1 | Ranking lives in a new **`aperture_ai::review`** module (data + derivation). `crates/ui/src/review.rs` keeps presentation only. No ranking logic in the UI. |
| D-2 | The review payload is plain data with closed enums; reason text is **derived** from tags + counts by one formatter, never typed free-hand. |
| D-3 | The score is the mean **practice outcome** over scored determinizations (win `+1` / loss `−1` / draw `0`). It is not a probability of anything. |
| D-4 | No win probability, no percentage, no centipawn-to-win-rate conversion, no confidence number. There is no centipawn scale in this project and none may be introduced. |
| D-5 | No verdict words (`blunder`, `mistake`, `inaccuracy`, `error`, `best move`, `should have`) in review output, except inside an explicit negation ("labels no alternative as an error"). |
| D-6 | Lines come only from recorded engine-submitted rollout steps. `aperture_ai::search` MUST gain bounded trace recording; until it does, the panel says no line exists rather than inventing one. |
| D-7 | Reproducibility is claimed only when the wall clock and cancellation did not truncate the analysis (see §3.3). The interactive budget is allowed to be non-reproducible; recorded evidence is not. |
| D-8 | Uncertainty is a combinatorial lattice statement over sample counts, not a statistical model fit (§4.5). |
| D-9 | Rule text and policy preference are produced by different constructors into different sections with different source labels; `ExplanationKind::Strategy` records never enter the rules section. |
| D-10 | The review is read-only: it submits no action, grants no priority, mutates no engine state. |
| D-11 | The search has **no depth metric** (rollout ensemble, not fixed-depth search), so no depth or plies figure is reported. Per-candidate cost is the rollout breakdown; global cost is `decisions_used`. |

---

## 1. Purpose and non-goals

A learner reviewing a decision needs three separate things, which this note keeps separate
everywhere:

1. **What the rules made possible** — the engine's offered actions and their rule citations.
2. **What a bounded search preferred, and how weakly** — ranked practice outcomes with the
   sample counts and the budget that produced them.
3. **What the tool does not know** — hidden information, small samples, an unfinished rollout,
   a truncated analysis.

Non-goals for #28: strength claims of any kind; a claim that "larger search plays better";
coverage of pending-choice decisions (mulligan, combat declarations, cleanup discard, trigger
targets) which the search refuses by design; retroactive review of earlier plies; and any
comparison against human or external engine strength.

---

## 2. Information boundary

### 2.1 What the reviewer may know

The reviewer — defined as `aperture_ai::review::rank` and everything downstream of it — may
consume **only** these inputs, and nothing else:

**A. The masked observation** (`aperture_core::view::PlayerView`), built for the reviewing
seat, and only after the review is bound to that seat:

- `viewer`, `viewer_name`, `seat_count`;
- `own_hand`, `own_battlefield`, `own_graveyard`, `own_exile`, `own_command_zone`,
  `own_library_count`, `own_life`, `own_eliminated`, `own_mana`, `own_mana_pool`,
  `own_lands_in_hand`, `own_mulligans`;
- per opponent: public name/deck name, life, hand/library counts, battlefield, graveyard,
  exile, command zone (`OpponentView`);
- `revealed` (CR 701.16a reveals that are public while they stay in the hidden zone);
- `commanders`, `commander_damage` (CR 903.10a public state);
- public turn state: `turn_number`, `active`, `step`, `priority`, `stack`, `attackers`,
  `blockers`, `pending`, `pending_owner`, `pending_kind`, `pending_index`, `pending_mode`,
  `pending_viewed`, `log`, `outcome`;
- `legal: Vec<LegalAction>` — the engine's own offer, each entry carrying `action`, `label`
  and `rules: Vec<&'static str>` (the citations backing its availability);
- `digest` (masked-projection digest only — never an oracle over hidden zones) and
  `rules_revision`.

**B. The analysis** (`aperture_ai::SearchAnalysis`): per-candidate `label`, `mean`,
`scored_samples`, `rollouts_without_result`, `refused_times`; `decisions_used`; `cancelled`;
`fidelity` (verbatim); and, after D-6, each candidate's recorded line (§4.4).

**C. The engine's own rule/trace explanations** for the offered actions
(`aperture_core::coach::Explanation { topic, kind, text, rules }`) and the pinned CR note text
reachable through `coach::rule_note`.

**D. The budget actually used** (§3) and the review binding (§4.2).

**E. The reviewing seat's own declared deck list** (`Knowledge::own_deck`) — legitimately
owned information, used only to refine that seat's own future draws. Absent it, the neutral
belief deck is used and the fidelity line says so.

**Permitted reads of the real engine state** (already enforced inside `reconstruct`, and
restated here so no downstream task widens it): object entries only for object ids that appear
in the view; seat-agnostic bookkeeping that no seat learns anything private from
(next-object/effect/timestamp counters, activation-use counts, seat names, deck labels); the
`trigger_queue`/`pending` emptiness gate. Opponent hands and library membership are never read;
they are re-dealt from the belief deck at real counts. No seat's real library order is used.

### 2.2 What the reviewer must never invent

The review MUST NOT contain, derive, estimate or imply any of the following. Each item is a
hard boundary, not a style preference.

1. **Win probability, win rate, "chance of winning", odds, equity, expected points, or any
   percentage.** The search's outcome is a mean of `{−1, 0, +1}` samples; mapping it to a
   probability would require a model nobody has validated.
2. **Centipawn-to-win-percentage conversion, or any centipawn scale.** This project has no
   centipawn unit and MUST NOT gain one. Chess evaluation units are not Magic evaluation units.
3. **A "forced blunder", "blunder", "mistake", "inaccuracy", "error", "best move", "only
   winning move", "should have played" verdict on the human's play.** The review may state what
   the engine offered and what the bounded search preferred. It passes no judgement.
4. **Handicapping by strength**: no Elo, no ranking against a named engine or player, no claim
   that more search or more samples mean stronger play.
5. **Hidden information**: opponent hand contents, opponent library order, the viewer's own
   library order beyond the declared list, face-down cards, or anything only an omniscient
   observer could know. "What they probably hold" is also forbidden — the belief deck is
   disclosed as a modelling choice, not reported as a belief about those hands.
6. **Fabricated lines**: a line whose steps were not engine-submitted actions, or a line for a
   candidate that never reached a result.
7. **Model-supplied content of any kind.** The review path MUST NOT consult
   `aperture_ai::DeciderClient`, and MUST NOT contain prose generated by a model. Models never
   supply game rules (project-wide constraint) and, inside the boundary of this note, models
   supply nothing at all.
8. **Silent defaults**: an absence of data (no scored sample, refused root, empty offer,
   truncated analysis) MUST be stated in the payload as an explicit state. It may never be
   rendered as an ordinary value, a zero, or an unranked omission.

### 2.3 Boundary proof obligation

Every implementation of this note MUST ship a test that constructs two real engine states whose
**masked views for the reviewing seat are identical** but which differ in an opponent's hand
contents, an opponent's library order, and the viewer's own undeclared library order, and
asserts the serialised review payload is **byte-identical** for both. That single test is the
operational definition of "no hidden reads" for this feature.

---

## 3. Compute budget and reproducibility

### 3.1 Search parameters

The review uses exactly `aperture_ai::SearchConfig`:

| field | meaning | review default |
| --- | --- | --- |
| `seeds` | determinization rounds; candidates round-robin one rollout per seed | `2` |
| `node_budget` | total engine decision points across all rollouts | `30_000` |
| `max_rollout_nodes` | per-rollout decision cap; hitting it yields a *rollout without result* | `6_000` |
| `max_millis` | wall-clock bound for the whole analysis; `u64::MAX` disables it | `8_000` interactive, `u64::MAX` for recorded evidence |
| `max_candidates` | root candidates taken from the engine's offer, in offer order | `6` |

The values above are the measured defaults already shipped as `REVIEW_BUDGET`; a change to any
of them is a spec change and MUST be accompanied by a re-measured justification recorded next
to the constant. `max_rollout_nodes` was raised from a guessed `1_500` to `6_000` because a
four-seat practice rollout does not reach a result below several thousand decisions — the
earlier value produced only "without result" rows and taught nothing.

### 3.2 Determinism requirements

Determinism is a property of the whole path, so every one of these is mandatory:

- Determinization seeds come from `determinization_seed(index)` (`SEED_SALT ^ (index+1) *
  SEED_STRIDE`). No other seed source, no time-derived seed, no OS entropy.
- Candidate order is the engine's `view.legal` offer order, deduplicated on `Action` equality,
  truncated at `max_candidates`. No sorting by score before rollouts, no hash iteration.
- Rollout scheduling is round-robin by seed index, then offer index. A candidate set must get
  its first sample before any candidate gets a second.
- State containers reached from the review path are `BTreeMap`/`Vec`; `HashMap`/`HashSet`
  iteration order MUST NOT influence any value in the payload.
- Scores are `f32` means accumulated in rollout order. Serialised scores MUST be emitted at
  fixed precision (`{mean:+.2}` in text, the same rounded decimal in JSON) so that two
  legitimate runs cannot differ in trailing digits.
- No wall-clock, thread-id, address, PID, locale or environment value may enter the payload.
  Timestamps belong to the evidence log, never to the ranking.
- Tie-breaking is a total order (§5.3) and MUST NOT depend on collection iteration order.

### 3.3 Stop reason and the reproducibility flag

`search` stops for exactly one of four reasons. The implementation MUST record **which**, in
this check order (cancel, wall clock, node budget — matching the existing loop), rather than
collapsing them into one boolean:

```rust
pub enum StopReason {
    /// Every determinization round ran for every candidate.
    DeterminizationsExhausted,
    /// The node budget was reached.
    NodeBudget,
    /// The wall clock was reached.
    WallClock,
    /// A SearchCancellation was observed.
    Cancelled,
    /// The decision point was refused before any rollout ran.
    NotStarted,
}
```

Rules:

- An analysis is **reproducible** iff `stop ∈ {DeterminizationsExhausted, NodeBudget}` **and**
  it was not cancelled. A wall-clock or cancellation stop is `reproducible: false`, and the
  panel MUST say so in plain words and MUST exclude that analysis from recorded evidence.
- A validation or corpus run MUST set `max_millis = u64::MAX`, so only node/round bounds can
  stop it. The interactive panel MAY keep a wall-clock bound for latency; when the clock fires
  it reports `reproducible: false` instead of pretending the sample is complete.
- Reproducibility is a claim about **this budget on this input**, not about the quality of the
  result. Both a reproducible and a non-reproducible analysis MUST carry the same honesty
  labels about sample size.

### 3.4 Budget record shape

Every review carries its budget, serialised with the payload:

```rust
pub struct BudgetRecord {
    pub seeds: usize,
    pub node_budget: usize,
    pub max_rollout_nodes: usize,
    pub max_millis: u64,          // u64::MAX serialised as the string "unbounded"
    pub max_candidates: usize,
    pub decisions_used: usize,    // engine decision points actually consumed
    pub stop: StopReason,
    pub cancelled: bool,
    pub reproducible: bool,
    pub fidelity: String,         // the search's own determinization disclosure, verbatim
}
```

The rendered form is one line, generated from the record (never hand-written):

```
budget: 2 determinization seed(s) · 30000 engine decisions total · 6000 decisions per rollout ·
8000 ms wall clock · at most 6 engine-offered candidates · used 5387 · stopped: node budget ·
reproducible: yes
```

Requirement: the rendered budget line MUST be a pure function of `BudgetRecord`. A test asserts
that mutating any field changes the line, and that the line never contains `%`.

---

## 4. Ranking contract

Module: `crates/ai/src/review.rs` (re-exported as `aperture_ai::review`). Pure data and pure
derivation: `rank` takes the masked view, the analysis and the budget; it performs no engine
call and no I/O.

### 4.1 Payload

```rust
pub struct RankedReview {
    pub spec: &'static str,              // "move-review/1.2"
    pub binding: ReviewBinding,
    pub budget: BudgetRecord,
    /// Empty exactly when `refusal` is Some.
    pub candidates: Vec<RankedCandidate>,
    pub preference: Preference,
    pub refusal: Option<String>,         // the search's typed reason, verbatim
    pub notes: Vec<ReviewText>,          // provenance/uncertainty sentences, kind-labelled
}

pub struct ReviewBinding {
    pub seat: PlayerId,
    pub generation: u64,     // session generation
    pub revision: u64,       // engine revision
    pub decision: u64,       // engine decision serial
    pub view_digest: String, // masked-projection digest the analysis ran on
    pub rules_revision: String,
}

pub struct RankedCandidate {
    /// Index into the engine's offer order. The identity of a candidate is this
    /// index plus `move`; it is never a re-derived guess.
    pub offer_index: usize,
    pub move_label: String,      // the engine's own label for the offered action
    pub offer_rules: Vec<&'static str>, // LegalAction::rules, verbatim
    pub score: Option<f32>,      // practice-outcome mean, rounded to 2 decimals
    pub scored_samples: usize,
    pub without_result: usize,   // rollouts that reached no result
    pub refused_times: usize,    // search world refused this root action
    pub rollouts_attempted: usize, // scored + without_result + refused_times
    pub line: Option<CandidateLine>,
    pub reasons: Vec<ReasonTag>,
    pub uncertainty: Uncertainty,
    pub chosen: bool,            // the search's own selection; not a verdict
}
```

**Depth and per-candidate cost.** This search has **no depth metric**: it is a rollout ensemble over
determinizations, not a fixed-depth tree search, and every candidate is evaluated by the same
round-robin schedule with the same per-rollout cap. Therefore:

- `depth` is **not** a field of the payload and MUST NOT be reported as a number, in text or in
  JSON. Writing "depth 4" or "evaluated 4 plies" would be a fabricated measurement.
- The per-candidate cost that genuinely exists is `rollouts_attempted` (derived, not stored
  independently) with its breakdown `scored_samples` / `without_result` / `refused_times`, and the
  line length `line.steps.len()` when a line exists.
- The global cost is `BudgetRecord.decisions_used` out of `BudgetRecord.node_budget`; that is the
  only "work done" figure the panel prints.
- If a future search gains a real depth or selective-depth control, it MUST arrive with a spec
  amendment naming the field, its unit and its meaning before any UI reports it.

`RankedReview` and every nested type implement `Serialize` + `Deserialize` + `PartialEq`, so
the corpus harness (t_baa0eff8) can round-trip fixtures and the determinism test can compare
bytes.

### 4.2 Ranking order

Total order over `candidates`, applied in this sequence, and only over candidates with a scored
sample:

1. `score` descending;
2. `chosen` first (the search's own selection never contradicts the preference line);
3. `scored_samples` descending;
4. `offer_index` ascending.

Candidates with no scored sample are listed separately, in `offer_index` order, and MUST NOT be
interleaved into the ordering — a zero-sample row is not a ranking position.

### 4.3 Uncertainty representation

```rust
pub enum Uncertainty {
    /// Two or more samples and the mean is at a lattice distance from every other
    /// scored row (§4.5). Ordering is claimed.
    Ordered,
    /// At the budget's resolution this row is not separable from the row it is
    /// compared with. Ordering is NOT claimed.
    EffectivelyTied { with: Vec<usize> }, // offer_index values
    /// `scored_samples < 2`: one sample cannot establish an order.
    SingleSample,
    /// No rollout reached a result. Not ranked at all.
    Unscored,
}
```

- `EffectivelyTied` is symmetric: if `a` is tied with `b`, `b` is tied with `a`.
- `SingleSample` is a distinct state from `EffectivelyTied` because the reason differs (an
  insufficient sample versus an inseparable pair) and the reader is told which.
- There is no confidence field and no probability field, anywhere, in any form. A caller that
  wants one is out of scope for this feature.

### 4.4 Candidate line (principal variation)

A line is a recorded rollout, not a search artefact invented afterwards.

```rust
pub struct CandidateLine {
    /// Engine-submitted actions from the recorded rollout, root first.
    pub steps: Vec<LineStep>,
    /// The rollout went further than MAX_LINE_PLIES; the line is a prefix.
    pub truncated: bool,
    /// Determinization seed index that produced this line.
    pub seed_index: u64,
    /// The recorded rollout's outcome value (+1/0/−1).
    pub outcome: i8,
    /// Explicit statement that the line belongs to the search world, not the
    /// real game.
    pub scope: LineScope, // always SearchGame
}

pub struct LineStep {
    pub decision: usize, // decision ordinal inside that rollout
    pub seat: PlayerId,
    pub label: String,   // the label of the action the engine accepted
}
```

Production rules:

1. `search` records, for the rollout currently running, the first `MAX_LINE_PLIES = 12`
   engine-accepted actions as `(seat, label)` pairs. Recording must not change search
   behaviour: no extra engine calls, no re-ordering, no re-decision.
2. A candidate's line is the recording of the **first rollout in scheduling order** that
   produced a scored result with the candidate's best value so far (strictly greater replaces;
   equal does not). Because scheduling is deterministic (§3.2), this is deterministic.
3. A candidate with `scored_samples == 0` has `line: None`. Never a line from a capped,
   truncated or refused rollout.
4. Whenever `line` is `None`, the panel states that no line is available and why
   (`no rollout reached a result` / `search exposes no line`). It never synthesises, extrapolates
   or concatenates one.
5. `LineScope::SearchGame` MUST be shown: the line happened in the reconstructed world under
   the disclosed determinization, so it is not a prediction about the real game's hidden cards.

### 4.5 Tie resolution (the only uncertainty arithmetic permitted)

Means are `s/k` for integer `s ∈ [−k, +k]` and `k = scored_samples`; the achievable values lie
on a lattice of step `1/k`.

- `MIN_ORDERING_SAMPLES = 2`.
- Two scored rows are **separable** iff `min(k_a, k_b) >= 2` **and**
  `|mean_a − mean_b| >= 1 / min(k_a, k_b) − ε` with `ε = 1e-6` for `f32` rounding.
- Otherwise both are `EffectivelyTied` (or `SingleSample` if either has `k < 2`).
- Exact equality always yields `EffectivelyTied`, and the pair is displayed as a group.

This is a combinatorial statement ("at this sample count, this difference is one lattice step"),
not a statistical confidence claim. Adding a confidence interval is an explicit non-goal for
v1.0 and is listed in the open questions.

### 4.6 Reason tags (closed registry)

A reason is a **closed enum**, not a string. Text is produced by one formatter from
`(tag, counts, evidence)`. Adding a tag is a spec change and a guardrail change (§6).

| tag | required evidence | permitted text |
| --- | --- | --- |
| `HighestPracticeOutcome` | `SearchCount` with `scored_samples >= 2` | "highest mean practice outcome of the offered alternatives (+X.XX over N scored samples)" |
| `EqualPracticeOutcome` | `SearchCount` for both rows | "level with another offered alternative at this sample count (+X.XX)" |
| `NoScoredSample` | `SearchCount{scored: 0}` | "no scored sample: no rollout reached a result, so this alternative is not ranked" |
| `IncompleteRollouts` | `SearchCount{without_result > 0}` | "N rollout(s) ended without a result and are counted as neither a win nor a loss" |
| `RefusedBySearchWorld` | `SearchCount{refused > 0}` | "the search's own reconstruction refused this action N time(s) as illegal" |
| `LowSample` | `SearchCount{scored < 2}` | "one sample cannot establish an order" |
| `BudgetStopped` | `BudgetRecord.stop ∈ {WallClock, Cancelled}` | "the analysis stopped at the wall clock/cancellation, so this sample is incomplete" |
| `SoleLegalOffer` | `EngineOffer` with exactly one entry | "the engine offers exactly one action at this decision point" |
| `EngineRuleApplies { rule, topic }` | `CoachExplanation{kind: Rule}` **and** `RuleCitation{number}` | "[RULE] <topic>: <engine text> (CR <number>)" |

Explicitly **unavailable** tags — no current engine trace establishes them, so they MUST NOT be
emitted until one does: hanging piece, tempo, card advantage, board development, mana
efficiency, "punishes a mistake", "sets up next turn". Positional vocabulary requires a new
engine trace, and a new trace requires its own card.

### 4.7 Evidence references

```rust
pub enum EvidenceRef {
    EngineOffer { index: usize },
    EngineLabel { index: usize },
    SearchCount { offer_index: usize, scored: usize, without_result: usize, refused: usize },
    CoachExplanation { index: usize, kind: ExplanationKind, topic: String },
    RuleCitation { number: &'static str },
    Budget { field: &'static str },
    LineStep { offer_index: usize, decision: usize },
    ViewDigest,
}
```

Every `ReviewText` and every candidate sentence MUST carry at least one `EvidenceRef` that
resolves inside the same payload. "Resolves" means: the referenced index exists, the referenced
count equals the field it claims to describe, the rule number appears in the source
`LegalAction::rules` or `Explanation::rules`, and the line step exists in that candidate's
recorded line.

### 4.8 Text kinds

```rust
pub struct ReviewText { pub kind: TextKind, pub body: String, pub evidence: Vec<EvidenceRef> }

pub enum TextKind {
    Rule,           // engine rule/simplification statement, CR-cited
    Policy,         // search preference / practice outcome statement
    Uncertainty,    // sample size, tie, truncation statements
    Provenance,     // budget, binding, fidelity, read-only statement
}
```

The UI maps `TextKind` to a section and a badge; the mapping is total and one-to-one
(`Rule → rules section`, everything else → policy section). The ranking layer never decides
presentation, and the UI never re-classifies a text kind.

### 4.9 Implementation resolutions (informative — added in 1.2, task t_f4baa891)

Task t_f4baa891 built this contract in `crates/ai/src/review.rs`. Three places needed a decision
the note left open, or stated something that cannot compile. None of them changes a decision
(D-1…D-11) or a criterion (AC-1…AC-17); each is recorded here per §10.

1. **`preference` is `Option<Preference>`.** AC-9 requires `preference: None` for a refused
   decision point, which the §4.1 sketch's bare `Preference` cannot express. `None` means exactly
   "no ordering is claimed": a refusal, or no scored sample at all. A scored but single-sampled
   head yields `Preference::NoOrder { chosen }`, because one sample establishes no order (§4.5).
   `Preference` is derived from the row the ranking places first, so the preference line can
   never contradict the order above it.
2. **Static strings are `Cow<'static, str>`.** §4.1 also requires `Deserialize`, and a
   `&'static str` cannot be produced from JSON. `spec`, `offer_rules` and rule numbers are `Cow`,
   so the payload round-trips while still borrowing where the value already is static. The
   serialised shape is unchanged: a JSON string.
3. **`BudgetRecord::rendered` excludes `fidelity`.** §3.4 asks for a line that is a pure function
   of the record, while §5.2 requires the panel to print the search's fidelity line **verbatim**
   on its own line. `rendered` therefore renders every other field, appends
   "cancellation observed" when `cancelled` is set, and leaves `fidelity` to the §5.2 line.

---

## 5. Presentation contract

Existing implementation to extend rather than replace: `crates/ui/src/review.rs` on the #28
branch (`POLICY_TITLE`, `POLICY_SOURCE`, `RULES_TITLE`, `RULES_SOURCE`, `ReviewSection`,
`MoveReview::policy_section`, `rules_section`). Its four structural properties are retained and
are restated here as normative.

### 5.1 Two sections, two constructors, no crossing

- `rules_section(&[Explanation])` is the **only** producer of rule text. It keeps
  `ExplanationKind::Rule` and `ExplanationKind::Simplification` (badged `[RULE]` /
  `[SIMPLIFICATION]`) and **skips** `ExplanationKind::Strategy` — a strategy record is a policy
  statement and MUST NOT appear as a rule.
- `policy_section()` is the **only** producer of ranked preference text. It contains no CR
  citation and no rule sentence.
- Titles and source labels are distinct constants and MUST NOT be merged into one block, one
  heading, or one colour. A test asserts the two sections have different titles, different
  source labels, and that no line from one constructor appears in the other's output.
- The rules section renders for an offer even when the analysis refused: rule availability does
  not depend on search availability.

### 5.2 Uncertainty display

- Every ranked row prints its mean **and** its sample count. A mean alone is a false
  impression.
- Tied rows print as one group: "effectively tied at +1.00 (2 scored samples each)" with the
  member labels; the group is not ordered internally.
- Rows with one sample print "one scored sample — not ordered against other rows".
- Unscored offers print in a separate list titled "unscored offers (no rollout reached a
  result; not ranked)".
- The analysis prints: the budget line (§3.4), the stopping rule, the reproducibility flag, and
  the search's fidelity line verbatim.
- No bar, no gauge, no percentage, no "confidence" indicator, no colour gradient implying a
  probability. Uncertainty is expressed in words and counts only.

### 5.3 Verdict-free language

Required phrasings and their forbidden opposites:

| required | forbidden |
| --- | --- |
| "the search's own choice" (on a unique maximum) | "best move", "correct move" |
| "the search's own choice among equally scored offers" (only on a real tie) | the same phrase on a unique maximum |
| "labels no alternative as an error" (standing note) | "blunder", "mistake", "inaccuracy" |
| "mean practice outcome" | "win probability", "chance", "winrate" |

Regression to preserve: on a **unique** maximum the panel MUST NOT say "among equally scored
offers". That defect was found and fixed on the #28 branch; the unique-maximum test is
retained under this spec.

### 5.4 Refusal, zero-sample and single-candidate degradation

- Refusal (typed `SearchUnavailable`): the panel carries the reason verbatim and ranks nothing.
  No empty table, no placeholder score, no dash that reads as zero.
- Zero scored samples: no ordering; the panel says so and names how many offers were tried.
- Exactly one offered action: `SoleLegalOffer` reason, the single row shown, and no ordering
  language ("not a preference — the engine offers only this action"). Note the engine's own
  `DecisionSource::Forced` label exists for the *decision* path; the review still must not
  editorialise about it.
- Cancelled or stale review: discarded with a stated reason (§5.5), never shown as current.

### 5.5 Binding, staleness and read-only semantics

- A review is bound to `(seat, generation, revision, decision, view_digest)`.
- The panel MUST discard a result whose binding no longer matches the live session on any of
  those five fields, cancel the running worker, and show that it discarded it (no silent drop).
  The digest comparison catches the case where a different game reproduces the same
  seat/revision/decision triple.
- Applying a review MUST NOT change the engine digest, revision, decision serial, priority or
  pending state. A test asserts all five are unchanged after a review is produced and rendered.

### 5.6 Accessibility (inherits #22 rules)

Uncertainty and section separation MUST NOT be encoded by colour alone; every section carries a
text title, every badge is a word, and the panel must remain legible at 70–200% text scale
within the enforced minimum window size. A section that only differs by hue is a defect.

---

## 6. Honesty guardrails (shared module)

Module: `crates/ai/src/review/honesty.rs`, re-exported as
`aperture_ai::review::honesty`. It is independent of the UI and of the search, and it is
CI-checkable.

```rust
pub fn audit_text(body: &str) -> Vec<HonestyViolation>;
pub fn audit_review(review: &RankedReview) -> Vec<HonestyViolation>;

pub enum HonestyViolation {
    PercentFigure { sentence: String },
    ProbabilityClaim { sentence: String },
    VerdictLabel { word: &'static str, sentence: String },
    UntraceableClaim { sentence: String },
    UnregisteredReason { tag: String },
    UnresolvableEvidence { index: usize },
    UnlabelledTextKind { index: usize },
    RuleTextInPolicySection { index: usize },
    PolicyTextInRulesSection { index: usize },
    MissingPublicDistinction, // rules and policy collapsed into one section
}
```

Audit rules (exact, so the implementation cannot drift):

1. Split on sentence boundaries `[.!?;]`.
2. **Percent figures** — any `\d+(\.\d+)?\s*%` in review text is a violation with **no**
   negation excuse. Numbers in prose about outcomes are forbidden outright.
3. **Probability claims** — `probabilit`, `chance of winning`, `winrate`, `win rate`, `odds`,
   `equity`, `expected score`, `centipawn`, `elo`, `tablebase`, `likely wins`.
4. **Verdict labels** — `blunder`, `mistake`, `inaccuracy`, `brilliant`, `best move`,
   `correct move`, `should have`, `forced blunder`, standalone `error`.
5. A sentence containing a rule-3 or rule-4 pattern is a violation **unless** the same sentence
   contains a negation marker from `{no, not, never, none, nothing, neither}`. This is what
   distinguishes the required disclaimer ("it labels no alternative as an error") from an
   assertion ("this was a blunder"). Rule 2 has no such excuse.
6. `audit_review` additionally checks: every `ReasonTag` is in the registry (§4.6); every
   `EvidenceRef` resolves (§4.7); every `ReviewText` has a `kind`; no `Rule` text appears in
   `policy_section` output and no `Policy`/`Uncertainty` text in `rules_section` output; scores
   print with sample counts.

Fixture corpus (committed with the guardrail): representative payloads including a known-bad
review whose policy line reads `72% chance this is best`, one labelled `forced blunder`, one
with a fabricated line step, one with an unregistered reason tag. Each MUST produce its specific
violation. The current, honest payload corpus MUST produce zero violations.

Adding a reason tag requires, in one commit: a row in §4.6, the enum variant, its formatter, its
evidence requirement, and a positive + negative guardrail fixture. The module documents this
procedure in its header so the next author does not invent a shortcut.

---

## 7. Acceptance criteria

Every later #28 task MUST satisfy the criteria it owns; the integration task verifies all of
them on the exact published head. A criterion without a passing, committed check is unmet.

| id | criterion | owner |
| --- | --- | --- |
| AC-1 | **No hidden reads.** Two engine states with identical masked views but different hidden contents yield byte-identical review payloads (§2.3). | ranking |
| AC-2 | **No fabricated probabilities.** No percentage, probability, odds, equity or centipawn figure can appear in any review text; guardrail fixture `72% chance this is best` fails. | guardrails |
| AC-3 | **No blunder verdicts.** `blunder`/`mistake`/`inaccuracy`/`error`/`best move` never appear as assertions; negation-only disclaimers are allowed. Fixture fails. | guardrails |
| AC-4 | **Every claim traceable.** Every sentence in a review payload carries ≥1 resolving `EvidenceRef`; an unresolvable reference is a test failure. | ranking + guardrails |
| AC-5 | **Byte-reproducible ranking.** Same view + same node-bounded budget + `max_millis = u64::MAX` ⇒ byte-identical serialised payload, asserted by running the search and ranking twice and comparing bytes, and once more out-of-process in the evidence harness. | ranking |
| AC-6 | **Determinism is not claimed for truncated analyses.** A wall-clock or cancelled stop sets `reproducible: false`, is labelled in the panel, and is refused by the evidence harness. | ranking + presentation |
| AC-7 | **Uncertainty is explicit.** Ties at the lattice resolution produce `EffectivelyTied` for both rows and a grouped display; one-sample rows are `SingleSample`; unscored rows are unranked. | ranking + presentation |
| AC-8 | **Unique maximum is not called a tie.** Retained regression: the "among equally scored offers" phrase appears only on a real tie. | presentation |
| AC-9 | **Refusal ranks nothing.** A typed `SearchUnavailable` yields `preference: None`, zero candidates, reason verbatim, and no empty table. | ranking + presentation |
| AC-10 | **Zero samples rank nothing.** No ordering, no placeholder mean, explicit count of attempts. | ranking + presentation |
| AC-11 | **Rules and policy are structurally separate.** Distinct constructors, badges, titles and source labels; `ExplanationKind::Strategy` never reaches the rules section; asserted on rendered output, not on intent. | presentation + guardrails |
| AC-12 | **Read-only.** Digest, revision, decision serial, priority and pending state are unchanged by producing and rendering a review; no action is submitted, no priority granted. | presentation |
| AC-13 | **Budget is recorded and printed.** Payload carries `BudgetRecord` (all fields, `stop`, `reproducible`); panel prints it; the printed line is a pure function of the record. | ranking + presentation |
| AC-14 | **Staleness discard.** A review bound to a superseded generation/revision/decision/seat/digest is discarded with a stated reason and its worker cancelled. | presentation |
| AC-15 | **Lines are engine-submitted only.** Every `LineStep` corresponds to an action the engine accepted in that rollout; a candidate with no scored sample has no line; missing lines are disclosed. | ranking + presentation |
| AC-16 | **Corpus validation.** The reviewed corpus run reports per case: ranked alternatives, uncertainty displayed, reason tags, and a false-certainty count of **zero**; determinism rerun matches; provenance (commit + tree + budget) recorded. | validation |
| AC-17 | **No invented measurements.** No `depth`, plies-evaluated or nodes-per-option figure appears anywhere; per-candidate cost is the real rollout breakdown and global cost is `decisions_used`. | ranking + presentation |

Acceptance gate for the whole feature (root card): AC-1…AC-17 green on the published head, plus
`full_issue28_accepted` only after the corpus exists (see open question Q7).

---

## 8. Validation protocol (for the corpus task)

1. **Corpus.** A reviewed set of decision points with human-reviewed expectations: for each
   case, the position, the reviewing seat, the engine offer, the expected practical reason
   categories, and whether any alternative should be reported as effectively tied. Reviewers
   record disagreement, not just agreement.
2. **Run.** Fixed budget with `max_millis = u64::MAX`; one recorded command line per case; the
   payload written as JSON next to the log.
3. **Checks per case.** (a) every ranked move is in the engine's offer; (b) every line step is
   an accepted engine action; (c) uncertainty state matches the sample counts under §4.5;
   (d) reason tags match the reviewed expectation; (e) guardrail audit returns zero violations.
4. **Counts.** Report `false_certainty_occurrences` (percentages, probabilities, verdicts) and
   `untraceable_claims`; both MUST be `0`. Also report `unscored_candidate_count`,
   `tied_pair_count` and `wall_clock_truncated_count` (the last MUST be `0` for recorded runs).
5. **Determinism rerun.** Re-run the whole corpus from a clean tree at the same commit and
   require byte-identical payloads. A single differing byte is a failure to be explained, not
   tolerated.
6. **Provenance pin.** The report names the commit, the tree, the budget and the exact commands.
   A report whose numbers cannot be re-derived from its own commands is not evidence.

---

## 9. Open questions

These are unresolved at spec 1.2. They do not block the child tasks that build the contract
above, but the root acceptance decision needs answers.

- **Q1 — interactive latency vs reproducibility.** The panel keeps `max_millis = 8000`, so a slow
  machine may produce a non-reproducible analysis that is still useful. Is that acceptable in
  the default UI, or should the interactive path also be node-only (potentially seconds of
  freeze-free but longer background work)?
- **Q2 — line recording cost.** D-6/§4.4 requires a change inside `aperture_ai::search` to record
  rollout traces. The cost is small in memory but it touches the module #24 already shipped and
  reviewed. If the ranking task declines it, criterion 2 of #28 (candidate lines) cannot be met
  and must be re-scoped rather than simulated.
- **Q3 — uncertainty strength.** The lattice rule (§4.5) is exact and cheap but crude: two
  2-sample means of `+1.00` versus `+0.50` are treated as separable. A proper interval over
  `{−1,0,+1}` outcomes is possible, but any presentation must avoid reading as a probability.
  Decide before the corpus claims anything about teaching quality.
- **Q4 — earlier plies.** Review of the current decision point only. Retroactive review needs
  engine snapshot/replay support plus the staleness model extended to historical revisions, and
  is out of scope for #28.
- **Q5 — positional vocabulary.** Tags such as "hanging piece" or "tempo" need an engine trace
  that establishes them. Today none exists, so the registry emits counts and rule citations
  only. Should a bounded threat trace be a separate roadmap item?
- **Q6 — payload serialisation surface.** The corpus harness needs the payload as JSON. Confirm
  `RankedReview` is `Serialize`/`Deserialize` and stable enough to be a public artefact (schema
  file plus a compatibility note), or keep it an internal evidence format only.
- **Q7 — corpus availability.** Criterion 4 needs #17's certified starter lists; 1 of 8 was
  certified at the time this spec was written. The corpus task cannot be attempted before that
  lands, and the root card stays a dependency wait until then.
- **Q8 — where the guardrail lives.** `aperture_ai::review::honesty` lets the UI and the evidence
  harness share one implementation. The alternative (a repo-level copy-scanning check over UI
  string constants) catches copy that never flows through the payload. Both may be wanted; only
  the module is specified here.

---

## 10. Change control

Amendments are commits to this file that (a) bump the spec version, (b) state which decision
(D-1…D-10) or criterion changes, and (c) name the task that motivated it. A child task that
implemented a different contract is not retroactively correct because its tests pass — it is a
deviation, and the deviation is recorded here.
