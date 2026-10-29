# Slice 64 — Goblin Sharpshooter, Krenko/Tin Street Kingpin, Hobgoblin Bandit Lord, Goblin Matron (+ review-363 record corrections)

Same draft PR46, same branch `aperture/issue-17`, same writer history. **Original
issue #17 is still unfinished: 2 of 8 lists certified.** This handoff asks for one
independent practical review of the increment, not for task completion, not for a
dependent unlock.

## What this slice is

Review 363 asked for the Krenko demanded cards to be implemented as a *group that
shares real primitives*, and for `R363-1..3` to ride in the opening commit of the
next product slice. Both were done in one run:

* **Opening record commit `053e13c`** — `R363-1`: `docs/issue17-starter-corpus.md`
  now ends the union bullet at **172** and carries the slice 59/60/62/63 deltas
  (including the out-of-list Volcanic Dragon, which is supported but is not one of
  the 459 union names). `R363-2`: the same document no longer calls Jasmine the
  only certified starter; it names both certified lists. `R363-3`: the token
  roadmap said Krenko had 46 missing names (43 as audited) and
  `slice63/TDD.md` claimed a final targeted count of 8 while the shipped binary
  runs 10; both corrected.
* **Product commit `c43c389`** — four sourced Goblin cards that share four
  printed primitives:
  1. **CR 502.3 standing untap restriction** — `Goblin Sharpshooter`'s first
     printed line is not the one-shot “doesn't untap during your next untap step”
     shape (Spectral Bears) but a standing static, so it is
     `StaticAbility::SelfDoesNotUntapDuringUntapStep`, honoured in
     `perform_untap` on every one of its controller's untap steps. The second line
     (`Effect::UntapSource`, CR 701.26a) untaps it directly and is deliberately
     *not* stopped by the restriction.
  2. **Tokens scaled to the source's own effective power at resolution** —
     `Krenko, Tin Street Kingpin` places its +1/+1 counter first and reads its
     power afterwards (`Effect::PutPlusOneCounterOnSource` then
     `Effect::CreateGoblinTokensEqualToSourcePower`), so the counter is already
     counted and a Goblin anthem raises the count again.
  3. **A per-seat Goblin-entry history for the turn** —
     `GameState::goblins_entered_this_turn`, grown on first use, cleared when the
     turn number advances, and read at resolution for `Hobgoblin Bandit Lord`'s
     damage. Tokens count (CR 111.1: a token entering is a permanent entering);
     Goblins that were already on the battlefield do not.
  4. **A subtype-filtered library search** — `LibrarySearchMode::GoblinCardToHand`
     behind the existing resumable search pending: the eligible set is every
     Goblin card in that player's library, the found card is revealed as it moves
     to hand, and the search ends with a shuffle (CR 701.23). `Goblin Matron`'s
     “you may” is answered on the stack *before* anything is searched, so
     declining neither reveals nor shuffles.
* **Fix commit `81e82f9`** — two hard-coded pool-size pins still said 172
  (`issue13_commander_combat`, `issue17_slice15_corpus`). CI caught them on
  `c43c389`; both now say 176 and the first records the per-slice chain.

`Effect::Damage`'s printed damage loop was factored into
`Game::deal_effect_damage`, so the fixed and computed amounts share one code path
and one set of events.

## Sources (authoritative printed text, with retrieval date)

Fetched live 2026-10-29 into `evidence/issue17/slice64/sources/` and recorded in
`fixtures/starter-corpus/slice64-sourced.json`; every card was checked across
*all* its printings and each has exactly one distinct Oracle text.

| card | printing | cost | type | P/T | printings |
|---|---|---|---|---|---|
| Goblin Sharpshooter | c13 111 | {2}{R} | Creature — Goblin | 1/1 | 6 |
| Krenko, Tin Street Kingpin | moc 287 | {2}{R} | Legendary Creature — Goblin | 1/2 | 9 |
| Hobgoblin Bandit Lord | afr 147 | {1}{R}{R} | Creature — Goblin Rogue | 2/3 | 6 |
| Goblin Matron | plst DMR-123 | {2}{R} | Creature — Goblin | 1/1 | 17 |

A fifth field was checked and deliberately not implemented: the *current*
Hobgoblin Bandit Lord text counts “Goblins that entered the battlefield under your
control this turn”, not “Goblins you control” — the older wording found in memory
of the card. The catalog carries the current text.

## Corpora and gaps

`evidence/issue17/slice64/corpus-audit.py` → `AUDIT OK` (log beside it):

```
catalog_cards = 176
union_names = 459, supported_union = 162, missing_union = 297
per_list_missing = [55, 0, 51, 39, 0, 52, 52, 53]
certified_lists = 2 (jasmine-vanilla-stompy, ayula-bear-counters)
```

Krenko's `missing_cards` (43 → 39) **and** `missing_interactions` were rewritten
whole in the same change; the interaction text now names the still-missing
families (sacrifice-a-Goblin costs, search-onto-battlefield, goblin-count combat
payoffs, artifact/graveyard recursion, non-Goblin token packages) instead of
describing the batch that now exists.

