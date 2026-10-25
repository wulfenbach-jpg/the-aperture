# Issue 23 evidence — live-session persistence, replay branches, postgame review

All commands were run in `C:/AI/aperture-worktrees/issue-23` (branch
`aperture/issue-23`) or in the packaged folder
`C:/AI/mtg-slice/dist/TheAperture-issue23-study-b9af16a`. The packaged folder is
a copy of the tracked worktree's release build (no source edits were made in it).

## Focused automated controls

```
cargo test --locked -p aperture-core --test issue23_branch
  4 passed — prefix cut/replay determinism, out-of-range refusal, public-only
  comparison (no card names), deterministic forward play + JSON round-trip

cargo test --locked -p aperture-ui --test issue23_study
  4 passed — four-seat APNAP pending save/load parity (same pending, chooser,
  decision serial, stack length, digest), stale-branch generation discard,
  branch-at-chosen-action via the worker + public comparison, postgame review
  refused while live / omniscient / read-only once over

cargo test --locked -p aperture-ui --lib
  12 passed — includes the_postgame_review_replaces_the_action_strip_at_1280x720
  (review heading + body visible, "YOUR ACTIONS" absent, restored on leave)
```

Full workspace gates at the delivered head:

```
cargo test --locked --workspace     68 test binaries, 420 tests passed, 0 failed
cargo clippy --workspace --all-targets --locked    0 warnings, 0 errors
cargo fmt --all --check                            clean
cargo build --release --locked                     clean
```

No existing save/replay digest, legality, privacy, pending-choice or coach test
was weakened; the whole pre-existing suite still passes.

## Packaged CLI on the real engine

```
$ aperture-cli selfplay --seats 4 --seed 41 --replay-out finished.replay.json --max-steps 200000
selfplay seed=41 seats=4 heuristic vs heuristic vs heuristic vs heuristic -> status terminal winner P0 in 1104 steps (26 turns)
  digest c76c56bd72fd | replay verified | casts 24 attacks 9 blocks 1 counters 2

$ aperture-cli replay --file finished.replay.json --save-out finished-save.json
replay seed 41 — 4 seat(s), 1104 action(s), recorded outcome Some(Win { winner: 0, reason: "opponents' life totals reached 0 or less (CR 704.5a)" })
  certified session save of the finished game written to finished-save.json (digest c76c56bd72fd)

$ aperture-cli replay --file finished.replay.json --branch-at 400 --branch-out sample-branch-at-400.replay.json
branch from finished.replay.json at action #400: 4 seat(s), 400 recorded action(s) kept, replays to digest 07017824c34f under the same seed

$ aperture-cli replay --file finished.replay.json --compare-with sample-branch-at-400.replay.json
public accepted-action comparison — action kinds only, card names/targets/zone contents are never shown:
branches agree so far: every accepted action on the branch is still on the base line
```

## Real packaged-app journeys

Two windows are summarised. Window A ran the package built from `0974abc`
(everything below except the review-refusal message line). Window B ran the
final package built from the delivered head; it re-drove the branch and
postgame-review steps on that binary. Everything quoted is text read from the
live window's accessibility tree by the desktop driver; no offline helper
produced any of these strings.

### Window A (commit 0974abc) — four-seat table, save/new/load/branch/review

Window: `aperture.exe --seats 4 --profile ordinary --size 1280x720 --seed 41
--study-file the-aperture-save.json --branch-at 4`.

1. Four-seat table, opening choice pending:
   `WAITING: P0: keep or mulligan the opening hand (CR 103.5)` ·
   `4 seats · profile practice-20 (20 life) · turn 1 · untap step · active P0 · digest 5d0ca4568932`
2. A click on `Keep this hand` answered P0; subsequent AI-step clicks/deliveries
   advanced P1–P3 and the step, recording a real line:
   `· 4 seats · profile practice-20 (20 life) · turn 1 · upkeep step · active P0 · stack 0 · digest 2f96b415bb55`
3. `Save (S)` produced, in the window:
   `saved to the-aperture-save.json (digest 3afe44df885d)` — the file is on disk
   (version 10) and verifies standalone:
   `save the-aperture-save.json — 4 seat(s) … stored digest 3afe44df885d | recomputed 3afe44df885d | matches`
   `replay of 8 action(s) -> digest 3afe44df885d | events 44 | reproduces the save`
4. `New game` moved the table to a different game (turn 1, active P1, digest
   e0cf61c39b40), then `Load (L)` restored the saved boundary:
   `loaded the-aperture-save.json — digest 3afe44df885d (stored 3afe44df885d), 8 action(s) in the replay`
   with `upkeep step · active P0 · digest 2f96b415bb55` back on the table.
5. Study branch (the same save, action #4):
   `branched from the-aperture-save.json at recorded action #4 — the recorded line is replayed to digest 792bacc22341, new actions from here are branch history`
   and the strip then offers `Pass priority   [CR 117.3d, 117.4]` at that exact
   boundary.
6. Postgame review: the finished 4-seat save (1104 recorded actions) was loaded:
   `loaded the-aperture-save.json — digest c76c56bd72fd (stored c76c56bd72fd), 1104 action(s) in the replay`
   `P0 wins — opponents' life totals reached 0 or less (CR 704.5a)`
   `P0 20 / P1 0 (out) / P2 -5 (out) / P3 -4 (out)`
   `R` opened: `REVIEW — omniscient postgame view · game over · acting is disabled`
   with the body `POSTGAME OMNISCIENT REVIEW — shown because the game is over; this is not a policy view / rules revision 2026-09-25 / --- P0 …`
   while the opponent seat panels still read `public zones; hand/library contents hidden`.
   `R` again restored `YOUR ACTIONS — only engine-offered decisions are accepted`
   with `game over — P0 wins … Postgame review (R) is enabled.`

### Window B (final head) — same branch + review on the delivered binary

1. Start: `WAITING: P0: keep or mulligan the opening hand (CR 103.5)` ·
   `turn 1 · untap step · digest 5d0ca4568932` (four seats, prefilled study file).
2. `G` branched: `branched from the-aperture-save.json at recorded action #4 —
   the recorded line is replayed to digest 792bacc22341, new actions from here
   are branch history`, with `Pass priority   [CR 117.3d, 117.4]` on offer.
3. The finished 4-seat save was placed at the load path and `L` was pressed:
   `loaded the-aperture-save.json — digest c76c56bd72fd (stored c76c56bd72fd), 1104 action(s) in the replay`.
4. `R` opened `REVIEW — omniscient postgame view · game over · acting is disabled`
   with `POSTGAME OMNISCIENT REVIEW …`, and `Leave review (R)` was present in both
   the top bar and the strip.

Screenshots of the real window (desktop grab, cropped to the app rectangle):
`journey-final-review-app.png` (window B, review open) and `smoke-4seat.png`
(the packaged `--smoke` render of a four-seat opening table, final binary).

Bounded caveats: the driver delivered keyboard input by `PostMessage`
(unverified by the driver itself), so every claim above is taken from a
subsequent fresh AX capture of the same window; one `P` press inside review
produced no visible change in the captured message line, so the read-only
refusal is evidenced by the automated test (which asserts the "read-only"
message) rather than by that keypress.

## What was not done

No merge, no master push, no issue closure, no successor activation. The
branch-comparison view is deliberately action-kind only; it is a study aid, not
a hidden-information oracle. Scope stays bounded 28-card four-seat practice with
one supported commander per seat.
