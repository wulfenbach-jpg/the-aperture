# Issue 17 — supported starter corpus

Status: **PARTIAL**. One starter package is implemented and exercised; the
eight-list release gate in the public issue remains open. Only
`jasmine-vanilla-stompy` is supported. Every other entry below is a **planned**
archetype with an exact sourced **desired** list under
`decks/starter-corpus/planned/`, and **no** claimed card support, playability or
certification.

The machine-readable target list is
`decks/starter-corpus/manifest.json` (schema `version: 1`). Its `status` field is
`planned`, `implemented` or `certified`; planned entries have a non-null
`deck_path` pointing at their exact desired 99-card library, but remain
engine-unsupported and non-playable. `deck_path` is not a support claim.

## Target archetypes

| id | commander | style | status |
| --- | --- | --- | --- |
| `isamaru-white-aggro` | Isamaru, Hound of Konda | white aggressive one-drop | planned |
| `jasmine-vanilla-stompy` | Jasmine Boreal | green/white vanilla-creature stompy | implemented |
| `talrand-draw-go` | Talrand, Sky Summoner | blue draw-go spells-matter | planned |
| `krenko-goblin-tokens` | Krenko, Mob Boss | red goblin token swarm | planned |
| `ayula-bear-counters` | Ayula, Queen Among Bears | green bear tribal +1/+1 counters | planned |
| `elas-sacrifice` | Elas il-Kor, Sadistic Pilgrim | white/black sacrifice/drain | planned |
| `tatyova-landfall` | Tatyova, Benthic Druid | green/blue landfall value | planned |
| `zada-spells` | Zada, Hedron Grinder | red single-target spell copying | planned |

All eight entries now have an exact desired 99-card library file. Only Jasmine's
is supported and playable; the other seven are **planned metadata**, never
supported defaults. Their `deck_path` points under
`decks/starter-corpus/planned/`, and their cards are mostly outside the current
114-card pool. Isamaru itself is supported, and the slice-4 package implemented
one of its cards (Soul Warden), so `isamaru-white-aggro` now lists 56 missing
cards; it stays `planned` because 56 of its chosen cards are still unsupported.
Slice 5 implemented one card in each of four other planned lists:
`Stitcher's Supplier` (elas), `Ruin Crab` (tatyova), `Negate` and `Preordain`
(talrand), `Pyretic Ritual` and `Seething Song` (zada). None of them is close to
complete, so all seven remain `planned`.

## Corpus union and staged gaps

Exact all-name totals over the eight desired lists, including commanders and
basic lands (coordinator evidence
`evidence/issue17/slice5/update_manifest.py` + `slice5-corpus-state.json`):

* union of all referenced names: **459 unique names**;
* **102** are in the supported 114-card catalog and **357** are missing;
* the manifest's `missing_cards` per list is computed from the same catalog
  membership and sums to the same verified contents. Planned lists remain
  metadata only; Jasmine is the only implemented/playable starter.
* Structure per list is still 99 library cards + a separately named commander;
  the Swamp basic in Elas is now supported, but the rest of that desired list
  still needs its unsupported cards and interactions.

Staged interaction roadmap (derived from the manifest `missing_interactions`):

1. tokens and token permanents: **partially implemented** as typed 1/1 red
   Goblin tokens for the Krenko package (Krenko, Dragon Fodder, Krenko's
   Command, Hordeling Outburst, Goblin Instigator, Beetleback Chief) and a 2/2
   blue Drake with functional flying for Talrand, Sky Summoner. Ayula and Zada
   still need their own token types; the exact Krenko starter is still planned
   (51 missing) and Talrand's desired list has 54 missing cards;
