# Slice 50 — Witness predicate settled from pinned rules; CR 509.1g combat defect repaired

**ORIGINAL #17 REMAINS UNFINISHED: 1 of 8 lists certified, catalog 161, union 459 = 148 supported + 311 missing, Ayula 5 missing. This slice adds no card and no certificate.**

## 1. The disclosed Witness conformance gap is closed against the pinned rules

Fetched the pinned official artifact to a live check: `https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt`,
**977752 bytes, SHA-256 `8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`** —
identical to `docs/rules-reference.md` §1 (`evidence/issue17/slice50/rules-source.json`).
Re-fetched `who/111` and its rulings (`witness-source.json`: bytes 5723, printed text verbatim,
printing `da05e23b…`, oracle `d273033b…`, no card-specific ruling; `witness-rulings.json` is empty).

* **CR 506.5** distinguishes *"attacks alone"* (only creature **declared** an attacker) from
  *"is attacking alone"* (attacking with no other attackers), and the blocking pair likewise.
* Witness prints **"was attacking or blocking alone"** — a past-state predicate, not "attacked alone".
* Therefore the pre-death **live** combat membership is the predicate; a companion leaving in an
  **earlier** event can make a later death alone, while a **simultaneous** batch must share one
  pre-departure snapshot (CR 704.3). The earlier declaration-only requirement was wrong and is
  corrected, not defended.

Executed pre-repair RED (`slice50/red-witness.log`): the two corrected sequential regressions both
compiled and **failed 6 passed / 2 failed, `Graveyard != Exile`, exit 101**. After the repair the
same file is **14 passed / 0 failed** (`slice50/unpaired-controls.log`, `slice50/expanded-green.log`).
New controls beyond the correction: a Cub-put attacker is attacking but never declared, the same
blocker bounced and legally **Flash-recast with the same ObjectId** no longer blocks, a real absent-
field (legacy-shaped) middle-of-combat state loads and still triggers, and 8 malformed states are
refused by validation **and** actual `SessionSave::load`.

## 2. A second, independently reproduced product defect: CR 509.1g

While probing the blocking predicate, a real **Unsummon** then **Lightning Strike** sequence showed
the blocker stopped counting as blocking once its attacker left. CR 509.1g says it *"remains a
blocking creature until it's removed from combat or the combat phase ends"*, and removing the pair
is not removal of the creature. That test compiled and **failed before the fix**
(`slice50/red-orphan-blocker.log`, `Graveyard != Exile`, exit 101).

Repair: omitted-when-empty `Turn::unpaired_blockers` holds only surviving live blockers whose
attacker left. It joins paired blockers in the snapshot, is dropped on that creature's own departure
or regeneration, retained when a noncreature attacker is pruned, cleared at end of combat/turn start,
and validated against step, defender, canonicity, creature-ness and non-overlap. The failing test
now passes with an **actual saved-state load** continuing identically, plus pair/unpaired counts and
the same-ObjectId recast control.

Files: `crates/core/src/{engine.rs,state.rs}`; `crates/core/tests/issue17_slice46.rs`
(**8 → 14 `#[test]`, delta +6** counted from the diff); `crates/core/tests/common/mod.rs`
(initializer); docs + README current-state prose; `crates/ai/src/search.rs` (**one public field copy
and its stale comment only**) and `crates/ai/tests/common.rs` (initializer) — a narrow shared-state
exception disclosed for review, not a policy, budgeting, sampling or hidden-information change.

## 3. Executed evidence (all exit 0)

`slice50/gates.log` records every command's own exit code:
`fmt --check`; focused **12 suites / 138 passed / 0 failed** (`issue17_slice46` 14, slice48 6,
slice49 8, slice43 4, combat 8, combat_teaching 2, commander_combat 43, m1_multiplayer 28,
ordering_persistence 2, replay_digest 6, starter_corpus 8, and `aperture-ai search` 9 — the search
suite is listed separately in the log and **not** counted in the 138); corpus audit **AUDIT OK**
(161 / 459 / 148 / 311, per-list `[55,0,51,48,5,52,52,53]`, 1 certified); **workspace Clippy
`-D warnings`**; locked release CLI build; `git diff --check`.

Journeys, both re-executed from the committed drivers:

* **Witness package unchanged and byte-identical** — `Win{P1}`, 3073 steps, 54 turns, 8098 events,
  digest `da4eacd5…b995bde`, 11 Witness casts, 38 solo attacks, **104 Clue tokens**, **14 exiles**;
  separate release-CLI replay matches (`slice50/witness-journey.log`). This recording did not
  exercise the corrected paths, so the fix changed no recorded behaviour here — say that plainly
  rather than claiming the journey proves the fix.
* **Cub package unchanged** — `Win{P2}`, 3463 steps, 95 turns, 8658 events, digest
  `acf204aa…747ac9`, replay matches.

## 4. Persistence position (recorded, not papered over)

`SAVE_VERSION` stays **10**, `REPLAY_VERSION` stays **6**. The witness predicate now needs no
declaration, so an older mid-combat save no longer under-reports an alone death; the absent-field
state-continuation regression proves that half by execution, and **no claim is made** that every
older artifact verifies its stored digest (an older recording still re-executes to a different
digest because new declaration/`unpaired_blockers` fields are written). A save made **after** losing
all blocking pairs has already discarded that membership and cannot be reconstructed; stated in
README and docs.

## 5. What is deliberately not done

No card was enabled, Ruxa's three-clause card is **not** implemented, and the exact 100-card
four-seat certification has **not** run — Ayula still has five missing names. Next work is written
in `slice50/NEXT-CARDS.md` (Ruxa first, with the no-abilities predicate and a typed unblocked-damage
choice as the real blockers). This run co-authored the increment and is **not** its independent
reviewer.

Children `t_26a7a4ff` (scheduled evaluation dependency wait) and `t_3c2fe51a` (unassigned soak
backlog) inspected and left gated. No merge, master push, issue closure, downstream unlock, new team
or provider/model-config change.
