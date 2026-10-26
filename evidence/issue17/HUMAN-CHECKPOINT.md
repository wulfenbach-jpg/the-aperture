# Issue 17 — runnable partial checkpoint, 2026-10-25

## What is usable
The first exact 100-card starter works in the headless CLI: Jasmine Boreal green/white stompy, 99 library cards plus its commander, 41 basics and 58 singleton nonlands. The implementation adds Jasmine and 47 real sourced vanilla creatures (catalog 31 -> 79). Existing ordinary defaults are preserved.

The actual four-seat 40-life exact-list game terminated after 2364 actions / 53 turns. All four replay libraries equal the published list; five Jasmine casts and 30 distinct new vanilla cards were exercised. Separate replay and save/replay matched digest d7f8c2adaa17d906bdaaed28f9482589faa65c5863d1f134377ca45665ddab48. Capped name-regression probes are recorded separately, not counted as games won.

## Publication and acceptance boundary
Draft PR46: https://github.com/wulfenbach-jpg/the-aperture/pull/46
Exact reviewed/local/remote head: 86db7d316edd6200f7f598849018e428d11b69eb.
Independent review-2 approved this bounded increment after both reproduced findings were repaired. All 10 exact-head CI checks completed SUCCESS, including both Windows package/reproducibility checks; configured required checks pass. Tracked tree is clean. No merge, master push, public issue closure or successor activation.

**Full issue 17 is NOT accepted.** One list is implemented; seven exact desired lists are sourced plans, not playable/certified lists. No list is labeled certified. All eight desired snapshots include 459 distinct names counting commanders and basics: 67 currently supported, 392 missing; manifest missing union exactly matches the catalog comparison. The separately documented library nonbasic union is 447 = 61 supported +386 missing. Planned lists analyze format-legal but engine-unsupported. Arbitrary Archidekt/Commander compatibility and AI strength remain unclaimed.

## Launch from this checkout (Windows cmd or PowerShell)
Worktree: C:/AI/aperture-worktrees/issue-17, branch aperture/issue-17.

```text
cargo build --release --locked -p aperture-cli
target/release/aperture-cli.exe selfplay --seed 1 --max-steps 12000 --deck decks/starter-corpus/jasmine-vanilla-stompy.txt --deck decks/starter-corpus/jasmine-vanilla-stompy.txt --deck decks/starter-corpus/jasmine-vanilla-stompy.txt --deck decks/starter-corpus/jasmine-vanilla-stompy.txt --commander-combat-practice --commander "0:Jasmine Boreal" --commander "1:Jasmine Boreal" --commander "2:Jasmine Boreal" --commander "3:Jasmine Boreal" --out evidence/issue17/try-game.json --replay-out evidence/issue17/try-game.replay.json
target/release/aperture-cli.exe replay --file evidence/issue17/try-game.replay.json
```

This is CLI-only starter acceptance, not a new native Jasmine setup control. The native table retains its existing Isamaru profile/defaults. The analyzer's gameplay.available=false refusal remains intact.

## Durable evidence / exact history
Within this worktree:
- docs/issue17-starter-corpus.md; decks/starter-corpus/manifest.json; fixtures/starter-corpus/.
- evidence/issue17/IMPLEMENTATION-HANDOFF.md and IMPLEMENTATION-HANDOFF-2.md; TDD.md; REPAIR-HANDOFF.md.
- Failed review-1.md/.json preserved; approved review-2.md/.json. Approval receipt also posted and read back on PR46, comment6395664107.
- final-ci.json; corpus-union-audit.json; exact game/replay and review-owned run logs.
- Writer history: evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl.
- Independent reviewer history: evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl.

Slice1 used five numbered OMP assignments, maximum one worker live at a time, one implementation writer, review only after writer stop, preferred Go/DeepSeek route with no fallback model changes. Ordinary OMP, not Vibe. Global configuration preserved; no purchases or new cards/teams/automation.

## Next gate / continuation ownership
Renew slice2 on THIS SAME native card under the existing continuous authorization; no new user approval is needed. Resume the preserved writer and implement the next bounded missing-card/interaction package from the sourced manifest, then exact-list games and independent review. Keep PR46 draft and #17 open until all original eight-list requirements are independently accepted. A partial increment approval is not permission to complete #17 or release dependent cards.

The existing continuous-release supervisor a60a00413961 remains enabled every15minutes and is the durable recovery owner after this bounded checkpoint; its enabled state was read back. No new schedule was created. Native scheduling of this same task at the runtime reserve is a continuation handoff, not a quota/auth/needs-input block or a claimed fresh worker launch.

## Disclosures
- Existing issue14 v6 evidence replay has the same digest mismatch on untouched base8a72cc6 and this candidate (legacy-replay-note.md). No new state/digest/version fields were added. This inherited recording issue is not silently recertified.
- Reporter active-phase.json still points at completed m3-tabletop/#20, independently read back. Shared ops/reporting files, five-hour reporter, passive warning and old paused recovery watcher were not changed. Do not present that old report as current issue17 progress.
