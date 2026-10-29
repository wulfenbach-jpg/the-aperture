# Review 296 — slice-34 spec split, contract lens with a call-site audit

Reviewed head: **`6575c8b224607b709be94d08f873d6e54f87a6de`** (code head `5b60c2c`, base `cff319d`).
Reviewer evidence: this directory (untracked). No implementation file was edited by the reviewer; every build/run outside
the checkout lives in `C:/AI/r296cand` (export of the reviewed head) and `C:/AI/r296base` (export of `cff319d`).

**Verdict on the slice-34 delta: APPROVED_INCREMENT — no blocking defect.**
**Verdict on the original contract of issue #17: STILL UNFINISHED**, so this card returns to sourced implementation, not completion.

## 1. The supervisor's question: every call site of the widened spec

`TargetSpec::CreatureYouControl` changed meaning (it now includes the ability's own source; the new
`AnotherCreatureYouControl` is the only spec that removes it). The reviewer enumerated every ability the pool ships
(`review296_specs.rs`, output `ability-specs.tsv`, 55 ability rows) and checked each "you control" target against its
printed text:

| Card | Printed target wording | Spec at this head | Correct? |
|---|---|---|---|
| Beorn's Hospitality | "put a +1/+1 counter on **target creature you control**" (live Scryfall `hob/120` re-fetched this run) | `CreatureYouControl` | **correct** — no "another"; the animated source is a legal target, which is the defect review 290 found |
| Dancing from Dark to Dawn | "put X +1/+1 counters on **target creature you control**" | `CreatureYouControl` | **correct** — same printed wording; source is an enchantment when the trigger fires, so the widening is unreachable today, as disclosed |
| Little Bear | "untap **another** target creature you control" | `AnotherCreatureYouControl` | **correct** — the only printing that narrows the set |
| Beorn the Fierce | "put a trample counter on **up to one** target creature you control" | `UpToOneCreatureYouControl` | **correct, unchanged** (that spec already included the source) |

No spell `SpellSpec` uses the spec (`catalog.rs` uses `AnyTarget`/`Creature`/`Player`/`Spell`/`NoncreatureSpell`/`None`),
no other card prints "target creature you control", and no other crate matches on the variant (the workspace compiles
clean under `-D warnings`). `grep -n exclude crates/core/src/engine.rs` shows the parameter is read only inside
`legal_targets`' arms, so there is no second place carrying the old exclusion.

## 2. Behaviour before/after, by execution (not by the handoff)

One probe file (`review296_probe.rs`, sha256 `ee10e982…`), byte-identical in the reviewed export and in the `cff319d`
export, run against both:

| Observation | base `cff319d` | reviewed `6575c8b` |
|---|---|---|
| `legal_targets(P0, CreatureYouControl, source)` for the animated Hospitality | `offered=[] contains_source=false` | `offered=["Beorn's Hospitality"] contains_source=true` |
| landfall after the animation, source the only creature | `counters=0 pending=false stats=4/4` (trigger dropped) | `counters=1 pending=false stats=5/5` |
| Little Bear's ETB offer list beside two other creatures | `["Exuberant Wolfbear", "Grizzly Bears"]`, count 2, source not in offer | identical |
| Little Bear alone | `TriggerRemoved` (CR 603.3d), no counter, no pending | identical |

So the flip changes exactly the printed "target creature you control" set and leaves Little Bear's offers and its
zero-legal-target removal bit-for-bit the same. Logs: `probe-base.log`, `probe-candidate.log`.

## 3. Verification actually run by the reviewer

