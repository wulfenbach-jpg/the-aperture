"""Apply slice-23 documentation edits (counts + new section) to docs/issue17-starter-corpus.md."""
import io
import pathlib

root = pathlib.Path("C:/AI/aperture-worktrees/issue-17")
p = root / "docs/issue17-starter-corpus.md"
s = p.read_text(encoding="utf-8")

old_counts = """* **127** are in the supported 139-card catalog and **332** are missing. Slice
  20 adds `Vivien's Grizzly` to the previously verified 124/335 split and slice
  22 adds `Dancing from Dark to Dawn` to the previous 126/333 split; the union
  is unchanged because both were already referenced names."""
new_counts = """* **128** are in the supported 140-card catalog and **331** are missing. Slice
  20 adds `Vivien's Grizzly` to the previously verified 124/335 split, slice 22
  adds `Dancing from Dark to Dawn` to the previous 126/333 split, and slice 23
  adds `Goreclaw, Terror of Qal Sisma` to the previous 127/332 split; the union
  is unchanged because all three were already referenced names."""
assert old_counts in s
s = s.replace(old_counts, new_counts)

section = """
## Slice 23: Goreclaw, Terror of Qal Sisma
Goreclaw (`{3}{G}` Legendary Creature — Bear 4/3, Scryfall CMM 293, immutable
printing `ee47f23a-3ba9-4615-b170-c89d8ab99d78`, oracle
`befb211f-37ca-4083-98d4-9ff1f28be3f2`, retrieved 2026-10-27T07:38:22Z, sourced
in `fixtures/starter-corpus/slice23-sourced.json`) adds the pool's first
**static cost reduction** and its first **layer 6 ability grant**.

* "Creature spells you cast with power 4 or greater cost {2} less to cast."
  `StaticAbility::CreatureSpellsYouCastWithPowerCostLess` is applied in
  `effective_cost`, after commander tax, to the *generic* part of the cost only
  and never below zero (CR 601.2f, 118.7a). The reduction reads the printed
  power of the card being cast: no supported effect changes a card's power
  outside the battlefield, so no hidden-zone recalculation exists. It applies
  only to creature spells of the permanent's controller — a noncreature spell
  and an opponent's spell are unaffected.
* "Whenever Goreclaw attacks, each creature you control with power 4 or greater
  gets +1/+1 and gains trample until end of turn." `TriggerEvent::Attacks`
  (slice 19) stages the trigger; resolution reads each battlefield creature's
  *current* power, so counters and earlier pumps count. The pump is an ordinary
  layer 7c `ContinuousEffect`; the trample is a new `ContinuousKind::GrantsTrample`
  in the new `Layer::Ability` (CR 613.4d), read by combat damage assignment
  through `Game::has_trample`. Both expire in cleanup (CR 514.2) and are dropped
  when their target leaves the battlefield, exactly like the existing pumps.

Known limits, disclosed rather than approximated: the cost reduction counts one
reduction per permanent (correct for one Goreclaw, untested for two because the
pool has no second copy); the pool still has no layer 4/5/7b (type, colour, base
power/toughness) effects, so cards such as `Circle of the Moon Druid` and
`Stuffed Bear` remain unsupported. Ayula has 25 missing names after this slice,
the eight-list gate remains open at 1 of 8 certified, and no new list is
certified by this slice.

Three mutation REDs are retained in `evidence/issue17/slice23/`: disabling the
reduction fails exactly the two cost tests, ignoring the printed power threshold
fails exactly the qualifying-creature test, and dropping the grant lookup from
`has_trample` fails exactly the trample combat test.
"""

marker = "\n## Slice 23"
if marker not in s:
    s = s.rstrip("\n") + "\n" + section
p.write_text(s, encoding="utf-8", newline="")
print("docs updated")
