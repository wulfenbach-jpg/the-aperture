# Issue 13 runnable candidate

Delivery policy: the user's October 24 `DELIVERY_FIRST.md` supersedes the earlier exhaustive evidence requirements. Preserve historical failures; do not interpret this as full Commander acceptance.

## Implemented
- Explicit bounded `commander-combat-practice-40` profile; both named 20-life profiles retained.
- Combat damage tracked independently by stable commander identity and recipient, with 21+ damage loss. Noncombat damage excluded; control/zone-return and Fog prevention covered by focused tests.
- Eliminated-player owned objects removed, survivor-owned still-controlled objects exiled, surviving APNAP choices resumed after their source leaves. The accepted departing-current-chooser regression is restored without weakening its assertions.
- Save v9 / replay v5; source-left APNAP continuation has serialized validation context. Older saves are refused explicitly.

## Executed delivery checks
- `cargo test -p aperture-core --test apnap --test issue13_commander_combat --test issue13_loss --locked`: 60 passed, 0 failed.
- `cargo build --workspace --release --locked`: pass.
- `cargo fmt --all --check` and `git diff --check`: pass.
- Actual packaged release CLI, four seats, seed1316440: terminal after3372 actions, 40 starting life, 33 commander-damage events, 3 commander-loss events. Separate replay command succeeds and digest matches.
- Raw checks: `run165-focused-tests.log`, `run165-release.log`, `run165-demo-summary.json` in retained local evidence; executable game/replay/logs included in the downloadable package.
- Required remote CI and one practical independent delivery review remain gates, recorded in the PR/task rather than preclaimed here.

## Run
Extract `aperture-issue13-commander-practice.zip` and double-click `RUN-DEMO.cmd` on Windows x64. It runs the bounded four-seat demonstration and verifies its replay. No model service/network is required.

From a release source checkout, equivalent command:
```
aperture-cli selfplay --commander-combat-practice --deck plains.txt --deck forest.txt --deck forest.txt --deck forest.txt --commander "0:Isamaru, Hound of Konda" --commander "1:Isamaru, Hound of Konda" --commander "2:Isamaru, Hound of Konda" --commander "3:Isamaru, Hound of Konda" --seed 1316440 --max-steps 12000 --out demo-report.json --replay-out demo-replay.json
aperture-cli replay --file demo-replay.json
```
The package contains 100-Plains/100-Forest fixture files. P0 alone has white mana, making the commander-loss demonstration reproducible. This is NOT Commander-legal deck or playing-strength evidence.

## Limits / deferred risk
Headless practice only, one supported Isamaru per seat, 22-card pool. No native Commander table, partners, arbitrary cards/decks, partial prevention/redirection, continuous control effects or non-card stack support. Non-sandbox midpending CommanderReturn/CommanderDraw continuation and Fog-specific AI/native consumer evidence remain deferred, not newly accepted. Direct-restored loss/control fixtures are not modeled concessions. Historical feature-first/TDD and assignment-accounting deviations remain disclosed in preserved histories. This candidate is not a full Commander/trainer/v1 release; no merge, issue closure or successor activation by this implementation worker.