| Check | Result |
|---|---|
| `cargo test --workspace --all-targets --locked` | exit 0 — **846 passed / 0 failed / 1 ignored** over 117 test binaries (`workspace-tests.log`) |
| `cargo test -p aperture-core --test issue17_slice33 --test card_printings --locked` | **12/12** and **12/12** |
| `cargo fmt --all --check` | exit 0 (`fmt.log`) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 (`clippy.log`) |
| My own re-run of both slice-34 mutations, out of tree | reinstating `o.id != exclude` fails **4** tests (both self-target tests + the two P/T tests); pointing Little Bear at the plain spec fails **exactly 1**; both files restored with a stable sha256 (`mutations296-run.log`, `mutation-*.log`) |
| Journey rebuilt from my own `git archive` of the reviewed head | terminal `Win{P3}`, seed 3333, **1117 actions / 24 turns / 2967 events**, digest `3dfb14d4ce76…`, snapshot `a7a45e4b…`, 29 on-turn / 28 off-turn CDA sightings, 2 animations, 13 landfall triggers (`journey-run.log`) |
| Generated artifacts | `demo-journey/save/replay.json` regenerate **byte-identically** (`1ee05e63…` / `c040689b…` / `e47bb078…`) |
| Separate release `aperture-cli replay` of my generated replay | `matches recorded`, 1117 actions, 2967 events, exit 0 (`cli-replay.log`) |
| Historical replays at this head | slice24 `697a846d`, slice25 `4d6ea58e`, slice26 `86bbd60b`, slice29 `ac16e510`, slice33 `3dfb14d4` all verify |
| My own corpus audit (`audit296.py` + a supported-name dump from this very build) | **149 catalog / 459 union = 137 supported + 322 missing**; per list `55/0/51/48/16/52/52/53`; every list's recorded `missing_cards` equals my recomputation; **1 of 8** lists fully supported (`corpus-audit-296.log`) |
| Live exact-head CI | `6575c8b`, `63fc378`, `bfbbceb`, `b74815d` — **10/10 check runs completed SUCCESS, every `head_sha` keyed to that commit** |
| `f3e628d` record | 2 `board and roadmap` success; the six Rust job runs **and both `windows package` runs cancelled** — recorded as cancelled, not as a pass |
| Local / remote / PR head | `6575c8b` == `6575c8b` == `refs/pull/46` head; PR **OPEN, draft, BEHIND** master |
| Delta scope | `git diff --stat cff319d..6575c8b` touches only `crates/core/src/{ability,card,engine}.rs`, `crates/core/tests/issue17_slice33.rs`, `docs/`, `evidence/`; **no** `crates/ai`, `crates/ui`, `crates/import`, `decks/`, `fixtures/` |
| Review-290 record items | `card_printings` corrected to 12/12, `cff319d` corrected to 10/10 owned runs, "with none" now qualified by "includes the source" |

## 4. Findings

No blocking finding. The four record corrections demanded by review 290 all landed, and the two slice-33 tests that were
updated now assert *more* (5/5 with the counter, then 4/4 after stripping it) rather than less.

Non-blocking, recorded for the next increment:

1. **The mutation receipts bind the checkout bytes, not the git blob.** `pre/post-mutation-ability-sha256.txt` name
   `dff6b60d…`, which is `crates/core/src/ability.rs` as checked out (CRLF); the blob at `HEAD` is `fdeab8b5…` (LF, and
   `core.autocrlf=true` in this repo). `engine.rs` happens to be LF in both, so `08988a74…` matches the blob exactly.
   `sha256sum -c` succeeds against the file the mutation ran on, and my own out-of-tree re-run discriminates the same way,
   so this is a wording precision issue ("the committed ability.rs"), not a false RED. Say "the checked-out file at the
   reviewed commit" next time.
2. **No committed runner for the two slice-34 mutations.** Slice 33 ships `run-mutations.py`; slice 34 shipped only the
   receipts and RED logs, so regenerating them needs a script that is not in the tree. I re-ran both by hand
   (`mutations296.py`) and they behave as claimed; committing the runner would make the record self-reproducing.
3. **`f3e628d`'s cancelled set is understated** — the checkpoint says "its three Rust jobs were cancelled"; both
   `windows package` runs were cancelled as well. The entry already says it is recorded as cancelled rather than a pass,
   so this is completeness, not a false pass.

## 5. Original issue #17 status

| Acceptance criterion | State at this head |
|---|---|
| 1. At least eight supported 100-card lists | **1 of 8** (`jasmine-vanilla-stompy`, 61 names) |
| 2. Every included card and interaction implemented with conformance coverage | **322 of 459** union names unsupported; Ayula 16 short |
| 3. Four-seat games on exact deck snapshots | journeys are four-seat exact-snapshot **40-card packages**, not one of the desired 100-card lists |
| 4. Document exclusions | recorded in `docs/issue17-starter-corpus.md` and the checkpoints |

Next sourced work remains the unfinished contract (Ayula's remaining names before any whole-list certificate), not more
polish of this delta. No merge, no master push, no issue closure, no downstream unlock.
