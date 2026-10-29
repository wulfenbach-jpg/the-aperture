# Issue17 slice7 — repaired and integrated candidate, independent delta review pending

Full issue17 is **UNFINISHED**: eight desired starter lists, one implemented, zero certified. This is a corrective increment, not eight-list acceptance. No merge to master, master push, force push, issue closure or successor activation.

## Exact candidate

- Existing draft PR46, branch `aperture/issue-17`.
- Local/remote/PR head: **7e1eb5f1b8218e24d7a03377c3cb3c5bb0981a4f**.
- PR OPEN, still draft, now **MERGEABLE** after serial integration.
- Master integrated: **8de0961370c595f82f3e1d7505b37c7e67958183**.
- Work order forbids force push; the integration skill's normal merge-master-into-feature fallback was used, preserving all earlier feature and evidence commits. Master itself was not modified.
- No non-evidence/source changes after the tested/published head. This checkpoint and the evidence archive are attachments, not new untested commits.

## Repair and integration history

1. Native coordinator reproduced run224's unmodified external probe: two failures / one passing control. A dying Zulaport drained twice; Zulaport+Jackal drained three times instead of twice.
2. Saved OMP writer resumed on the exact existing history; `f1b7ab6` retains the original battlefield identity in each death record and excludes the separately dispatched self-death from the watcher batch. Three fully drained tests were added (single watcher, both batch object orders, surviving watcher control); expected totals were not relaxed.
3. `edd8008` integrates merged master/#16 counters, revealed state, keyword and token APIs. Conflict paths: core `card.rs`, `catalog.rs`, `engine.rs`, `state.rs`, README and HANDOFF. Base-derived import/UI/AI/search files are not new corpus authorship.
4. Coordinator rejected the initial integration's token semantic downgrade and deleted substantive conformance assertions. Same writer repaired it at `04c39e6`: real destination/death event and applicable watchers, then token cessation at the SBA checkpoint. Tokens are finally removed rather than retained as `Zone::Removed` tombstones. Corrected #16 tests that asserted an invented 'instead of exile' replacement; preserved real event ordering and LKI life gain. Typed counters, keywords and settled-state validation remain integrated.
5. Coordinator authored `7e1eb5f`: genuine RED/GREEN regression for a remaining contradictory Swords trace ('instead of being exiled' after a real exile), fixed that trace and token module documentation. Restored historical slice3 audit files after the writer accidentally reran an old script. Current corpus audit is new `slice7-native-audit.py`/`.json`; historical evidence is retained.

No new card was added. Printed-card provenance remains in existing sourced fixtures with retrieval dates.

## Verified at exact published head

- `cargo fmt --all --check`: PASS.
- `cargo test --workspace --all-targets --locked`: **639 passed / 0 failed**, 90 test binaries/summaries. No failures or ignored-case substitution.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS.
- `cargo build --workspace --release --locked`: PASS.
- Original external `review224-probe`: **3 passed / 0 failed** with full stack/trigger/resolving/pending drainage. Single watcher ends [21,19,19,19]; two dying creatures [22,18,18,18], both creation orders; surviving control unchanged.
- Real release CLI exact four-seat 40-life package game, seed610: terminal P3, **2952 steps / 66 turns**, 73 queued triggered abilities / 62 resolved abilities. Final elimination by opponent drawing an empty library, not a claim that every opponent lost to life drain.
- Separate replay: **7239 events**, digest **8e0ab4b00e7ff6d018b71ee9317f59c6d794e12472d83bdf4fe2d278c4a3ca8e**, matching. It differs from pre-integration slice6 digest; nothing was rewritten to hide that.
- Native audit compares each of four replay seat snapshots to `decks/package-slice6-dies.txt`: exact match, no bundled replacement.
- All eight desired deck contents unchanged from pre-repair `3987dd6`; each library remains 99 cards.
- Exact-head GitHub Actions: **10/10 completed SUCCESS**, all `head_sha` equal 7e1eb5f...; required-check query PASS. Receipts: `slice7-exact-head-ci.json`, `slice7-required-checks.json`.
- Public PR body edited and read back exactly; tracked worktree clean after publication.

