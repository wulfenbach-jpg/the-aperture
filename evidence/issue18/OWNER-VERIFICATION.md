# Issue 18: owner delivery verification

Bounded native practice, not full Commander or a trainer. Implementation is ready for one independent practical review; no merge/issue closure is authorized to this worker.

## Delivered

- Native 2/3/4-seat engine-backed tables. Ordinary20-life default remains available; bounded40-life starts one supported Isamaru per seat and now uses the compiled Plains-containing command library, so commanders can actually be cast. Explicit/imported lists remain exact.
- All seats' battlefield/public zones, life, commander/tax/damage ledger and status. Only the human hand is projected; other hands/libraries remain counts. Pending decisions and legal actions use engine ownership/submission checks. Sandbox counters are explicitly storage/display only.
- Persistent primary decisions, parent table scrolling, public-zone/card inspection, text zoom and Tab/Enter/shortcuts. Native minimum lowered to1024x640; actual minimum acceptance was exercised at1280x720 client pixels at125% Windows DPI, as well as the larger viewport.
- Async save/load and stale generation guards retained; native loads accept existing supported ordinary/command profiles.

## Executed checks

- Owner: `cargo test -p aperture-ui --all-targets --locked`: **41 passed** (11 lib,11 import/async,19 session), zero failed.
- Owner: `cargo fmt --all --check`, workspace/all-targets locked Clippy with `-D warnings`: PASS.
- Fresh locked release built and portable ZIP integrity checked; the executable extracted for the final journey matches `package-manifest.json`.
- Reused implementation's focused affected core controls: command_zone_setup33, issue13_commander_combat43, m1_multiplayer28; no unrelated historical matrix repeated.
- Real defect RED→GREEN: command-zone inspector falsely said battlefield; removed the unsupported location assertion. Retained `inspect-red.log` and focused regression.
- Default bounded library RED→GREEN: previously no white source; final regression drives actual heuristic/legal actions and observes an Isamaru cast, with ordinary defaults and explicit overrides covered.

## Actual packaged input, not screenshot-only

`PACKAGED-JOURNEY.json` records the final extracted binary's real Windows UIA input/readback sequence (fresh control lookup for every input): 4seats → bounded40 → Start → Keep → Save → New → Load. It starts from the ordinary two-seat default. Saved/loaded profile is CommanderCombat40, four ordered seats, four commanders, pending P1 mulligan, one accepted action. The stored full digest is `f05c25867f617ba052b55344e621721c655f636cc0f1d133d93055aa42415f3f`; UI readback matches its prefix after replacing the game.

The native owner also used app-scoped `computer_use` background UIA/keyboard input and verified subsequent captures: real Keep moves P0→P1; AI autoplay advances AI-seat choices; New retains four seats/40life; keyboard L restores the save; Tab focuses A− and Enter changes100→90%; A+ changes100→110%. Public-zone and card inspection were invoked and visibly read back. At1920x1080 physical client, all four panels/primary controls remained available. The inspector repair was re-exercised in a freshly packaged binary. Earlier interaction evidence is reused only where the behavior was unchanged; the final default-library repair received the full setup/Keep/save/new/load journey again.

At1280x720 physical client, the four panels and primary strip are usable; larger content is intentionally scrollable. Renderer tests separately execute the actual parent layout at1280x720/1920x1080 and exercise scrolling rather than an isolated panel. The driver's background SB_LINEDOWN/pixel input was a verified no-op on egui; this is not claimed as a successful native wheel test. Semantic UIA control input and keyboard navigation did work; no foreground escalation, personal-app interaction or process killing was used.

## Scope and remaining gate

The supported28-card pool is bounded practice: no arbitrary100-card Commander, partners, legend-rule support, measured AI strength, trainer or v1. Combat damage division remains the disclosed automatic simplification. Existing engine/reporting limitations are not recertified or silently fixed. The Windows portable build is unsigned and requires extraction to a writable directory; no GPU/model service is required.

The focused PR must pass protected exact-head checks, then receive **one concise independent practical review**. Integration supervisor owns protected merge, merged-tree verification, public issue closure and next-card activation. All six child cards are unassigned product backlog, not review lanes; untouched.

Local reporting: inspected the actual stale issue13 pointer (not assumed issue11), added a tested report-only native-owner phase, retargeted reporter and passive alert to issue18, and rendered a current report. One new focused test plus17 pointer regressions passed. Existing scheduler cadence/IDs and disabled legacy OMP revival watcher were preserved; no delivery acknowledgement is invented.
