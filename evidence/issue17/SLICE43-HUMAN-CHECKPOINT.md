# Slice 43 human checkpoint — review-314 corrections, the CR 702.6b regressions, the effect-ordering sweep, and Lumra

**Task:** t_67778feb — #17 Certify the v1 Commander starter corpus end to end
**PR:** https://github.com/wulfenbach-jpg/the-aperture/pull/46 (OPEN, draft, `aperture/issue-17`)
**Base:** `cddb5f5` (review 314: APPROVED_INCREMENT for slice 42, three record corrections required)
**Original #17 status after this slice: UNFINISHED.** 1 of 8 lists certified; Ayula has **9** names missing
(was 10); union 459 = **144 supported + 315 missing**. Nothing here is a certificate, and this slice
does not claim whole-issue acceptance.

This slice does three things the supervisor's run-315 handoff asked for, and one card of real
implementation:

1. the three record corrections review 314 required (R314-1, R314-2, R314-3);
2. the CR 702.6b equip-targeting regressions that approval left unproven, in both directions;
3. the continuous-effect ordering sweep review 314's supervisor note demanded; and
4. **Lumra, Bellow of the Woods** — a printed `*`/`*` body, reach, vigilance and a mill-then-return-lands
   enters trigger — the tenth Ayula name closed.

## 1. Record corrections (review 314 §5)

- **R314-1 — the live manifest field now matches the engine.** `decks/starter-corpus/manifest.json`
  `starters[ayula-bear-counters].missing_interactions` is rewritten as a whole: the equipment entry is
  gone (slice 42 implemented it), the "library search … (The Earth King …)" entry is gone (slice 41
  implemented it), and one entry remains per still-missing interaction, each naming the card that
  prints it. `missing_cards` drops Lumra (this slice) and now holds 9 names.
- **R314-2 — the published workspace count is the command's own number.** `SLICE42-HUMAN-CHECKPOINT.md`
  and `slice42/RESULT.json` now publish **892 passed / 0 failed / 1 ignored over 125 binaries** for
  `cargo test --workspace --locked`, name that command explicitly, and record that the previously
  published 995 was the aggregate sum across the whole `native-verify.log` receipt (which counts the
  focused suites twice).
- **R314-3 — the live capability sentences are true again.** `README.md` no longer says layer 6 grants
  stop at trample/haste (the two Equipment grant shroud and hexproof), and
  `docs/issue17-starter-corpus.md` no longer says nothing in the pool grants shroud/hexproof; both name
  the slice-42 source and keep "protection is unsupported" — which is still true.

## 2. Equip is a targeted ability (CR 702.6b) — now proven, not assumed

Review 314 approved slice 42 while listing this rule as unproven. It **passes**, and the proof is a new
committed regression in `crates/core/tests/issue17_slice42_equip_controls.rs`:

`equip_cannot_target_a_protected_creature_but_keeps_existing_attachments`
- Greaves {0} onto creature A → A has shroud; the Boots equip is **not offered** for A and a direct
  submit is refused with the state untouched (`attached_to(A) == 1`, nothing moves to B);
- Boots onto B (hexproof only) **succeeds** — hexproof is opponents-only, so the controller's own
  targeted ability is legal;
- Greaves onto B **succeeds while B has hexproof**, and B keeps **both** attachments after gaining
  shroud: a protection keyword gained after attachment never detaches anything retroactively;
- moving Greaves back to A leaves exactly one attachment per Equipment — no stale relation survives.

So the engine's answer to the supervisor's question is: shroud blocks the controller's own equip,
hexproof does not, and there is no retroactive detach. The test fails if any of those three change.

## 3. Continuous-effect ordering sweep

`evidence/issue17/slice43/EFFECT-ORDERING-SWEEP.md` enumerates every site that clears or rebuilds
`state.effects` and states, per site, whether the slice-42 defect class is reachable:

| site | mutates effects | derived query after the mutation? | reachable? |
|---|---|---|---|
| `recompute_layers` (~493) | `mem::take` + restore | yes — attachment validity, **after** restore | no (this was the defect; fixed at `20484f8`) |
| `detach_object` (1607) | `retain(target != id)` | no | no |
| `finish_cleanup` (~5715) | `retain(duration != UntilEndOfTurn)` | no (recompute follows) | no |
| `AttachSourceToTarget` (~7733) | `retain(source != item.object)` | no — target validity computed **before** | no |
| query functions | none (`&self`) | n/a | n/a |