## Gates actually run here

* `cargo fmt --all --check` → 0.
* `cargo clippy --workspace --all-targets --locked -- -D warnings` → 0.
* Focused suites: `issue17_slice64` **11 passed / 0 failed**, `card_printings`
  **23/0**, `issue13_commander_combat` **43/0**, `issue17_slice15_corpus` **2/0**.
* Whole workspace `cargo test --workspace --all-targets --locked` — see
  `workspace-all-targets.log` in this directory; the figure recorded there is the
  run at the final code, not an earlier head.
* A **real four-seat 40-life journey** (evidence-only masked-action demonstrator,
  `decks/package-slice64.txt` in all four seats, Jasmine Boreal injected as every
  commander — **not** a 100-card starter, not product AI, not a strength claim):
  terminal **Win{P3}**, 1325 actions / 27 turns / **3498 events**, digest
  `a0196628d36d…`. All four new cards were cast (Matron 2, Krenko 7, Bandit Lord
  6, Sharpshooter 3), 2 Goblin Matron searches revealed a Goblin, 16 ability
  activations, 42 Goblin tokens entered, 17 attack declarations, 68 combat-damage
  events. The save round-trips (`SessionSave` capture → load → identical digest and
  state, `sandbox_modified = false`), and the **separate shipped CLI process**
  (`aperture-cli replay --file …/demo-replay.json`) reproduces the digest:
  `replayed digest a0196628d36d | matches recorded | events 3498`, exit 0
  (`replay.log`).
* Exact-head CI on the code head — see the CI section below.

## Disclosures

* Original #17 is **unfinished**: 2 of 8 certified, six planned lists with real
  gaps `[55, 51, 39, 52, 52, 53]`.
* The native writer authored this increment and is **not** its independent
  reviewer.
* `goblins_entered_this_turn` is empty until the first Goblin entry of a turn, so
  saves and replays from before this slice keep byte-identical canonical JSON —
  but **any replay or save written mid-turn after a Goblin entered** now carries
  the field, which re-digests that state. No broad backward-continuity claim is
  made.
* The Bandit Lord's count is per *entered permanent*, so a Goblin that entered and
  then died still counts, and a token counts like a card. Control-change edge
  cases (a Goblin entering under one seat and changing controller the same turn)
  are not modelled beyond the entry event, because no pool card does it.
* `Goblin Matron`'s reveal is recorded as an event, not as a separate “revealed”
  zone flag; the card moves through the ordinary library→hand path.
* The damage from a zero Goblin-entry count deals nothing at all (no event), which
  is the CR 120.4a reading; the ability still costs `{R}, {T}`.
* Krenko is a **legendary** creature and the general legend rule (CR 704.5j) is
  implemented and reachable here — unlike the certificate exclusions corrected in
  slice 61, this slice does not claim otherwise.
* The journey is a 40-card package: it proves the new cards are playable end to
  end and that the search and untap paths run inside a real four-seat game with a
  matching separate replay. It is **not** a certificate for the Krenko starter
  (39 names still unsupported) and does not move “2 of 8”.
* No completion, merge, master push, closure, dependent unlock, new team or global
  config change was made.

## What the reviewer should try to break

1. **Standing vs one-shot untap.** Tap Sharpshooter, run two of its controller's
   untap steps: it must stay tapped in both, an ordinary creature beside it must
   untap in both, and `skip_next_untap` must stay empty. Then kill a creature and
   check it untaps *and* can ping again the same turn. Slice 19's one-shot shape
   must not be what this is.
2. **Krenko's order.** Attack with Krenko alone → one +1/+1 counter and 2 tokens;
   with one Goblin Chieftain → 3 tokens. A version that reads printed power, or
   reads power before the counter, gives 1/2/2 respectively — try to construct
   that and see the shipped tests fail.
3. **Bandit Lord's history.** No entry this turn → activation deals nothing (not
   “0 damage”); after Goblin Instigator (card + token) → exactly 2 damage, not the
   board's Goblin count; a Goblin that entered and died still counts; the history
   is empty again after the turn advances.
4. **Goblin Matron's search.** Only Goblin cards offered; a forged non-Goblin
   `SearchLibraryPick` refused; declining the “may” leaves the library byte-equal
   and emits no shuffle; accepting reveals the found card and shuffles.
5. **Printing conformance.** The four catalog strings against the live Scryfall
   printings recorded in `slice64-sourced.json` and the `PRINTINGS` table.
6. **Pool pins.** The 176 counts are the real compiled pool, not a number typed to
   match a test.

## Next

Same writer/history/card/branch/draft PR46 continues the Krenko starter (39
missing names), starting with the package that shares the *next* primitive rather
than one card per slice — the strongest candidate is the sacrifice-a-Goblin cost
family (Skirk Prospector, Siege-Gang Commander, Goblin Trashmaster, Pashalik
Mons), which would also give the six-card token package the three-token enters
ability it still lacks. “3 of 8” will only be claimed on a real exact-100-card
certificate for the next list.
