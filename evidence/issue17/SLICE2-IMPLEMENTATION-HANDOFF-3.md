# Issue 17 slice 2 assignment 3 — Talrand package handoff

Talrand, Sky Summoner is now a complete supported commander package: printed
cost/type/P-T, a real instant-or-sorcery cast trigger, an engine-only 2/2 blue
Drake token with functional flying, and flying block restriction. The exact
100-card Talrand desired list remains `planned` with 54 missing names. No commit,
push, merge or #16 integration claim.

## Sourced printing

* Talrand: stored real planned-list printing, `otc/116`,
  `{2}{U}{U}`, Legendary Creature — Merfolk Wizard, 2/2, text
  “Whenever you cast an instant or sorcery spell, create a 2/2 blue Drake
  creature token with flying.” Retrieved `2026-10-25T08:32:00Z`.
* Drake: live Scryfall token search, `tfdn/8`, Token Creature — Drake, 2/2,
  Flying, blue identity.
* Fixture: `fixtures/starter-corpus/talrand-sourced.json`. Tokens are engine-only
  (`catalog::by_name`/`all` exclude them; deck parser refuses them).

## Behavior

* The cast trigger is queued for the caster’s permanents only and is put on the
  stack **above** the spell before priority; opponents’ spells do not trigger it.
* The trigger resolves even if the spell is later countered (regression test
  casts Opt, then Counterspell; Drake appears and Opt reaches the graveyard).
* Flying is real in blocker validation: a nonflyer cannot block a flyer
  (`CR 509.1b` refusal), and a flyer may block either. `sample_blocks` now skips
  illegal flying blocks instead of proposing them.
* Assignment 2’s CR 111.7/111.8 token death/zone/SBA behavior is retained; the
  Drake uses the same token object/creation/cessation path.
* No new serialized state fields; old Jasmine replay still reproduces
  `d7f8c2adaa17…`.

## Genuine RED (after adding compile-scaffold symbols, not counted as RED)

```
casting_an_instant_stages_the_trigger_above_it_and_creates_a_drake
  assertion failed: "spell + trigger"
    left: 1   right: 2      # no trigger was queued/stacked; no Drake

nonflyers_cannot_block_flyers_and_the_refusal_preserves_state
  panicked: expected rejection   # engine accepted a nonflyer blocking a flyer
```

Fix: typed `TriggerEvent::SpellCast` + Talrand TriggeredAbility +
`Effect::CreateDrakeToken`; `apply_cast` queues and stages caster-only triggers
above the spell; `CardDef::flying`, `Object::has_flying`, blocker validation and
`sample_blocks` restriction.

The countered-spell trigger test was added after GREEN as regression coverage,
not claimed as RED. No implementation-first history is rewritten.

## Tests / gates

```
cargo test -q -p aperture-core --test issue17_talrand --test issue17_tokens \
  --test catalog_pool --test card_printings --test issue13_commander_combat \
  --test starter_corpus -p aperture-import --test starter_corpus_planned \
  --test starter_corpus_analyzer
  -> 6, 10, 5, 4, 43, 8, 1, 3 passed
```

`cargo fmt --all --check`, clippy `-D warnings`, release CLI build: exit 0
(`slice2-talrand-gates.log`, `slice2-talrand-focused.log`).

## Executable journey

`decks/package-talrand-drakes.txt` is a clearly named **40-card supported package
fixture**, not the desired starter. Four seats, each the package + Talrand
commander, Commander-combat 40 life, seed 11:

* **terminal**, P2 wins, 1830 steps / 39 turns
* 29 casts, **11 Drake creations logged**, 3 blocks, 3 counters
* actual log lines show Talrand triggers going on the stack and the Drake entering
  under the triggering player’s control (not assumed equal to cast count)
* digest `c9fb97c6fc08…`

Separate replay and `--save-out` save both reproduce the digest
(`slice2-talrand-replay.log`, `slice2-talrand-save-replay.log`). Old Jasmine
replay still matches (`slice2-talrand-jasmine-replay.log`).

## Corpus / manifest

Only `missing_cards` was recomputed against the 86-card pool: Talrand’s desired
list now has 54 missing names. All eight exact desired list files are unchanged;
Talrand remains `planned`, never replaced by the package fixture.

## Timing correction

Assignment 2 stopped after roughly **7–8 active minutes**, not because its
25-minute budget was exhausted. Its preserved handoff sentence claiming the
repairs consumed the runtime budget is not supported; the file is retained
unchanged as history.

## Remaining gaps

* Talrand’s 100-card desired list remains planned/unsupported (54 missing).
* Flying exists only for the typed Drake token; no other keyword semantics.
* Tokens remain the Goblin and Drake types only.
* Legend rule absent; Talrand and Krenko are commander-only.
* No #16 integration/approval claim.