2. triggered abilities: **bounded implemented package** for Talrand's
   controller-only instant/sorcery cast trigger (staged above the spell,
   independent of counters), **bounded implemented** landfall for Tatyova and
   Jaddi Offshoot (life then draw, controller-only), and the slice-4
   "another creature enters" trigger for Soul Warden (fires for creatures the
   Warden's controller does not control, never for the Warden itself). Slice 6
   adds the "creature dies" watcher family: any creature (Blood Artist), a
   creature you control including the watcher itself (Zulaport Cutthroat), and
   "another creature you control" (modelled, no card in the pool uses it yet).
   Card deaths and creature-token deaths alike fire them. Tribal triggers and
   "creature card put into a graveyard from anywhere" triggers remain
   unimplemented (Ayula, Syr Konrad);
3. activated non-mana abilities: **bounded implemented package** for Krenko's
   `{T}` Goblin-count token production (including >255 counts) and the existing
   Prodigal Sorcerer/Rootwalla/Brindle Boar abilities. Sacrifice outlets and
   broader activated abilities remain unimplemented (Elas);
4. keyword semantics: **implemented** for printed Flash (Ashcoat Bear may be cast
   at instant timing) and printed Trample (Owlbear assigns lethal damage to
   blockers first and tramples the surplus to the player); haste (Krenko) and
   every other keyword remain unimplemented;
5. counters as rules objects (Ayula);
6. single-target spell copying and go-wide pump (Zada);
7. legend rule, equipment and Aura permanents (Isamaru and any future legend);
8. black basic land support (Elas): **implemented** as the sourced Swamp basic;
9. graveyard filling (all four lists): **bounded implemented package** — mill
   from the top of a library as a real zone move (Stitcher's Supplier mills its
   controller on entering *and* on dying; Ruin Crab mills each opponent on
   landfall). Milling an empty library does nothing and never loses the game,
   because that is a draw-only state-based action (CR 704.5b). Slice 6 adds the
   first **targeted** player effect: `TargetSpec::Player` with "target player
   mills three cards" (Hedron Crab), together with "target player loses N life"
   and "each opponent loses N life" as life loss that is *not* damage (CR 118.2),
   so damage prevention cannot stop it. Anything that *reads* the graveyard is
   still unimplemented;
10. stack interaction (Talrand): **bounded implemented package** — "counter
    target noncreature spell" as a real target restriction (Negate may not name a
    creature spell on the stack, CR 205.2a). "Counter target spell" was already
    supported (Counterspell);
11. mana acceleration (Zada): **bounded implemented package** — adding red mana
    to the pool without tapping a permanent (Pyretic Ritual, Seething Song), and
    Preordain's scry-2-then-draw as an ordered two-effect resolution;

Elas still has 55 unsupported desired-list nonland names plus sacrifice outlets,
tutor/reanimation interactions and keyword text on its own commander (Elas
prints Deathtouch, which the rules layer does not model, so Elas itself stays
refused at import); its Swamp requirement is now met by the supported basic land,
and Blood Artist plus Zulaport Cutthroat now supply the death-drain payoff.
None of these planned lists is called supported, legal-for-play or certified.

## First implemented package: jasmine-vanilla-stompy

* Commander: **Jasmine Boreal** — `{3}{G}{W}`, legendary 4/5 Human with no rules
  text. It is a genuine printed vanilla legend and is registered in the typed
  `CardId::SUPPORTED_COMMANDERS` allow-list next to Isamaru.
* Library: **99 cards** in
  `decks/starter-corpus/jasmine-vanilla-stompy.txt`, commander supplied
  separately:
  * **58 unique nonland cards**, all singleton, all supported: 47 newly sourced
    vanilla green/white creatures, 11 previously supported green/white
    nonland cards (four vanilla creatures, four instants, three creatures with
    already-tested typed abilities);
  * **41 basic lands**: 20 Plains and 21 Forest.
* Total with the commander: 100 cards. No card in the library is legendary, so
  the unimplemented legend rule is never exercised. No nonbasic singleton card
  is duplicated. There is no snow, artifact, enchantment creature, token,
  counter, keyword or unsupported rules text in the package.
