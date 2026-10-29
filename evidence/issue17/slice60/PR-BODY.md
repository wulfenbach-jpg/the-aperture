## Slice 60 — Evercoat Ursine implemented and the exact Ayula 100-card list certified (**2 of 8**)

Same draft PR, branch `aperture/issue-17`. **This is a candidate for independent
increment review, not a self-approval.** Original #17 remains **unfinished**: six
lists are still planned with real card gaps; no merge, master push, issue
closure or dependent unlock.

### Record fix (opening commit `99af329`)
R359-1: the slice-59 checkpoint had the CI run ids swapped. Push is
`40825526651`; the `pull_request` merge-ref run is `40825533912`.

### Card — Evercoat Ursine (BLC 30, `{4}{G}`, 6/5 Elemental Bear, Trample)
Oracle text and the six official rulings fetched live (2026-10-29T06:58:12Z) and
recorded in `fixtures/starter-corpus/slice60-sourced.json` (2 printings, one
distinct rules text).

* **Trample** is a real keyword flag.
* **Hideaway 3, hideaway 3** is two separate triggered abilities: entering
  resolves two independent private looks (CR 701.20a), each exiling one card
  **face down linked to that permanent** (CR 702.75a) and bottoming the other
  looked-at cards in a random order (CR 701.20e).
* **"…you may play one of them without paying its mana cost"** is a real combat-
  damage trigger with the printed intervening-if (CR 603.4) and a resumable offer
  with **one action per playable linked card** plus an explicit decline. The
  printed ruling is enforced: play while resolving, a land needs the turn's land
  play (CR 305.2), a printed `{X}` is 0, no cost is paid, and the card stops
  being exiled with anything when it changes zone (CR 400.7).
* A face-down exiled card is identified only to the player who controlled the
  exiling permanent (CR 406.3); every other seat's projection shows "a face-down
  exiled card" and the public log never names it.
* Both new pending kinds are validated by the same structural validator a save
  load uses; a real `SessionSave::from_json` refuses an injected option.

### Defect fixed in an accepted increment
**Ursine Monstrosity printed "Trample" but was never given the keyword** — it has
been a 3/3 without trample since slice 54. It is now a `card_trample!` row with a
named regression. Stated here, not folded silently into the new card.

### Certification — 2 of 8
`aperture-cli certify --starter ayula-bear-counters --seats 4 --seeds 1,2,3`
hashes the exact snapshot
(`decks/starter-corpus/ayula-bear-counters.txt`, 99 cards,
sha256 `7a1b3f9c…19c4`), puts it in every seat with Ayula as commander, and plays
three four-seat 40-life games: all **terminal** (winners P1/P3/P2, 1499/1373/1233
actions, digests `78b301cbef68…`, `8ccf4bc4a753…`, `0adc5f3097ea…`) with every
replay reproducing its digest. The same games were replayed **separately** via
`selfplay --replay-out` and `replay --file` (`matches recorded`, exit 0).
Certificate: `evidence/issue17/slice60/ayula-certificate.json`.

The certificate generator's exclusion text used to claim the list contains no
equipment/enchantments — true for Jasmine, false for Ayula. Exclusions are now
computed from the list being hashed; the Ayula certificate names the artifacts,
Equipment and enchantments it really holds and says only the cast ones are proved
by the trace.

### Corpus
Fresh `AUDIT OK`: **166 catalog / 459 union = 153 supported + 306 missing**,
per-list `[55, 0, 51, 48, 0, 52, 52, 53]`, **2 of 8 certified**; Ayula's
`missing_cards` and `missing_interactions` are both empty and its `deck_path`
now points at the supported exact snapshot.

### Tests
`crates/core/tests/issue17_slice60.rs` (9 regressions: two hideaways, face-down
privacy, free cast, land-play rule, decline, no-trigger-without-link, forged
offer refused by validate *and* a real save load, dead link, Ursine Monstrosity
trample) plus the printing chain in `card_printings.rs`.

### Not claimed
Full Commander format, arbitrary Archidekt compatibility, playing strength, or
conformance of cards not exercised by these three games (28 of 59 names are
covered by suites, not by the trace).