There is exactly one `mem::take(&mut self.state.effects)` in the crate and no `state.effects.clear()`.
The sweep also discloses a new, narrow limitation it found: `Keyword::Vigilance` is read from printed
keywords, not from the live granular-effect path `has_haste` uses. Nothing in the pool grants or removes
vigilance, so no reachable state differs today; a future grant must be routed through that path first.

## 4. Lumra, Bellow of the Woods (BLB 183) — the tenth Ayula name

Source: `fixtures/starter-corpus/slice43-sourced.json`, raw capture in
`evidence/issue17/slice43/sources/lumra-blb-183.json`, printing `ae4f3aaf-3960-48cd-b34b-32e4ae5ae088`,
oracle `97a84e9d-bfc4-4ca2-b1e8-908dba56ccdb`, Scryfall, retrieved **2026-10-28T10:55:27Z**,
commander-legal. The catalog string equals the printing's `oracle_text` verbatim, and
`card_printings::slice43_lumra_matches_its_recorded_printing` asserts that line by line, including that
a `*`/`*` card carries **no invented numbers**.

What is new in the engine:
- **printed `*`/`*`** — represented as no printed power/toughness plus a characteristic-defining
  static (`PowerToughnessPerCount { count: LandsYouControl }`); `catalog_pool` now requires a CDA
  static for any creature without printed numbers instead of allowing a hand-written constant;
- **reach** as a blocking permission (already-executed keyword table) and **vigilance** (CR 702.20a)
  read at the single site that taps an attacker (CR 508.1f);
- **mass graveyard→battlefield return** (`Effect::ReturnLandCardsFromGraveyardTapped`): the set is
  taken at resolution (CR 608.2c) and every land enters through the canonical entry path, so each
  returning land fires existing landfall watchers once, and all of them enter tapped.

## 5. Verification actually run

Focused (`issue17_slice43` 4/4):
- P/T equals the number of **its controller's** lands, and an opponent's lands do not count;
- the enters trigger mills exactly four and returns **every** land card from the graveyard tapped: two
  seeded lands return, no land card remains in the graveyard, a Jaddi Offshoot's landfall fires twice
  (life 20 → 22), and Lumra becomes 5/5, `validate()` Ok;
- vigilance: attacking leaves Lumra untapped while an ordinary attacker still taps, and Lumra is
  recorded as attacking;
- reach: Lumra blocks a Spirit token that a Grizzly Bears cannot block ("flying or reach").

Suites re-run at this head: `card_printings` **16/16** (new printing test), `catalog_pool` 5/5,
`starter_corpus` 8/8, `issue17_slice42` 9/9, `issue17_slice42_equip_controls` **4/4**,
`issue17_slice41` 11/11, `issue17_slice12` 11/11, `issue17_slice15_corpus` 2/2,
`issue13_commander_combat` 43/43 (pool-size assertion moved 155 → 156).

Four-seat journey (`decks/package-slice43.txt`, snapshot sha256 `32378ca0…3099`, 40 cards, 4 seats,
40 life, seed 2324, evidence driver `evidence/issue17/slice43-journey-driver`): terminal
`Win{P2}`, **1433 steps, 34 turns, 3604 replay events, digest `689ed550…677f`**, 4 Lumra casts,
**9 declared Lumra attacks each verified still untapped**, 4 graveyard-land-return triggers,
**12 returned land entries**, 29 combat-damage events; the replay re-runs to the same digest and a
fresh `SessionSave` JSON reloads to an equal state. Receipt: `slice43/journey-driver.log`,
artifacts `slice43/demo-{journey,replay,save}.json`.

Gates: `cargo fmt --all --check` 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` 0;
`cargo test --workspace --locked` in `slice43/native-verify.log`.

## 6. What this slice does not do

- **No certification.** Ayula still has 9 unsourced-in-engine names, so it stays `planned`; the
  eight-list gate is still 1 of 8 and no starter status changed.
- No 100-card four-seat game: the journey is a 40-card evidence package and masked demonstrator, not a
  certificate and not product AI strength.
- The legend rule (CR 704.5j) and the Ring/hideaway/modal/snow/top-of-library interactions that the
  remaining 9 cards need are **not** implemented; they are now listed per card in the manifest.
- Lumra's vigilance is printed-only (see §3), and its return effect returns lands, not arbitrary
  permanents — a future card that returns nonlands needs its own effect.
- No merge, master push, issue closure, downstream unlock, new team, or global config change.
