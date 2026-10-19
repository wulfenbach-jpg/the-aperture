# Supervisor acceptance — runnable practice slice

This is a bounded original Rust MTG practice simulator, not the full research plan and not Commander. The implemented catalog and current limits are authoritative in `README.md` and `aperture-cli pool`.

## Verified independently

- Final source gates: `cargo fmt --check`, `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --release` all exit 0. The final test total is **136 passed, 0 failed**, parsed from real Cargo output in `evidence/supervisor-final-verification.json`.
- Fresh heuristic selfplay seeds 101–108 in both deck orders: **16 terminal games**, no truncation, and **16 separately executed replay digest matches**. Raw commands/results are in `evidence/supervisor-matrix/manifest.json`. These demonstrate execution/reproducibility, not AI strength.
- A fresh live HTTPS Archidekt request using a normal slug URL fetched deck 26206384. It reported 28 unsupported cards with explicit reasons and correctly refused launch. No claim is made that an arbitrary EDH list can play in this tiny pool. See `evidence/supervisor-import-live.json`.
- A separate real Rust-adapter probe used Decider for P1: **15 model decisions**, 50 forced decisions without calls, and 55 heuristic decisions for the other seat. The probe stopped at its declared 120-step cap and has **no winner**; its replay matches. The loaded model's health read-back reports Vulkan and zero CPU fallbacks. See `evidence/supervisor-model.json`, `supervisor-model-replay.json` and final verification JSON. This is integration evidence, not a strategy benchmark.
- Native desktop interactions were exercised through background UI Automation: both seat file fields were populated with the two bundled practice lists, each Import file button was invoked, both lists certified, and Start imported match created a new game with those exact deck snapshots. Save, New game, and Load restored digest `04c215a30522`; the saved file was read back to check the actual deck lists. These used text-file imports, not invented successful live Archidekt URLs.
- The packaged native executable was run from outside the source directory using `--smoke`; its actual framebuffer was captured and it exited successfully. The packaged CLI also executed `pool` successfully. No model weights, private access settings, or card artwork are included.

## Supervisor-found defects repaired

The first independent pass found invalid normal-card metadata handling, rejection of ordinary URL slugs, silent quantity clamping, an import panel that only reported rather than applied decks, blocking UI network work, relative life totals mislabeled as absolute seats, ignored model-input truncation, incorrect lethal-first rule coaching, and opponent pending-choice controls. The second worker fixed these with regression evidence in `evidence/supervisor-fixes-tdd.log` and details in `FIX_HANDOFF.md`.

A final native layout check found that accumulated board/log/import content clipped the second seat and Start button. An isolated import-panel scroll fix passed its first test, but the independent reviewer correctly identified that the real parent could still consume all height. The final fix scrolls the real application body. The regression now renders the same `render_ui` used by eframe at the actual 1100×760 minimum, proves the Start text is initially outside the viewport, feeds wheel events, and requires the complete button text to enter its clip rectangle. It was observed failing before the application-body fix and passing afterward (`evidence/viewport-actual-frame-red.log`, final test log).

A separate actual-frame test exposed the remaining P1-human header life-label inversion; it was observed failing with `P0 17 / P1 3`, then fixed to render the public absolute seats correctly (`evidence/ui-p1-life-red.log`). The isolated first RED log remains as historical evidence, not a claim about final line numbers.

## Run

Portable folder: `dist/TheAperture/`. Double-click its `launch.cmd` for baseline opponent autoplay; read `START_HERE.txt`. The zip is `dist/TheAperture-Windows.zip`.

Source launcher: `launch.cmd` in this project. Optional Decider requires the separate existing loopback model lab; it is off by default. Background model/import work cannot apply stale results to a replaced game.

## Explicit remaining limits

15-card pool; 1v1 practice profile; text-only cards; no Commander, multiplayer pods, general triggers/replacements/layers, tokens or broad card support. Counters are sandbox storage/display, not combat effects. Multiple-blocker combat damage uses a disclosed automatic allocation rather than full player-directed assignment. Coaching explains implemented rules; no calibrated win percentages, move grading or strong search engine is claimed.

The upstream engines surveyed in the planning stage were not compiled or used as runtime dependencies. Passing these tests does not certify full Magic rules compliance.