* Sourced printings: `fixtures/starter-corpus/sourced-cards.json` records the
  concrete Scryfall printing, printing URL, Oracle fields and retrieval time for
  every referenced card. Raw provider responses are in
  `evidence/issue17/scryfall-responses.json`.
* Offline analyzer snapshots: `fixtures/starter-corpus/` contains the
  `aperture.commander-deck.v1` list snapshot, the
  `aperture.card-metadata.v1` card snapshot and a fixture manifest with the
  bound SHA-256 digests. These use the existing `commander-check` analyzer; no
  parallel legality engine was added.

Reproduce the source capture and snapshots:

```
python evidence/issue17/source-starter-cards.py
python evidence/issue17/generate-starter-fixtures.py
cargo run -p aperture-cli -- commander-check \
  --deck fixtures/starter-corpus/jasmine-vanilla-stompy.deck.json \
  --cards fixtures/starter-corpus/jasmine-vanilla-stompy.cards.json --json
```

The analyzer reports format legality `legal`, one commander, 99 library cards,
100 total, 61 unique canonical names and engine support `supported`. Its
`gameplay.available` field remains `false` with the pre-existing blanket reasons;
that refusal was deliberately not relaxed. Bounded command-zone and
Commander-combat practice are still **not** the Commander format.

## Phased gaps (why the eight-list gate is open)

1. **Card-support surface.** The implemented pool is 88 cards. Non-vanilla
   archetypes need token creation, activated abilities beyond land mana,
   triggered abilities, keyword semantics, counters as rules objects and
   static/layer effects that this slice does not implement. Those are the
   `missing_interactions` entries in the manifest.
2. **Commanders.** Six target commanders are not in the supported pool.
   Extending the typed registry is deliberately explicit: adding a name to
   `SUPPORTED_COMMANDERS` without implementing its printed text would fake
   support.
3. **Exact lists.** Every planned target has an exact sourced desired list under
   `decks/starter-corpus/planned/`, and its `deck_path` points at it. Jasmine's
   implemented list is directly under `decks/starter-corpus/`.
   A desired-list path is a planning artifact, not support: every planned list is
   engine-unsupported, non-playable and non-certified. Jasmine is the only list
   built entirely from supported cards; Tatyova now has a bounded landfall
   package (Tatyova, Benthic Druid + Jaddi Offshoot with defender). A 90-land pile or duplicated nonbasic
   singleton cards would not be a meaningful style and is not used to satisfy
   counts.
4. **Certification.** `implemented` means an exact list exists, is analyzer
   legal/supported and has played an exact four-seat bounded game. It does not
   mean eight-list certification, independent review or merged CI.

## Exclusion policy

* Only cards whose complete printed text maps to implemented engine semantics
  may enter the catalog. Unsupported text means the card is refused, never
  approximated.
* Genuine vanilla creatures are preferred for volume: they add no hidden
  keyword, token, counter or replacement semantics.
* Legendary cards are commander-only. The legend rule is not modelled, so a
  supported commander is refused in ordinary starting libraries and no other
  legend may enter a starter library.
* Nonbasic singleton cards may appear once per list. Basic lands are the only
  repeatable cards.
* Source data must come from an authoritative printing (Scryfall printing
  endpoint or official Gatherer) with a retrieval date. No card name, cost,
  type line, power/toughness or Oracle text is authored by hand.
* Format legality, engine support and actual gameplay support are reported
  separately. A legal/supported list is still not Commander format gameplay.

## Practical limitations

* The game/replay journey is a local 2–4-seat deterministic heuristic run under
  the explicit bounded 40-life Commander-combat practice profile; it is not
  networked multiplayer or full Commander.
* The AI is the existing deterministic heuristic. This package adds no
  strategy tuning.
* The native UI's bundled table still defaults to the practice decks; the
  starter corpus is exercised through the CLI with explicit deck paths.
