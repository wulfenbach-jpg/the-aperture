# Issue17 — slice18 human checkpoint

## Increment, not whole-issue acceptance

Same branch `aperture/issue-17`, draft PR46. Resumed from the approved slice17 head
`76f1b31`. One sourced card, and it is the target starter's commander:

**Ayula, Queen Among Bears** `{1}{G}` — Legendary Creature — Bear 2/2.
Printed: "Whenever another Bear you control enters, choose one — • Put two +1/+1
counters on target Bear. • Target Bear you control fights target creature you
don't control."
Immutable printing `https://api.scryfall.com/cards/c046dfb0-9ca3-4219-a6b0-d7503bc2fbd0`
(mh1 155), retrieved `2026-10-26T23:40:57Z`, recorded in
`fixtures/starter-corpus/slice18-sourced.json`.

Implemented from that text, not from the mechanic name:
- triggers only for **another Bear its controller controls** — Bear tokens trigger,
  Ayula's own entry does not, an opponent's Bear entry does not;
- the mode is chosen **before** targets (CR 603.3c / 700.2b) and a mode with no
  legal full target set is not offered;
- counters mode targets **any Bear**, including an opponent's — the printed first
  mode is deliberately not narrowed to "you control";
- fight mode requires a Bear you control plus a creature you don't control, is
  revalidated at resolution (CR 701.14b: neither creature fights or deals damage if
  either is gone/illegal), and both damage instances go through the existing
  damage pipeline one after the other as a single noncombat pair (CR 701.14a/701.14d).

Registered as a supported commander (command zone, catalog, setup, mana/cast path);
ordinary-library refusal follows the existing supported-commander policy.

## Review, corrections and genuine REDs

Three bounded passes on the **same saved writer history**. The first cut
(`9c3a594`) was **not accepted**; native inspection of the actual diff found and
proved real defects:

1. **BLOCKING regression of approved behavior** — the new `apply_trigger_choice`
   did an unconditional `pending.take()` inside `if let Some(Pending::TriggerMode …)`,
   so an ordinary optional `TriggerChoice` was consumed and dropped: accepting the
   ability no longer resolved it and declining no longer emitted `TriggerDeclined`.
   Proved by running the committed head: `cargo test -p aperture-core --test abilities`
   → `optional_trigger_is_declined_or_used_by_its_controller` FAILED (12 passed, 1
   failed, "declining is a typed event"). Repaired in `79d976d` with one `match` on
   the taken pending; the existing test was kept, not weakened.
2. Ayula's `(false,false)` and `0 => {}` staging paths emitted/parked without
   draining the queued batch, silently losing a second trigger queued in the same
   batch. Repaired; a scratch revert of the drain reproduces the failure
   ("the second trigger must still be placed") and the restored code passes.
3. Fight cited CR 701.12a. The pinned rules text
   (`evidence/issue17/slice2-rules-source.txt`) has Fight at **701.14a**, with
   **701.14b** for the neither-fights case and **701.14d** for "not combat damage".
   Corrected at every site; modal cites are 603.3c / 700.2b.
4. Archived-reference validation covered only `trigger` for `TriggerMode` and
   missed the new `first` anchor on `TriggerTarget`. Both are now traversed.

Coverage added on the same history: Ayula + Soul's Attendant same-batch
preservation; source-left resolution; fight target departure via a real Unsummon;
fight damage prevention through Mending Hands while the other direction still
kills; unavailable fight mode; token-vs-opposing trigger distinction; pending
`TriggerMode` save/reload and half-answered two-target fight save/reload through the
real `SessionSave` loader with matching final state and digest; wrong-seat answer
refused; forged `ChooseTriggerTarget`/`AnswerTriggerChoice` and unoffered targets
refused with pending, decision serial and digest unchanged.

Disclosed deviations: production code preceded the new tests (scaffold-first, **not**
strict TDD); the modal choice reuses the boolean `AnswerTriggerChoice` instead of a
new typed action; a dedicated forged *archived-state* refusal test is still missing
(the traversal itself is implemented); compatibility-only initialisation of the new
public state fields in `crates/ai/tests/common.rs` and `crates/core/tests/common/mod.rs`
(mechanical `match` arms, no policy/search/model change).

## Executed verification (native, this run)

- `cargo test -p aperture-core --test abilities` → **13 passed** at the repaired head
  (the same suite was 12/1 failed at `9c3a594`).
- Focused, independently re-run at `af0feea`:
  `command_zone_setup` 44, `abilities` 13, `rules_conformance` 5,
  `issue17_slice17` 6, `card_printings` 8, `catalog_pool` 5,
  `issue17_slice15_corpus` 2, `issue13_commander_combat` 43 → **126 focused tests, 0 failed**.
- `cargo fmt --all --check` exit 0; `cargo clippy --workspace --all-targets --locked -- -D warnings`
  exit 0; release CLI built.
- Independent replay of the recorded journey with the release CLI:
  `replay --file evidence/issue17/slice18/demo-replay.json` → 4 seats, **1961 actions,
  5028 events**, terminal `Win{P2}`, replayed digest `09e6bb25c505` matches the record.
  `sha256(decks/package-slice18.txt)` = `40028e6e00c2…4490ec` equals the journey's
  `snapshot_sha256`, so the four seats played the exact snapshot.
- Journey is an **evidence-only masked card-preferring demonstrator, not product AI or
  strength evidence**: 40-card package, 40 life, seed 1616, **Ayula injected as the
  commander for all four seats by the driver**. Ayula was cast 3 times, counters mode
  chosen 5 times, fight mode 4 times, 95 noncombat damage events.

## Bookkeeping reconciled (the reviewer's blocking finding)

The writer's handoff states **135 catalog / 123 supported / 336 missing** (union
459). My first independent parse produced 454 / 118 and I recorded the 5-name gap
as unexplained; the original-history reviewer then returned CHANGES_REQUESTED on
the stale records and supplied the resolution: the canonical union **includes the
five basic land types** (`Plains`, `Island`, `Swamp`, `Mountain`, `Forest`), and my
parse had filtered them out. 454 + 5 = 459, and 118 + 5 = 123 because all five
basics are supported. So the correct figures are **459 union / 123 supported / 336
missing**, per-list `55/0/51/48/30/52/52/53`, and my earlier 454/118 was an
audit artefact, not a second truth.

The same review found three stale statements in `docs/issue17-starter-corpus.md`
that this increment had left behind (`122`/`337`, "Ayula's printed counter/tribal
text is not implemented", and Ayula listed among unsupported commanders). I applied
the corrections as a documentation-only commit, distinguishing authored code from
record prose, and they are re-adjudicated below rather than assumed fixed.

## Original gate remains open

Eight-list gate unchanged: **1 of 8 desired starters certified**, seven planned.
Ayula's own list still has 30 unsupported names; equipment, soulbond, hideaway,
saddled mounts and Ring temptation in that list remain unimplemented. No arbitrary
Archidekt compatibility, no full Commander, no v1 claim. Desired 100-card snapshots
are unchanged; the journey package is explicitly **not** a 100-card certificate.
No merge, no master push, no issue closure, no certified-downstream unlock.

## Resume

Same card / PR46 / branch / exact saved histories:
- writer `evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl`
- reviewer `evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl`

Next: continue sourced missing cards for the Ayula list (Beorn the Fierce,
Druid's Familiar, Lightning Greaves, Stuffed Bear are the smallest remaining
semantics) and then genuine certification of one whole desired 100-card list. Do not
re-audit approved increments. Current-lane target only; preserve the other agent's
build-cache cleanup; no retired-worktree builds or global config edits.
