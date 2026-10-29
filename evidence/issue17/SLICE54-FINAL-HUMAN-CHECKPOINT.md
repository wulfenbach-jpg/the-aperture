# Slice54 — Ursine Monstrosity shipped as a candidate; original #17 unfinished

One complete printed card is newly enabled. Ayula falls from four to **three** missing cards/interactions; the corpus remains **1 of 8 certified**. This is an implementation handoff for independent increment review, not self-approval or full #17 acceptance.

## Exact candidate and preserved ownership

- Same task, writer history, branch `aperture/issue-17`, draft PR46.
- Accepted base: `dc9bb3c0788df022853e5bdcbc00cbfab8035963` (Ruxa approved by review340).
- Initial Ursine implementation: `7fa3b133ec231dc85e140c7823973fa5b51c0a4e`.
- Native functional repair: `2d5cc18c35eb9f3c33c71a1f2bf0e5605d340e7a`.
- **Final code/corpus and local-gates head: `1b134db04cd2ddf03520c47384bfef7e1695d935`.**
- This report will be published in a later evidence-only commit. The native task's post-publication metadata names that actual tip separately. No CI pass is claimed for the later evidence tip.

The original OMP JSONL was resumed and appended, not replaced by a blank worker. Native inspected and co-authored the corrections, so native is **not** the independent reviewer. Other inherited session edits, reviewer archives and untracked files were excluded. No new team/nested agent, global config change, merge, master push, issue closure or child unlock.

## Printed source and implemented clauses

**Ursine Monstrosity**, DSC36, printing `73cc6df4-3564-4ace-bf8a-eac3e62d725a`, Oracle ID `647feda9-2876-4430-902f-10dc1f4f99a9`. Exact source and retrieval stamp `2026-10-28T23:54:49.142002+00:00` are in `fixtures/starter-corpus/slice54-sourced.json`; raw provider response and retrieval receipt are under `slice54/sources/`. Native also read the exact live printing and its official-source rulings before integration.

The actual `{2}{G}` 3/3 Bear Mutant is offered/cast/paid normally and has trample. Its beginning-of-combat trigger mills one, chooses a live opponent with the serialized seeded engine RNG, creates an attacks-that-player-if-able requirement **for this combat**, and grants indestructible plus fixed **+N/+N** until end of turn. N counts distinct supported card types in its controller's graveyard **after milling**. CR611.2d fixes that value on resolution; it is not a continuously recalculated graveyard static.

Indestructible participates in lethal-damage SBA and destruction handling; zero toughness is still a graveyard move. EndCombat removes the attack requirement while pump and indestructible remain until cleanup. Source-left resolution still mills; the focused real Unsummon control proves no grant survives on the departed source in hand at the settled boundary. The ordinary source-incarnation invalidation uses the existing `affected` binding; this report does not claim a new legal flash-recast test for this non-Flash card.

## Concrete defects caught and repaired before delivery

1. Initial native inspection caught the incorrect +N/+0/dynamic model. The worker corrected it to the existing fixed layer7c P/T modifier before its local commit. **Initial mechanic construction was implementation-first; no strict TDD RED is claimed for it.** The original tests had passed the wrong model; do not read the earlier draft's phrase “tests exposed it” as genuine pre-fix assertion evidence.
2. **Real native RED: attack requirement bypass.** At `7fa3b13`, a four-seat declaration could send an able Ursine at an unrelated opponent, obeying zero requirements. Unequal conflicting groups also offered zero/one obeyed requirement where two were possible. `native-red-attack-requirements.log`: compiling suite, **2 passed/2 failed, EXIT101**. Minimal repair compares obeyed requirements with the maximum achievable under the inherited single-defender model (CR508.1d). The same controls now pass; every emitted offer validates and the pending has a continuation.
3. **Real native RED: default-policy consumer.** Once the engine refused that bypass, the unchanged heuristic proposed the wrong defender at actual game step622. `native-red-policy.log`: compiling behavioral failure, **0/1, EXIT101**. A narrow public-offer adapter now applies only when no-attack is unavailable; unrestricted policy behavior remains unchanged. No search ranking, budget, sampling, model, provider or hidden-state input change. Ordinary heuristic suite15/15 and the full real four-seat default-policy regression1/1 pass.
4. The worker's first corpus script checked missing-card arrays but not interaction prose. The strengthened whole-field audit genuinely failed on its stale Ursine interaction (`native-red-whole-field.log`, EXIT1). The **whole Ayula field** now contains only the three real missing names. All eight interaction fields are scanned for stale supported-card citations; the three Ayula citations must exactly equal its missing cards. Final audit passes.

