# #34 D1 — cleanup-discard deadlock: handoff

Card: native Kanban `t_94c9841e` (#34, roadmap key `d1-cleanup-discard`)
Branch: `aperture/d1-discard-fix` · worktree `C:/AI/aperture-worktrees/issue-34`
Base: master `8de0961` · public issue: https://github.com/wulfenbach-jpg/the-aperture/issues/54

## Rule text (pinned revision)

Repository pin: `RULES_REVISION = "2026-09-25"`
(`crates/core/src/lib.rs`), rules text read from
`C:/AI/aperture-worktrees/rules-run162/MagicCompRules-20260925.txt`
retrieved for this session (official *Magic: The Gathering* Comprehensive Rules,
effective 2026-09-25).

> **514.1.** First, if the active player's hand contains more cards than their
> maximum hand size (normally seven), they discard enough cards to reduce their
> hand size to that number. This turn-based action doesn't use the stack.

> **402.2.** Each player has a maximum hand size, which is normally seven cards.
> A player may have any number of cards in their hand, but as part of their
> cleanup step, the player must discard excess cards down to the maximum hand
> size.

> **103.5.** … To take a mulligan, a player shuffles the cards in their hand back
> into their library, draws a new hand of cards equal to their starting hand
> size, then puts a number of those cards equal to the number of times that
> player has taken a mulligan on the bottom of their library in any order. …

Both rules leave *which* cards leave the hand to the player. Nothing in either
rule names a default set, so the offer the engine publishes is a UI/AI surface,
not a rules constraint.

## Root cause (verified, not the supervisor hypothesis verbatim)

1. `Game::legal_actions()` returned **exactly one** `LegalAction` for
   `Pending::Discard` and for `Pending::BottomCards`: `sample_discard()` /
   `sample_bottom()`, i.e. the last `count` cards in hand order. The engine's own
   validator (`check_selection`, unchanged) accepted *any* `count` distinct hand
   cards.
2. `HeuristicPolicy` answered with `worst_cards()` — a ranked, different set.
   Both sets are legal, so the engine accepted the heuristic's set whenever it
   was submitted directly (this is why `selfplay` never showed the bug).
3. `UiSession::poll_workers()` compared the finished decision against the live
   offer by equality and, on a mismatch, set `busy_note` and `continue`d without
   applying anything. `ai_busy` was already cleared, so the next frame asked
   again and got the same answer: an infinite re-request loop with the table
   frozen. That is the packaged symptom
   (`discarded an AI decision [illegal_choice] …`, `WAITING: P1: discard …`).
4. The same one-sample offer pattern existed for the London mulligan
   **bottoming** choice (`Pending::BottomCards`), which stalls the same way. The
   new headless repro found that variant on turn 1, before any cleanup discard;
   it is fixed by the same change and is covered by the same tests.

`Action::DeclareAttackers` / `Action::DeclareBlockers` offers are still
under-described (the offer lists "none" and "all eligible", while the policy
builds its own subset). Those answers were silently dropped before this change
and are now engine-validated and applied; the offer itself is unchanged and
remains a separate, disclosed gap.

## What changed

| File | Change |
| --- | --- |
| `crates/core/src/engine.rs` | `Pending::Discard` and `Pending::BottomCards` offers now come from one helper, `hand_choice_offers`: `count == 1` → one action per card in hand; `count > 1` → the historical "last `count` cards" sample first, then a deterministic lexicographic prefix of combinations, capped at the new documented `pub const MAX_DISCARD_OFFER = 12`. Labels carry the card name and its 1-based hand position so two different sets can never share a label. `check_selection` is untouched. `sample_discard`/`sample_bottom` are removed (now dead). |
| `crates/ai/src/heuristic.rs` | Pending-discard and pending-bottom branches now answer with the **offered** set that gives up the worst cards (`offered_hand_set` + `expendability`); `worst_cards` remains only as the fallback for a view built without an offer. |
| `crates/ui/src/session.rs` | `poll_workers` no longer drops a pending answer: an unenumerated completion is submitted (the engine is the legality authority) and, if the engine refuses it, the engine's own first offered action is applied with an `illegal_choice` note naming both. `stale_generation` / `stale_revision` / `stale_decision` / `wrong_seat` are untouched, and the impossible case (engine refuses its own offer) still stops with a diagnostic. |
| `crates/ui/src/app.rs` | While the human owns a discard/bottom choice, the generic legal-action button strip no longer re-lists every offered set — the card-selection panel above is the answer surface. |
| `crates/core/tests/replay_digest.rs` | One existing assertion encoded the bug (`offered.len() == 1` at a count-1 bottoming choice); it now asserts one action per card in hand. |

### Shared-hunk declaration for `crates/ai/src/heuristic.rs` (#17 also edits it)

Touched lines, post-format: `choose()` arms for `PendingKind::BottomCards` and
`PendingKind::Discard` (the two `cards:` expressions now call
`self.offered_hand_set(...)`), the new `offered_hand_set` method (~line 126-160,
between `keep_viewed` and `attackers`), and the new free function
`expendability` next to `worst_cards`. Nothing else in the file changed; the
priority/attackers/blockers logic is untouched.

## Tests (RED first, retained)

New files: `crates/core/tests/issue34_discard.rs` (4 tests),
`crates/ai/tests/issue34_discard.rs` (2 tests),
`crates/ui/tests/issue34_discard.rs` (4 tests) — **+10 `#[test]` functions**,
counted from the diff against the base (`git show 8de0961` has no such files).

RED evidence, captured before the fix, is in `evidence/issue34/RED.log.md`:
the offer held 1 action instead of 8; a multi-card discard offered 1 instead of
a bounded choice; the session reported
`discarded an AI decision [illegal_choice] (not in the live legal offer)` and
never advanced (frozen at `BottomCards` on turn 1 in one seed, at step 327 in
another).

What the new tests assert:

- one offer per hand card for `count == 1`, each citing CR 514.1, distinct sets,
  non-empty labels;
- `count > 1`: ≥ 2 offers, ≤ `MAX_DISCARD_OFFER`, exactly `count` cards each, all
  in hand, no repeats, deterministic across two identical runs;
- `check_selection` still refuses short / over-count / duplicated / foreign /
  nonexistent selections with digest, revision, decision and object count
  unchanged, and the position still completable afterwards;
- the offer never lists the other seat's cards;
- the heuristic's discard answer is one of the engine's offers and the engine
  accepts it (AI-crate);
- the decider envelope exposes one option per offered set, distinct labels, every
  offered key resolves to that offered action, an outside key is refused;
- an all-AI table reaches a terminal result through the real
  request/poll path and the recorded replay re-runs to the same digest and
  outcome;
- the cleanup-discard answer is applied (no `illegal_choice` on a legal
  position, decision moves, a `Discarded` event is recorded);
- the human seat can discard the cards *it* chose (the first cards, not the old
  "last cards" sample), a non-entitled seat is refused with the choice still
  pending, and only the chosen cards leave the hand;
- a misaddressed (`wrong_seat`) answer is still refused without mutating state;
- the existing UI regression for a refused completion was rewritten to the new
  contract: the impossible `Pass` is refused, the engine's legal offer is
  applied, exactly one action is recorded and it is never the refused one.

## Commands and results (this worktree)

```
cargo fmt --check                          -> clean
cargo clippy --all-targets -- -D warnings  -> clean (exit 0)
cargo test --all-targets --no-fail-fast    -> see "Suite" below
cargo build --release                      -> ok
target/release/aperture-cli.exe selfplay … -> see "Real game + replay" below
```

Suite (`cargo test --all-targets --no-fail-fast`, 83 test binaries/sections):
**565 passed, 0 failed, 1 ignored** (`TEST_EXIT=0`).
New tests in the diff: **+10**. `cargo build --release` (`BUILD_EXIT=0`).

### Reproducing the defect / the fix from the CLI

The deadlock lived in the UI session path, which the CLI's `selfplay` driver does
not use (it submits policy actions directly, so the engine always accepted
them). The reproducible headless driver is the session test:

```
cargo test -p aperture-ui --test issue34_discard -- --nocapture
```

It drives `request_ai_decision_default` + `poll_workers` exactly as
`app::update` does, with both seats on AI, and asserts a terminal result plus a
digest-verified replay. Before the fix it failed with the packaged
`illegal_choice` drop.

## Real game + replay

Release binary, both seats on the baseline AI, bundled practice decks:

```
$ target/release/aperture-cli.exe selfplay --seed 30 \
    --out evidence/issue34/selfplay-seed30.json \
    --replay-out evidence/issue34/selfplay-seed30-replay.json
selfplay seed=30 seats=2 heuristic vs heuristic -> status terminal winner P0 in 343 steps (15 turns)
  digest 8f53af86b8f0 | replay verified | casts 10 attacks 6 blocks 0 counters 2
  replay written to evidence/issue34/selfplay-seed30-replay.json
  report written to evidence/issue34/selfplay-seed30.json
```

Report fields: `status: terminal`, `turns: 15`, `steps: 343`,
`replay_steps: 343`, `replay_digest_matches: true`, `replay_error: null`,
`final_digest: 8f53af86b8f06af8c76d34f8f93c8cd70f70604444679acfc3b943559550ca8c`.
The verbose event log for the same seed contains one
`P1 discards a card` cleanup discard, so this run really exercises the changed
mechanic — and it now reaches a result instead of freezing.

Note the division of evidence: `selfplay` drives the `aperture-ai` policy loop
(which never compared against the offer, hence never deadlocked);
`cargo test -p aperture-ui --test issue34_discard` drives the real session
request/poll path, which is where the packaged app froze. Both are reported.

## Unverified / remaining risk

- **Human GUI click-through not exercised here.** The discard/bottom panel is
  covered by a session-level test of the same `submit_human` call, not by a
  native click. The packaged acceptance run belongs to the supervisor.
- **Model-decider variance.** The envelope test proves the offer shape and key
  resolution without an HTTP model. A live Decider-2B run was not performed (the
  service is optional and this worktree does not start it); with a bounded
  12-option offer the model now receives a real multi-way question for a discard
  that it previously answered as "forced".
- **Multi-card offers are a prefix, not the whole space.** `MAX_DISCARD_OFFER =
  12` bounds the offer; a player who wants a set outside the prefix can still
  submit it (the engine accepts any legal set), and the UI's card-selection panel
  is not limited to the prefix. The AI can only choose from the prefix, which is
  a disclosed simplification, not a rules claim.
- **Attackers/blockers offers remain under-described** (see above). They no
  longer stall the table, but the offer does not enumerate subsets.
- No strength, Commander or certification claim is made here. This is a practice
  slice (`Profile::Practice20`, 20 life, bundled 40-card lists).