### Reproduce the representative package game

```
target/release/aperture-cli.exe selfplay --seed 610 --max-steps 5000 --deck decks/package-slice6-dies.txt --deck decks/package-slice6-dies.txt --deck decks/package-slice6-dies.txt --deck decks/package-slice6-dies.txt --commander-combat-practice --commander "0:Isamaru, Hound of Konda" --commander "1:Isamaru, Hound of Konda" --commander "2:Isamaru, Hound of Konda" --commander "3:Isamaru, Hound of Konda" --out evidence/issue17/slice7-native-journey.json --replay-out evidence/issue17/slice7-native-journey.replay.json
target/release/aperture-cli.exe replay --file evidence/issue17/slice7-native-journey.replay.json
```

This is a **40-card package**, not one of the desired 100-card starters and not Commander certification. It has no white mana: Isamaru/Soul Warden are not cast; no commander-cast or commander-damage coverage is inferred from this journey. Focused tests, not the terminal outcome alone, prove the repaired dispatch/token rules.

## Corpus and release gaps

Native recomputation: **106 real catalog cards**, exact desired union **459 = 94 supported + 365 missing**, **1 implemented / 0 certified** lists. Missing names per list: Isamaru56, Jasmine0, Talrand52, Krenko51, Ayula47, Elas55, Tatyova54, Zada55. All manifest missing-name sets match the live catalog. No list or acceptance criterion changed to improve these numbers.

Seven desired starters still lack card/interaction support. Jasmine has no missing card names, but has not been independently certified as full Commander gameplay; the existing analyzer's bounded-gameplay distinctions were not relaxed. Legend rule, equipment/auras, sacrifice outlets, tutors/reanimation and broader commander mechanics remain real corpus gaps. #16 typed counter/keyword/library primitives now exist, but their existence does not implement the missing printed cards. Arbitrary Archidekt decks and measured playing strength remain unclaimed. No #26 certified evidence/#32 release soak unlock.

## Next gate and exact histories

**One concise independent practical delta review is still required**, on the exact published head. Do not re-audit approved slices1–5 or rebuild the old application. Adjudicate S6-R2-F1 with the unchanged external probe and inspect identity-preserving death dispatch plus the actual conflict resolutions/token ordering against #16 and the previous corpus. Use focused controls; reuse the fresh 639-test/build/CI receipts. Native coordinator implemented the final trace-only production repair, so it is NOT an independent approval.

After repair/integration delta approval, return continued corpus work to this SAME card/branch/PR; do not mark the original #17 DONE, merge/close it or activate certified downstream lanes from an increment verdict. Prefer certifying a genuinely complete sourced starter if feasible; otherwise continue sourced missing-card packages. No human/quota block is present.

Existing exact histories (preserved, no replacement team):
- writer: `C:/AI/aperture-worktrees/issue-17/evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl`
- reviewer: `C:/AI/aperture-worktrees/issue-17/evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl`
- both complete directories backed up before this slice in `evidence/issue17/slice7-backup/`; final backup in `slice7-final-backup/`.
- run overlay `.starter-run.yml` preserves global configuration and disables paid fallback. Actual writer binding: `opencode-go/deepseek-v4.1-flash`.
- Resume reviewer with `omp -p --no-pty --auto-approve --no-title --config .starter-run.yml --model opencode-go/deepseek-v4.1-flash --session-dir evidence/issue17/review-sessions --resume <exact-reviewer-jsonl> --tools read,bash,grep,glob,write --max-time 20m @<scoped-review-assignment.md>`.

Both writer runs stopped exit0. First launch with unsupported `ls`/`find` tools failed before work; the corrected launch used real registered tools, and failure log is retained. No reviewer was launched concurrently or silently duplicated; canonical native delta review is the next owner. Reporter/ops routing was not changed by this corpus repair.
