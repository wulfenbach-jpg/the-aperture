# Slice 20 — independent delta review

**APPROVED_INCREMENT** at `f4a02a2a05bce7ea6b22a66c60568256327f7ef5` (base `7dc933a`).

## Actual checks

- `cargo test -p aperture-core --test issue17_slice20 --test command_zone_setup --test abilities --test card_printings --test catalog_pool --test issue17_slice15_corpus --test issue13_commander_combat --test rules_conformance --locked`:
  slice20 **9/9**, command-zone **44/44**, abilities **13/13**, card printings **8/8**, catalog **5/5**, slice15 corpus **2/2**, commander combat **43/43**, rules conformance **5/5**.
- `replay --file evidence/issue17/slice20/demo-replay.json`: 4 seats, 2064 actions, terminal Win P3, 5281 events, digest `72cd69fb1143…` matches.
- `replay --file evidence/issue17/slice20/demo-save.json`: stored/recomputed digest matches and the save reproduces all 2064 actions.
- `sha256(decks/package-slice20.txt)` = `12e6d1e9…`, matching the journey `snapshot_sha256`.
- `cargo fmt --all -- --check` and `git diff --check` pass.

## Findings

No blocking behavior defect found. Vivien’s printed `{3}{G}` ability is a real hand→stack→battlefield test path. The top card remains anchored in the library under `Pending::TopCardChoice { player, card, can_take }`; eligible creature/planeswalker cards offer owner-only private take-or-bottom, ineligible cards offer mandatory private bottom, take reveals only the chosen card, and empty library neither draws nor loses. Wrong-seat/wrong-action and forged-take actions are refused with no mutation; mid-pending save/reload and two same-source activations both resolve exactly once. The validator repair correctly treats ability stack entries as source references and still requires exactly one actual spell container.

Nonblocking notes: the planeswalker eligibility branch is code-inspected only because no planeswalker card exists in the pool; the demonstrator injects Jasmine Boreal for all four seats and is evidence-only/40-card, not product AI or a 100-card certificate. Exactly one of eight original desired starters is certified.

Full #17 remains unfinished: **137 catalog / 459 union = 125 supported + 334 missing**, Ayula 28 missing. No merge, closure, or downstream unlock. Exact-head CI is still parent-owned; this report does not claim it green.
