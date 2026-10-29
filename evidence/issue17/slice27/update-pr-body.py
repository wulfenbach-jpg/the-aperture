import json
from pathlib import Path

root = Path("C:/AI/aperture-worktrees/issue-17/evidence/issue17/slice27")
body = (root / "pr-body-before.md").read_text(encoding="utf-8")

old_state = """- Catalog **142 real cards**.

- Desired eight-list union **459 names = 130 supported + 329 missing** (per-list missing `55/0/51/48/23/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **23 missing names**.

### Latest increment: slice25 — Exuberant Wolfbear (independently approved increment)"""

new_state = """- Catalog **143 real cards**.

- Desired eight-list union **459 names = 131 supported + 328 missing** (per-list missing `55/0/51/48/22/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **22 missing names**.

- Last **behavior** change: slice 26, `Ursine Champion` (code head `9cd0b68`). Latest head: slice 27 record repair (`27d8f48`), comment-only source edits, no behavior change.

### Latest increment: slice 27 — record repair for review 280 (no behavior change)

Round-2 review of the slice-26 delta found **no functional defect** and four record defects, three of them in files slice 26 itself edited. All four are corrected on this head:

1. `crates/core/src/layer.rs` (module header and the `Layer` enum doc) and `engine.rs::recompute_layers` still described the engine as implementing **only layer 7c** and called layers 4-6/7b unimplemented. `Layer::Type` (4), `Ability` (6), `Setting` (7b) and `Modifying` (7c) all exist and are exercised by slices 23-26; the documents now state the four implemented layers and scope the gap to layers 5 (colour change), 7a and 7d.
2. `docs/issue17-starter-corpus.md` described "7b colour changes" as missing. Colour change is layer 5; 7b (base power/toughness setting) has been implemented since slice 25. The sentence now reads layer 5 colour changes plus non-creature animation as the remaining layer gap.
3. `crates/core/src/card.rs` still said Little Bear's counter is decided by the target's *printed* subtype. The branch reads `Game::has_subtype` (effective, layer-4 aware); the comment now says so.
4. `evidence/issue17/slice24-journey-driver/src/main.rs` asserted the pre-`c2d9b35` event wording (`"enters with a +1/+1 counter and gains haste"`), so the shipped driver panicked at the recorded head and the slice-24 evidence was not reproducible from the record. The one-line correction is now **committed**, the worktree is clean, and the driver re-run from the committed bytes reproduces **terminal `Win{P3}`, 1602 actions / 37 turns / 4127 events, digest `a41bbfd2…`** with `demo-save.json`/`demo-replay.json` byte-identical (`slice27/slice24-driver-head-run.log`, `slice27/slice24-artifact-hashes.txt`).

Also corrected: `80fff73` is described as **comment-only source edits with no behavior change** (`crates/core/src/{ability,catalog,engine}.rs`, `tests/issue17_slice26.rs`), not "evidence-only".

Verification at `27d8f48` (all run this round): `fmt --check` clean, `clippy --workspace --all-targets --locked -D warnings` clean, **122 focused tests pass**, workspace **113 suites / 798 passed / 0 failed**, and an independent corpus audit recomputed from `catalog.rs` + the deck files with a manifest-equality assertion: **143 / 459 = 131 supported + 328 missing**, per list `55/0/51/48/22/52/52/53`, **1 of 8 certified** (`slice27/corpus-audit.py`, fails on any mismatch).

### Behavior increment: slice 26 — Ursine Champion (record corrected, code `9cd0b68`)

- Exact M19 304 printing `e60b6c02-c1f1-4ab9-bd62-ae3b447a898f`, oracle `4f142d99-8146-4b16-a2cc-ac03da19b960`, retrieved `2026-10-27T09:59:58Z`; raw response, stamp and fixture `fixtures/starter-corpus/slice26-sourced.json` retained. Text: `{5}{G}: This creature gets +3/+3 and becomes a Bear Berserker until end of turn. Activate only once each turn.`
- The pool's first **layer 4** (CR 613.1d) effect: `Layer::Type`, a closed `layer::SubtypeLine`, `ContinuousKind::SetCreatureSubtypes`, `Effect::SetSourceCreatureSubtypes`, and `Game::has_subtype` as the effective-subtype query now used by the Bear/Human target filters, Little Bear's counter branch, Ayula's fight revalidation and the Surrak entered-Bear filter. The subtype line is replaced, not added to; the +3/+3 applies after it by layer order and both expire in cleanup (CR 514.2). The once-each-turn limit reuses the restriction `Rootwalla` already prints - no new activation-limit machinery.
- 6 slice-26 tests (196 focused across 15 suites), two mutation REDs regenerated in byte mode with a byte-for-byte restore proof, and a real four-seat/40-life 40-card package journey: terminal `Win{P3}`, 1366 actions / 34 turns / 3613 events, digest `86bbd60b…`, 9 casts, 14 type-change events, separate release-CLI replay matching. Slice-25 replay still `4d6ea58e`.

### Historical increment: slice 25 — Exuberant Wolfbear (independently approved increment)"""

assert old_state in body
body = body.replace(old_state, new_state)

# Slice 25 section body stays; fix its trailing current-state line.
old25 = "Current142catalog/459union=130supported+329missing; per-list55/0/51/48/23/52/52/53. Only1of8desired100cardstarterscertified, Ayula23missing."
new25 = "At that head: 142catalog/459union=130supported+329missing; per-list55/0/51/48/23/52/52/53; only1of8certified, Ayula23missing (current totals are in Current state above)."
if old25 in body:
    body = body.replace(old25, new25)

(root / "pr-body-after.md").write_text(body, encoding="utf-8")
print("bytes", len(body), "lines", body.count("\n") + 1)
print("catalog line ok:", "- Catalog **143 real cards**." in body)
print("slice27 ok:", "### Latest increment: slice 27" in body)
print("slice26 ok:", "### Behavior increment: slice 26" in body)
print("slice25 ok:", "### Historical increment: slice 25" in body)
