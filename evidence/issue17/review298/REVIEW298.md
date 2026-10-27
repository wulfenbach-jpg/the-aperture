# REVIEW 298 — slice 36 (review-297 evidence repair) on PR46

* Reviewer: independent review of the review lane, task `t_67778feb`, tenant `the-aperture`.
* Delta reviewed: `ec8dac2..be561114114010991b3371ebb4508370dc0d7825` (code head `7bb8197c2b842ff7c2ba071b05bdb309988e735d`).
* Lens: **contract** (round ≥ 3 — re-read the ORIGINAL #17 body and audit the deliverable against it, plus verify every item of the prior `changes_requested` round landed).
* Verdict: **APPROVED_INCREMENT** at exact `be56111`. No functional defect. The ORIGINAL issue-17 contract is still unfinished, so the card returns to implementation.
* Reviewer made **no implementation edits**. Everything below was produced in an out-of-tree copy (`C:/AI/r298c`, `git archive be56111 | tar -x`) or read-only in the worktree, except the untracked report in this directory.

## 1. The delta, cold

`git diff --stat ec8dac2..be56111` = 21 files: one product line
(`crates/core/src/card.rs`, the player-facing wording of
`Effect::ReplaceNextDrawWithBearToken`, "that player" → printed "you"), 30 lines of
`docs/issue17-starter-corpus.md`, the new non-workspace driver
`evidence/issue17/slice35-journey-driver/` (+ `Cargo.lock`), its committed artifacts, two
CI receipts, the checkpoint, and the PR-body script/read-back. `git diff --stat -- crates/`
is that single string. No engine, catalog, ability, AI, UI or import behaviour changed.

## 2. What I ran, out of tree, at `be56111`

`git archive be56111 | tar -x -C C:/AI/r298c`, then in
`evidence/issue17/slice35-journey-driver`: `cargo run --release --locked` → **exit 0**,
terminal `Win{P3}` (opponents' life ≤ 0, CR 704.5a), seed 3535, **2048 steps / 41 turns /
5347 replay events**, digest `1c13d8fdcdce2e427588650b0a69e0c7365d382c432b43d2983b361d1a6099c0`,
snapshot `0726fadc6ca9503fe405f3b944d15f5421903d6a0d0082ef999b6d3daf7fee7e`,
`replacements_armed 124`, `draws_replaced 25` by seat `[3,3,9,10]`,
`unspent_replacements_cleared_at_cleanup 94` by seat `[6,18,28,42]`,
`armed_remaining_at_game_end 5`, `bear_tokens_from_replacements 25`,
`drew_from_empty_events 0`, casts `Words of Wilding 4 / Grizzly Bears 6 / Runeclaw Bear 1 /
Ursine Champion 3` — i.e. the printed conservation law **124 = 25 + 94 + 5** holds.

Reproducibility, checked against the **git blobs** rather than a checkout (see finding N-3):

| artifact | my fresh run / git blob at `be56111` |
|---|---|
| `slice35/driver-journey.json` | `75b0f497…a90aa2` — identical |
| `slice35/driver-replay.json` | `81d05546…a8db21` — identical |
| `slice35/driver-save.json` | `35f7ec9e…fb5123` — identical |
| `slice35/driver-run.log` | `a9a60fb0…4548a9` — my stdout is identical line-for-line |

Separate replay, built by me in the same tree:
`cargo run --release --locked -p aperture-cli -- replay --file …/driver-replay.json` → exit 0,
`replayed digest 1c13d8fdcdce | matches recorded | events 5347`, 2048 actions — the same
values `driver-cli-replay.log` records.

### The driver's assertions are load-bearing (my mutation, out of tree)

I replaced the body of `Game::take_draw_replacement` with `false` (the replacement is never
consumed) in my copy only, rebuilt, re-ran the driver: **panic at `src/main.rs:207`
"CR 614.1a/614.6: a replaced draw is never performed", exit 101**. The evidence instrument
therefore fails when the replacement stops happening; it is not a decorative trace.
`engine.rs` restored (`sha256sum -c` OK, `8750301f…326e68`) and byte-identical to the
committed blob modulo the archive export's line endings.

The driver also keeps the delivery bar honest by construction: it chooses only from
`game.view_for(player)` legal actions (land → Words of Wilding → creature, arm in its own
upkeep, always attack), submits everything through `submit_action(...).expect(...)` so a
rejected action aborts instead of being swallowed, and asserts `replay.seats == seats`,
`replayed.digest() == game.digest()`, equal life totals, an exact `SessionSave` round-trip and
absence of `Event::Truncated`.

## 3. Review-297's five items, one by one

| # | Required | Status and evidence |
|---|---|---|
| 1 | BLOCKING: a committed slice-33-style four-seat/40-life driver that casts Words of Wilding, activates it, and asserts the replaced draw from engine events, with log and regenerated journey/replay receipts | **Closed.** Committed driver + 4 artifacts; reproduced above by execution; 25 verified replaced draws across all four seats; byte-identical on re-run; separate CLI replay matches |
| 2 | `card.rs:437` wording ("that player" vs printed "you") | **Closed.** `crates/core/src/card.rs:436` now reads "the next time **you** would draw this turn…" — the only product-code line in the delta |
| 3 | Slice 33's retro-edited Known-limits count | **Closed.** The slice-33 section now reads "Ayula had **16** missing names when this slice landed (each slice section states its own-time count)"; the slice-35 section keeps its own 15. 16 is the correct slice-33 value (attempt-44 audit `[55,0,51,48,16,52,52,53]`) |
| 4 | Cite the docs tip `ec8dac2`'s own CI, not an inherited green | **Closed.** `slice35/exact-head-ci-ec8dac2.json` and `exact-head-ci-7bb8197.json` committed; I re-read both heads live: 10/10 completed SUCCESS, every `head_sha` equal to the head, zero other head shas |
| 5 | One-line limits note: the replacement is consumed before the CR 903.9b commander-draw suspension, no CR 616.1 choice surface | **Closed.** Stated in `docs/issue17-starter-corpus.md` Known limits and in the checkpoint (both effects replace the same draw, the engine has no choice surface, unreachable with this pool) |

Also in the delta and checked: `slice35/capture-ci.py` now resolves an abbreviated sha through
the API before comparing `head_sha`, so a green head cannot be reported as `ALL_KEYED=False`;
every `gh` call keeps `timeout=120` (the supervisor's slice-30 requirement). That is a
strictly stronger gate, not a weakened one.

## 4. Gates I ran (my copy at `be56111`)

| gate | result |
|---|---|
| `cargo test -p aperture-core --test issue17_slice35 --test card_printings --locked` | 6 passed / 13 passed, 0 failed, exit 0 |
| `cargo test --workspace --locked` | **120 suites / 853 passed / 0 failed**, exit 0 |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| driver (`--release --locked`, out of tree) | exit 0, reproduces the record |
| separate release-CLI replay | digest matches, 5347 events |

CI, read live from the API this run (not from the receipts):
`7bb8197` 10/10 completed/SUCCESS all keyed to itself; `ec8dac2` 10/10 keyed to itself;
tip `be56111` 10/10 keyed to itself; `gh pr checks 46 --required` → all 8 required contexts
pass; PR **OPEN / draft / MERGEABLE / BEHIND** master at `be561114114010991b3371ebb4508370dc0d7825`.
Local == remote == PR `headRefOid`.

## 5. Corpus audit — recomputed by me, not read from the lane

My own script (`audit298.py`, generic `card*!` macro parse of `crates/core/src/catalog.rs`
plus the eight manifest deck files, asserting 99 + commander each and set-equality with every
`missing_cards`): **150 catalog / 459 union = 138 supported + 321 missing**, per list
`[55, 0, 51, 48, 15, 52, 52, 53]`, **Ayula 15**, **1 of 8 certified**. Every manifest entry
agrees; the lane's `slice35/corpus-audit.log` says the same numbers.

Ayula's remaining 15 names (the release-gate tail):
Druid's Familiar, Dúnedain Rangers, Evercoat Ursine, Lightning Greaves, Lumra Bellow of the
Woods, Owlbear Cub, Rampaging Yao Guai, Ruxa Patient Professor, Spider-Ham Peter Porker,
Spirit of the Aldergard, Swiftfoot Boots, The Earth King, Thijarian Witness, Titania's Command,
Ursine Monstrosity.

## 6. The ORIGINAL #17 contract (contract lens)

| acceptance criterion | state at `be56111` |
|---|---|
| Publish at least eight explicitly supported 100-card lists across play styles | **Unmet**: 1 of 8 certified (`jasmine-vanilla-stompy`) |
| Every included card and required interaction has an implementation and conformance coverage | **Unmet**: 321 of 459 union names unimplemented; Ayula 15 short |
| Play four-seat games using exact deck snapshots | Met only for 40-card evidence packages (this driver included); no 100-card starter snapshot has been played |
| Document per-card/interaction exclusions | Maintained and extended by this delta |

So this increment is evidence repair plus one wording fix. It closes a review gate; it does
not move the release gate. Slice 35's card was itself an Ayula name, so the count is unchanged
from slice 35.

## 7. Findings

**No blocking defect in this delta.**

Non-blocking, all cheap and foldable into the next evidence commit:

* **N-1 — `SLICE36-HUMAN-CHECKPOINT.md:9` is stale by its own commit.** It states
  "Local = remote = PR `headRefOid` = `7bb8197`", but the checkpoint is committed at
  `be56111`, which is the PR head. State the two heads explicitly (code head `7bb8197`,
  evidence tip `be56111`, as the handoff comment and PR body already do). The gate table
  itself is accurate and is *not* an overstated-green claim: it cites only `7bb8197` and
  `ec8dac2`, and the tip's own 10/10 exists live but is not claimed in the record.
* **N-2 — the tip's CI receipt is untracked.** `slice35/exact-head-ci-be56111.json` exists in
  the worktree but is not committed, and no document cites it. Either commit it with the next
  evidence commit or leave the record as-is, but do not cite a tip green without the artifact
  (the review-288 requirement).
* **N-3 — artifact receipts should say what they hash.** `driver-reproducibility.log` hashes
  are the **git blob** hashes (LF), while a plain checkout on this host and a `git archive`
  export produce CRLF, so a reader who runs `sha256sum -c` on a fresh export sees three
  FAILED lines. Is this a false record? No: my fresh run reproduces the blobs exactly. But
  label the receipt "git blob hashes (`git cat-file`)" to avoid a false alarm — this is the
  same class as review 296's "bind the checked-out file" note.
* **Carried forward, still open and disclosed (not required here):** review 290's
  printed-7b-before-7a ordering inversion (unreachable) and the `card_printings`
  "colour identity" wording; review 296's missing committed runner for the slice-34
  mutations and the `f3e628d` cancelled `windows package` note; the three tracked slice-23
  evidence files that carry pre-existing uncommitted edits (unchanged by this delta and by me).

## 8. Verdict

**APPROVED_INCREMENT at `be561114114010991b3371ebb4508370dc0d7825`** (code head
`7bb8197c2b842ff7c2ba071b05bdb309988e735d`). Review 297's only blocking item is closed with a
genuine four-seat game whose assertions I broke on purpose and watched fail. The ORIGINAL #17
is unfinished (1 of 8 certified, 321 of 459 union names missing), so the card returns to the
same implementation lane — same card, PR46, branch `aperture/issue-17`, same histories — to
continue the supervisor's Ayula-to-zero package.

No merge, no master push, no issue closure, no downstream unlock (`t_26a7a4ff` /
`t_3c2fe51a` stay gated), no duplicate task/team/PR, no global config change.
