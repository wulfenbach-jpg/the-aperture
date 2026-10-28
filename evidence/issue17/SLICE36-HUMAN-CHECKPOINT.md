# Slice 36 — review-297 evidence repair (same draft PR46, same branch)

Card / branch / PR are unchanged: kanban `t_67778feb`, branch `aperture/issue-17`,
draft PR46 (https://github.com/wulfenbach-jpg/the-aperture/pull/46). Approved
increments 1–35 were **not** re-audited; this slice answers review 297 only.

* **Code head of record:** `7bb8197c2b842ff7c2ba071b05bdb309988e735d`
  (driver + its artifacts + one player-facing description line + docs).
* **Evidence tip of record:** `be561114114010991b3371ebb4508370dc0d7825`
  (this checkpoint, both exact-head CI receipts and the PR-body update). The two
  are different commits on purpose: the code head is what the focused tests and
  the journey describe, the tip is what carries the record for them.
* Local = remote = PR `headRefOid` = `be56111`. PR is OPEN, **draft**, `BEHIND`
  master, no merge, master push, issue closure or downstream unlock.
* **Original issue-17 contract remains unfinished**: 1 of 8 desired 100-card
  lists certified, Ayula 15 names short, 321 of 459 union names unsupported.

## What review 297 required, and what was done

**Blocking (the only required change): the replacement effect had no committed
four-seat evidence.** Slice 35's heuristic journey drew Words of Wilding three
times and never cast it, so a *replacement* effect was demonstrated by unit
tests alone — which the supervisor's delivery bar excludes.

Added `evidence/issue17/slice35-journey-driver` (a separate non-workspace cargo
project, the same pattern as the slice-33 driver). It plays a **real four-seat /
40-life game** from the exact `decks/package-slice35.txt` snapshot, chooses only
from the masked `view_for` of the seat that has priority (no injection, no deck
substitution, no hidden-information access), casts Words of Wilding and
activates its `{1}` ability during its own upkeep so that the following draw
step consumes the armed replacement. Every assertion is made against the
engine's own events and state, not a test-only shortcut:

* no `Event::Draw` for the drawing seat, and no `Event::DrewFromEmpty`;
* that seat's library and hand sizes are **unchanged** across the action;
* exactly one `Event::PermanentEntered` for a `BearToken` under that seat's
  control, and the token really answers `Game::is_creature`,
  `Game::has_color(Green)` and 2/2;
* the armed count drops by exactly one and the seat is still live and not
  `drew_from_empty` (CR 614.1a/614.6, 704.5b).

It also exercises the **unspent** half of the printed text in the same game:
from turn 6 a seat may arm two replacements in one upkeep, so one is consumed by
the draw step and the survivor ends at that cleanup (CR 514.2). The driver
asserts the conservation law `armed = replaced + cleared + leftover`, which is
what the printed text says must happen and nothing else.

### Result (regenerated, byte-identical on re-run)

| quantity | value |
|---|---|
| game | 4 seats, 40 starting life, terminal `Win{P3}` |
| deck / seed | `decks/package-slice35.txt`, seed 3535 (exact snapshot) |
| steps / turns | 2048 / 41 |
| digest | `1c13d8fdcdce2e427588650b0a69e0c7365d382c432b43d2983b361d1a6099c0` |
| replay events | 5347, `replay_digest_matches: true` |
| casts | Words of Wilding 4, Grizzly Bears 6, Runeclaw Bear 1, Ursine Champion 3 |
| Words activations | 124 |
| **draws replaced** | **25** (by seat `[3, 3, 9, 10]` — every seat) |
| spare arms cleared at cleanup | 94 (by seat `[6, 18, 28, 42]`) |
| armed when the game ended | 5 |
| balance | **124 = 25 + 94 + 5** |
| Bear tokens from replacements | 25 |
| `DrewFromEmpty` events | 0 |

Artifacts committed in `evidence/issue17/slice35/`:
`driver-run.log` (the driver's own report),
`driver-journey.json`, `driver-replay.json`, `driver-save.json`
(regenerated twice with `sha256sum -c` clean: `driver-reproducibility.log`),
and `driver-cli-replay.log`, a **separate release-CLI replay** of the driver's
own replay file:

```
replay seed 3535 — 4 seat(s), 2048 action(s), recorded outcome Some(Win { winner: 3, ... })
  replayed digest 1c13d8fdcdce | matches recorded | events 5347
```

The heuristic selfplay journey (`demo-journey.json`, digest `59f8bf2017…`) is
left untouched and still reproduces; it remains the receipt for the *policy*
game, not for the replacement.

**Non-blocking items from review 297, all fixed in the same commit**

2. `Effect::ReplaceNextDrawWithBearToken`'s player-facing description now reads
   the printed subject: "the next time **you** would draw this turn, create a
   2/2 green Bear token instead" (`crates/core/src/card.rs:436`).
3. `docs/issue17-starter-corpus.md` no longer retro-edits slice 33's own-time
   count: the slice-33 section reads "Ayula had **16** missing names when this
   slice landed (each slice section states its own-time count)", while the
   slice-35 section keeps its own 15.
4. This record cites the docs tip `ec8dac2`'s own CI, captured live, not
   inherited: **10/10 completed SUCCESS, all keyed to `ec8dac2`**
   (`exact-head-ci-ec8dac2.json`), and the new head likewise:
   **10/10 completed SUCCESS, all keyed to `7bb8197`**
   (`exact-head-ci-7bb8197.json`, capture exit 0).
5. The commander-draw ordering limit is now a stated limit, not silence: the
   armed replacement is consumed before the CR 903.9b commander-draw
   suspension, so it pre-empts the reveal with no CR 616.1 choice surface; both
   are replacement effects on the same draw, the engine has no such choice
   surface, and nothing in the pool can put a commander into a library, so it is
   unreachable today and disclosed as a simplification.

Also fixed in passing, because it produced a false failure in this run's own
capture: `evidence/issue17/slice35/capture-ci.py` now resolves an abbreviated
sha to the full commit id before comparing `head_sha`, so a green head can no
longer be reported as `ALL_KEYED=False`. Every `gh` call keeps its hard
`timeout=120` (exit 124 on timeout).

## Gates (mine, in this checkout at `7bb8197`)

| gate | result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test -p aperture-core --test issue17_slice35 --locked` | 6 passed / 0 failed |
| `cargo test -p aperture-core --test card_printings --locked` | 13 passed / 0 failed |
| `cargo test --workspace --locked` | **120 suites / 853 passed / 0 failed**, exit 0 |
| corpus audit (`corpus-audit.py`) | 150 catalog / 459 union = **138 supported + 321 missing**, per list `[55, 0, 51, 48, 15, 52, 52, 53]`, Ayula **15**, **1 of 8 certified** |
| exact-head CI | **10/10 completed SUCCESS, all keyed to `7bb8197`** |

Logs: `evidence/issue17/slice35/gates-repair.log`,
`workspace-tests-repair.log`, `corpus-audit.log`,
`exact-head-ci-7bb8197.json`, `exact-head-ci-ec8dac2.json`.

## What this slice does *not* do

* It does not certify a second starter list, and it does not move Ayula's 15.
* It does not fix the two open non-blocking notes from review 290 that were
  already disclosed: the printed-7b-before-7a ordering inversion (unreachable
  today, no card prints 7b after animating) and the `card_printings`
  "colour identity" wording.
* It does not merge, push master, close issue 17, or unlock
  `t_26a7a4ff` / `t_3c2fe51a`.
* No import/start UI, AI-search or model behaviour changed. The driver is
  evidence-only, is not part of the workspace, and is not product AI.

## Next

Same card, same PR46, same histories: continue the supervisor's Ayula-to-zero
direction from the sourced-name list (15 remaining) toward a whole certified
100-card list, checkpointing after every card.
