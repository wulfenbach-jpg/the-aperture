import subprocess

body = subprocess.run(['gh', 'pr', 'view', '46', '--json', 'body', '--jq', '.body'],
                      capture_output=True, text=True, check=True).stdout

marker = '## Historical checkpoints below'
head = body.split(marker, 1)[0]
tail = body.split(marker, 1)[1]

new_head = """## Partial starter corpus — issue #17 (draft, not acceptance)

**Current slice48 candidate, not independently reviewed yet.** Approved increments 1-47 preserved (latest independent verdict: review327 APPROVED_INCREMENT at `1b1c542`, with its three record items closed here). Original eight-list contract unfinished; **1 of 8 certified**. No merge/master push/closure/downstream unlock.

### Current state (slice 48)

- Catalog **161** real cards; eight-list union **459 = 148 supported + 311 missing**. Per-list gaps `55/0/51/48/5/52/52/53`. **Ayula 5 short**; desired 100-card snapshots unchanged.
- **Record items from review 327 are closed.** R327-1: the slice-47 claim that pre-change saves "still verify their own digest" was false — executed here, `aperture-cli replay --file evidence/issue17/slice45/demo-save.json` parses, re-executes and fails `digest 99bc8446… != recorded 45d674d2…` (exit 1); both sites now say so, and a legacy mid-combat save (no declaration record) simply answers "not alone" for the rest of that combat. Version decision recorded in four places: **`SAVE_VERSION` stays 10, `REPLAY_VERSION` stays 6**, because a bump would reject every older artifact including the pinned pre-#16 v10 fixture; the limitation is user-facing under README "Versioned persistence". R327-2: the journey row now cites `journey-build.log` (EXIT=0) and the failed `--locked` log is kept, renamed `journey-locked-attempt-FAILED.log`, labelled. R327-3: code head and evidence tip are stated separately. Also corrected: `starters[elas-sacrifice]` "53" → 52, and the declaration record being outside `GameState::validate` is written up as a known weakness.
- **Owlbear Cub (CLB 246) implemented** from its captured printing: intervening-if on the *defending* player's land count (CR 603.4, checked at declaration and at resolution); a **look, not a search** (CR 701.20a/e) — `Pending::SearchLibrary` gained a `looked` list so only the examined cards go to the bottom in a random order and the rest of the library is never reordered; the put creature enters tapped and attacking and is **deliberately absent from `declared_attackers`** (CR 508.1 makes "attacked" a declaration, CR 506.5 reads "declared"), so the Cub still counts as having attacked alone; `GameState::validate` constrains the new pending (top-of-library anchor, creature-only eligibility, one card).
- Code head `3e7675a3d9768a2c7e64fec4b6be91313ffe9043`; final evidence tip is this branch head.
- Execution at this head: `cargo test -p aperture-core --locked` **674/0**, `cargo test --workspace --locked` **134 binaries / 933 passed / 0 failed / 1 ignored exit 0**, fmt 0, `clippy -p aperture-core --all-targets -D warnings` 0, `cargo build --workspace --release --locked` 0.
- Two **discriminating controls** prove the new tests are not vacuous: with the printed condition replaced by `Always` the seven-land test fails; with a whole-library shuffle instead of the targeted bottom placement both ordering tests fail (logs `slice48/control-*.log`, exit 101).
- **Both four-seat journeys executed**: the unchanged slice-46 package re-executes **byte-identically** (`da4eacd5…`, 8098 events), and a Cub package (`decks/package-slice48.txt`) plays terminal `Win{P2}`, 3462 steps, 8666 events, digest `2b9f3b3d…` with **12 Cub casts, 5 looks, 4 puts tapped-and-attacking, 5 bottom placements**. Both replay in a separate release CLI and reload from their own `SessionSave`. The package is NOT 100-card certification / product AI / strength.
- Checkpoint `evidence/issue17/SLICE48-HUMAN-CHECKPOINT.md`; `evidence/issue17/slice48/RESULT.json`. **Next gate: independent practical delta review** (`1b1c542..` tip), then the same writer/history for the remaining five Ayula cards and the exact 100-card certification. Implementation-first deviation disclosed; this run co-authored the increment and is not its own reviewer.

"""

open('evidence/issue17/slice48/pr-body.md', 'w', encoding='utf-8').write(new_head + marker + tail)
print('written', len(new_head))