The first native test compile attempt used nonexistent P2/P3 aliases; it is retained as `native-compile-attempt-FAILED.log`, **not** a behavioral RED. `SLICE54-HUMAN-CHECKPOINT.md` and `slice54/RESULT.json` remain the pre-native drafts; this final checkpoint and `slice54/FINAL-RESULT.json` supersede them. Historical initial journey artifacts remain untouched, not silently regenerated to hide the changed rules.

## Final executable gates

`slice54/run-native-gates.sh` logs every command and its **own** EXIT in `slice54/native-gates.log`, headed by full `1b134db`:

- fmt0.
- **9 core suites /107 passed/0 failed**: Ursine4, native controls6, printings19, catalog5, slice15 corpus2, commander combat43, replay6, Witness14, starter corpus8.
- **2 separate AI suites /16 passed/0 failed**: ordinary heuristic15 and the Ursine real-game compatibility regression1.
- Thus **11 suites /123 passed** is an aggregate of those TWO test commands, not a full-workspace total. No retained search tests are included.
- Whole-field corpus audit0; workspace/all-target warnings-denied Clippy0; locked **workspace** release0; real journey and separate release CLI replay0.
- Test delta versus the accepted base: **11 new tests** across three new files (4+6+1); existing suites are not counted as new tests.
- No fresh local full-workspace test matrix is claimed. Exact code-head CI supplies that gate; its separate receipt is authoritative.

**Exact-code-head CI: 10/10 completed SUCCESS, all keyed to full `1b134db04cd2ddf03520c47384bfef7e1695d935`, no rerun requested.** Receipt: `slice54/exact-head-ci-1b134db.json`. The later evidence-only published tip is named separately in the native handoff and has no CI claim of its own.

## Real four-seat games, exact package snapshots, separate replays

Final evidence driver: `slice54/run-native-journey.sh`; receipt `slice54/native-journey.log`; artifacts under `slice54/native/`. Four seats,40life,seed2323, exact `decks/package-slice54.txt` snapshots, Jasmine commanders. **WinP3,1801 actions,39turns,4596events**, digest `cebf64e0243ea2925dcfe11b191e7bb1b64a3e631fe4ac834432104652ff35fb`. Authoritative events count eight Ursine casts,30 Ursine combat-trigger event lines,55 combat-damage events. The “combat_mill_events” field counts the trigger's event line, not an assertion that every resolution necessarily moved a library card. Fresh non-sandbox save/load and the separate release-CLI replay reproduce the exact state/digest (EXIT0 each).

Default product heuristic is separately executable, not only an evidence policy: `native-default-policy.log`, `native/default-report.json` and `native/default-replay.json`. **WinP2,1752 actions,38turns,4463events**, digest `2bd43d639ed51c7a70e5321c961ccf09471d8c4b60af8ac888198672c672e255`; real CLI selfplay0 and separate replay0/match. This verifies legal continuation, **not playing strength**.

Both are **40-card evidence packages, not 100-card starter certificates**. The old pre-repair recording under `slice54/` has different actions/digest because it accepted wrong-defender attacks; it is retained as superseded evidence, not certified or claimed replay-compatible. No approved prior-slice fixture/expected digest was changed.

## Corpus and next gate

Catalog **163**; eight-list union **459 =150 supported+309 missing**. Per-list gaps **[55,0,51,48,3,52,52,53]**; **1/8 certified**. Ayula's remaining names in both fields: **Dúnedain Rangers, Evercoat Ursine, Rampaging Yao Guai**.

The inherited combat model permits only one defending player per declaration. Conflicting Ursine requirements are maximized within that disclosed model, not presented as full multi-defender Commander combat. Only the six currently implemented card types can appear in supported graveyards; unsupported card types/cards do not become accepted through this feature. No new GUI interaction or arbitrary Archidekt support is claimed.

Children inspected: `t_26a7a4ff` is the scheduled certified-input evaluation wait; `t_3c2fe51a` is unassigned release-soak backlog. Neither is a review child; both remain gated. Next: one concise independent practical review of this increment, then SAME writer/history/card/branch/PR46 completes `slice54/NEXT-CARDS.md`. Run the exact100-card four-seat certification and separate replays only once both Ayula gaps are zero; report2/8 only on actual success. Do not complete original#17 or unlock dependents on an increment approval.
