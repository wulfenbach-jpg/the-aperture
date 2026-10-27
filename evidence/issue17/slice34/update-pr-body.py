#!/usr/bin/env python3
"""Refresh PR46's body for the slice-33 increment and the slice-34 review-290 repair.

Deterministic surgery on the body read back from the PR: the "Current state"
bullets are replaced and one new section is inserted directly after them.
Everything else is left untouched. The script is idempotent (it refuses to run
if the old state block is absent) and the caller reads the body back afterwards.
"""
import pathlib
import subprocess
import sys

HEAD = sys.argv[1]
BODY = pathlib.Path(sys.argv[2])
REPO = pathlib.Path(__file__).resolve().parents[3]

old_state = """- Catalog **148 real cards**.

- Desired eight-list union **459 names = 136 supported + 323 missing** (per-list missing `55/0/51/48/17/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **17 missing names**.

- Last **behavior** change: slice 32, `Kamahl's Summons` plus `Surrak and Goreclaw`'s printed trample repair (code head `40a87aa`). `Beorn the Fierce` (slice 31) remains the previous behavior increment.
"""

new_state = """- Catalog **149 real cards**.

- Desired eight-list union **459 names = 137 supported + 322 missing** (per-list missing `55/0/51/48/16/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **16 missing names**.

- Last **behavior** change: slice 33, `Beorn's Hospitality` (first layer 7a characteristic-defining power/toughness, code head `01fdc3b`), repaired in slice 34 (`5b60c2c`) after review 290; `Kamahl's Summons` plus `Surrak and Goreclaw`'s printed trample (slice 32) remain the previous increments.
"""

section = """### Latest increment: slice 33 — Beorn's Hospitality, with the review-290 target repair in slice 34

`Beorn's Hospitality` (`{1}{G}` Enchantment; Scryfall **HOB 120**, immutable `153ca57e-30f0-4ad7-ae9d-c55cbf0fd4c9`, oracle `d9ed7252-11d1-432b-9101-ac08cd28826d`, retrieved **2026-10-27T18:21:29Z**; every printing checked for one identical text) prints "Landfall — Whenever a land you control enters, put a +1/+1 counter on target creature you control. / {5}{G}{G}: This enchantment becomes a Bear creature in addition to its other types and gains \\"This creature's power and toughness are each equal to the number of lands you control.\\" (This effect doesn't end.)"

* **Layer 7a now exists.** `Layer::CharacteristicDefining` (CR 613.4a) with `ContinuousKind::PowerToughnessEqualToLands`, recomputed from the board on every layer pass, so a land entering after the animation raises the P/T with no second activation (CR 604.3). A 7b snapshot would fail the test that proves it.
* **A duration that does not end.** `EffectDuration::WhileSourceOnBattlefield` on an activated ability, because the card prints "(This effect doesn't end.)" (CR 611.2b): cleanup leaves it, departure ends it (CR 400.7).
* **Reviewed repair (slice 34).** Review 290 showed the landfall trigger reused `TargetSpec::CreatureYouControl`, which removed the ability's own source to model Little Bear's "another". The spec is split: `CreatureYouControl` is the printed set **including** the source, and the new `AnotherCreatureYouControl` is the only one that excludes it (Little Bear keeps it). Five regressions cover the animated self-target, the source offered beside another creature, the unanimated source not being legal, the CR 603.3d removal when no creature exists, and Little Bear's excluded source.
* **Verified at `bfbbceb`** (review 295 APPROVED_INCREMENT): `issue17_slice33` 12/12, `card_printings` 12/12, workspace 119 suites / 846 passed / 0 failed / 1 ignored, fmt and clippy `-D warnings` clean, two discriminating mutations (source re-excluded: 4 failures; Little Bear pointed at the plain spec: 1 failure) with `sha256sum -c` clean pre/post receipts, a four-seat / 40-life exact-snapshot journey rebuilt at the repaired head (terminal `Win{P3}`, 1117 actions, 2967 events, digest `3dfb14d4ce76…`, demo artifacts byte-identical, separate CLI replay matching) and 10/10 exact-head CI SUCCESS keyed to `b74815d` and to `bfbbceb`.
* The journey is a 40-card evidence package driven by a masked demonstrator, **not** a 100-card certificate and not product AI or strength evidence. Original #17 remains unfinished: 1 of 8 lists certified, 322 of 459 union names unsupported.
"""

text = BODY.read_text(encoding="utf-8")
if old_state not in text:
    raise SystemExit("the current-state block this script replaces is not present; refusing to rewrite")
text = text.replace(old_state, new_state + "\n" + section, 1)
BODY.write_text(text, encoding="utf-8")
print("body rewritten:", BODY, len(text), "chars")
