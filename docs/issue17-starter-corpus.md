# Issue 17 — supported starter corpus

Status: **PARTIAL**. One of the eight desired lists is now **certified end to
end** from its exact snapshot; the eight-list release gate in the public issue
remains open. Only `jasmine-vanilla-stompy` is certified. Every other entry
below is a **planned**
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
| `jasmine-vanilla-stompy` | Jasmine Boreal | green/white vanilla-creature stompy | certified |
| `talrand-draw-go` | Talrand, Sky Summoner | blue draw-go spells-matter | planned |
| `krenko-goblin-tokens` | Krenko, Mob Boss | red goblin token swarm | planned |
| `ayula-bear-counters` | Ayula, Queen Among Bears | green bear tribal +1/+1 counters | planned |
| `elas-sacrifice` | Elas il-Kor, Sadistic Pilgrim | white/black sacrifice/drain | planned |
| `tatyova-landfall` | Tatyova, Benthic Druid | green/blue landfall value | planned |
| `zada-spells` | Zada, Hedron Grinder | red single-target spell copying | planned |

All eight entries now have an exact desired 99-card library file. Only Jasmine's
is supported, playable and certified; the other seven are **planned metadata**, never
supported defaults. Their `deck_path` points under
`decks/starter-corpus/planned/`, and their cards are mostly outside the current
134-card pool. Isamaru itself is supported, and the slice-4 package implemented
one of its cards (Soul Warden), so `isamaru-white-aggro` now lists 55 missing
cards; it stays `planned` because 55 of its chosen cards are still unsupported.
Slice 5 implemented one card in each of four other planned lists:
`Stitcher's Supplier` (elas), `Ruin Crab` (tatyova), `Negate` and `Preordain`
(talrand), `Pyretic Ritual` and `Seething Song` (zada). None of them is close to
complete, so all seven remain `planned`.

## Corpus union and staged gaps

Exact all-name totals over the eight desired lists, including commanders and
basic lands (coordinator evidence
`evidence/issue17/slice13/corpus-audit.py` + `slice13/corpus-state.json`):

* union of all referenced names: **459 unique names**;
* **122** are in the supported 134-card catalog and **337** are missing;
* the manifest's `missing_cards` per list is computed from the same catalog
  membership and sums to the same verified contents. Planned lists remain
  metadata only; Jasmine is the only certified/playable starter.
* Structure per list is still 99 library cards + a separately named commander;
  the Swamp basic in Elas is now supported, but the rest of that desired list
  still needs its unsupported cards and interactions.

Slices 11–13 add the islandwalk bears `Pale Bears` and `River Bear`, Sol Ring,
and Professor of Zoomancy from the Ayula list. Islandwalk uses CR 702.14c and
the defending player's board. Slices 14–17 add the dynamic mana stones,
Werebear, the card-costed Bear makers, `Fade from History` and `Little Bear`
(slice 16), and Friendly Teddy, Ulvenwald Bear and Caller of the Claw
(slice 17). Ayula currently has **31** missing names.

Slice 16 is the first time "each player who controls ..." creates the token
(the token's controller is the permanent's controller, not the spell's), and
the first simultaneous mass destruction (`Fade from History` destroys every
artifact and enchantment in one event with one dies-watcher batch). `Little
Bear` reads its counter condition off the *target's* printed subtype, so a
non-Bear target is untapped and left uncountered.

### Slices 12–13: reusable artifact and landfall packages

* **Sol Ring, Mind Stone, Hedron Archive:** real noncreature artifact spells,
  immediate tap-mana abilities, and Mind Stone/Archive's printed mana + tap +
  sacrifice costs for stacked draw abilities. Artifact mana must be activated
  explicitly; the land-only automatic payment planner and product heuristic do
  not automatically use it.
* **Arcane Signet, Fellwar Stone, Commander's Sphere, Werebear (slice 14):**
  immediate dynamic mana with an explicit offered color. Signet and Sphere use
  the registered commander's color identity (none without a commander); Fellwar
  Stone uses the typed basic mana types of lands an opponent controls, tapped
  ones included, and excludes your own lands and eliminated players. Werebear's
  `{T}: Add {G}` obeys summoning sickness and its threshold is a real layer 7c
  static (+3/+3 at seven or more cards in *your* graveyard). Sphere's printed
  sacrifice-draw works while the artifact is tapped. Tapped or summoning-sick
  sources are never offered by `legal_actions`.
* **Sazh's Chocobo:** controller-only landfall adds a real typed +1/+1 counter
  to the same battlefield object, not an end-of-turn pump.
* **Rampaging Baloths:** printed trample and controller-only landfall creating
  a 4/4 green Beast Token.
* **Professor of Zoomancy:** enters creating a 1/1 black and green Pest Token.
  A Pest's death queues its controller's gain-1-life trigger before cessation;
  that ability resolves later from the stack. Beast/Pest tokens are engine-only
  objects, never importable cards.

Printing sources and retrieval dates are in
`fixtures/starter-corpus/slice12-sourced.json` and `slice13-sourced.json`.
Conformance is in `crates/core/tests/issue17_slice12.rs` and
`issue17_slice13.rs`. Both packages have actual four-seat 40-life terminal,
replay-verified journeys and fresh session-save roundtrips. The artifact
journey uses an explicitly labeled evidence-only masked demonstrator; it is
not a product-AI improvement. These 40-card packages do **not** certify another
100-card starter. The seven planned starters and arbitrary-deck compatibility
remain unsupported.

### Slice 15: costs paid with cards, and the first enchantments

* **Ayula's Influence** (`{G}{G}{G}` enchantment) — "Discard a land card: Create
  a 2/2 green Bear creature token." The activator names the land in the action;
  the engine refuses a non-land, a card that is not in hand, and a missing or
  duplicate cost card. This is the pool's first card paid from hand.
* **Bearscape** (`{1}{G}{G}` enchantment) — "{1}{G}, Exile two cards from your
  graveyard: …". Every legal pair is offered as its own action; the two named
  cards are exiled as the cost. Offered only when two cards are actually there.
* **Mother Bear** (`{1}{G}` 2/2) — "{3}{G}{G}, Exile this card from your
  graveyard: Create two 2/2 green Bear creature tokens. Activate only as a
  sorcery." This is the pool's first ability activated **from the graveyard**,
  and the first "activate only as a sorcery" timing restriction (CR 307.5): it
  is offered only in its owner's main phase with an empty stack, and the card
  exiles itself as the cost.
* New engine surface: `Kind::Enchantment` (noncreature permanents that cast and
  resolve like other permanent spells), `Action::ActivateCostedAbility` (named
  cost cards), `AbilityCost::{discard_land, exile_graveyard_cards,
  exile_source_from_graveyard}`, `ActivatedAbility::sorcery_timing`, a 2/2 green
  Bear token card, and `Effect::CreateBearTokens`. `Action::ActivateAbility`
  keeps its old shape, so every earlier replay and test literal still loads.
* The product heuristic still does not activate these abilities (no AI file was
  touched); the journey below uses an evidence-only masked demonstrator.

Printing sources and retrieval dates are in
`fixtures/starter-corpus/slice15-sourced.json`; conformance is in
`crates/core/tests/issue17_slice15.rs` (behaviour) and
`issue17_slice15_corpus.rs` (the manifest's `missing_cards` must equal pool
membership for all eight lists — a recomputation guard that did not exist
before). Four genuine REDs are recorded in `evidence/issue17/slice15/`.

Slices 8–9 currently support eight sourced cards: Soul's Attendant, Razorclaw
Bear, Volley Veteran, Dramatic Reversal, Goblin War Strike, Brightstone Ritual,
Mana Geyser and Rite of Flame. Friendly Teddy and Aetherize remain explicitly
excluded (serialized multi-player draw and attacker-list commander replacement,
respectively) and are manifest-missing. Goblin War Strike's printed planeswalker
target is not usable because no planeswalker permanents exist in the pool; the
player target is implemented. Slice 9 widened the stored mana pool to `u32` so
Brightstone can produce more than 255 red mana without truncation.

Staged interaction roadmap (derived from the manifest `missing_interactions`):

1. tokens and token permanents: **partially implemented** as typed 1/1 red
   Goblin tokens for the Krenko package (Krenko, Dragon Fodder, Krenko's
   Command, Hordeling Outburst, Goblin Instigator, Beetleback Chief) and a 2/2
   blue Drake with functional flying for Talrand, Sky Summoner. Ayula and Zada
   still need their own token types; the exact Krenko starter is still planned
   (48 missing) and Talrand's desired list has 51 missing cards;
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
   blockers first and tramples the surplus to the player); defender, token
   flying and printed islandwalk are also implemented in their named packages.
   Haste and wider keyword semantics remain unimplemented;
5. typed counters exist in the core (issue #16); Ayula's printed counter/tribal text is not implemented;
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
   so damage prevention cannot stop it. Rite of Flame now counts its namesake
   cards in all graveyards; tutors and reanimation remain unimplemented;
10. stack interaction (Talrand): **bounded implemented package** — "counter
    target noncreature spell" as a real target restriction (Negate may not name a
    creature spell on the stack, CR 205.2a). "Counter target spell" was already
    supported (Counterspell);
11. mana acceleration (Zada): **bounded implemented package** — adding red mana
    to the pool without tapping a permanent (Pyretic Ritual, Seething Song), and
    Preordain's scry-2-then-draw as an ordered two-effect resolution;

Elas still has 52 unsupported desired-list nonland names plus sacrifice outlets,
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

## Certification of jasmine-vanilla-stompy (slice 10)

`certified` is an executable claim, not a catalog count. The `certify` command
reads the list's own deck file through the manifest path, hashes those exact
bytes, refuses to substitute any bundled practice deck, verifies every card in
the list is supported, plays real four-seat Commander-combat games with that
snapshot in every seat, and requires every game to end with a terminal outcome
whose recorded replay reproduces the same state digest:

```
cargo run --release -p aperture-cli -- certify \
  --starter jasmine-vanilla-stompy --seats 4 --seeds 1,2,3 \
  --out evidence/issue17/slice10/certification.json
```

Recorded certificate (`evidence/issue17/slice10/certification.json`, schema
`aperture.corpus-certificate/1`):

* verdict `certified`; snapshot SHA-256 (over the deck file text with CRLF
  normalized to LF, so the digest is the same on an LF or CRLF checkout)
  `4312ab2a90a94a4f9c9578b330a093dcec7eb1aa253785c97667cb0fab967491` over the
  deck file bytes actually parsed; 99 library cards + 1 separately supplied
  commander = 100 cards; 58 unique nonland names + 41 basic lands.
* three four-seat 40-life games, all `terminal`, all replay-verified:
  seed 1 winner P1, 2364 steps/53 turns, digest `8e4feea3a9e1…`;
  seed 2 winner P2, 2802 steps/69 turns, digest `4677eb42988a…`;
  seed 3 winner P3, 1685 steps/40 turns, digest `520e8a0dd58d…`.
* coverage by execution: 55 of 61 unique names were actually cast or played as
  lands across those games. The 6 names those particular games never cast
  (Knight Errant, Alpha Tyrranax, Enormous Baloth, Fog, Swords to Plowshares,
  Mending Hands) are covered by engine conformance suites instead; the
  certificate lists them rather than implying in-play evidence for them.
* conformance: `crates/core/tests/starter_corpus.rs` proves all 47 sourced
  vanilla creatures match their printings and cast/resolve;
  `crates/core/tests/issue17_slice10_certification.rs` proves the exact library
  is 99 supported singleton cards and the whole 100-card snapshot holds exactly
  one legendary card (the separately supplied commander), casts all 11
  remaining non-vanilla cards from that exact set, and fails if the manifest
  certification record stops matching the real deck file;
  `crates/cli/tests/certify.rs` runs the shipped binary, including the
  no-substitution and edited-snapshot refusals.

Documented interaction exclusions for this list (also machine-readable in the
manifest `certification.interaction_exclusions`):

* **Legend rule (CR 704.5j)** is not modelled. It is unreachable in this list —
  each seat supplies exactly one copy of its commander and no copy effect exists
  — so no certified game depends on it. Any list that could put two same-named
  legends under one player stays uncertifiable until the SBA and its controller
  choice exist.
* Equipment, Auras, planeswalkers and nonbasic lands are absent from the list,
  so none of their rules are exercised or claimed.
* The native Commander table, trainer, measured playing strength and arbitrary
  Archidekt compatibility remain explicit non-goals.

Limits of the claim: the profile is bounded commander-combat practice (40 life,
one supported commander per seat, 21 commander damage), not the Commander
format, and the certified games use the built-in heuristic — the certificate
proves the list plays to a terminal, replay-verified game, not that any policy
plays it well.

## Phased gaps (why the eight-list gate is open)

1. **Card-support surface.** The implemented pool is 134 cards. Non-vanilla
   archetypes need token creation, activated abilities beyond land mana,
   triggered abilities, keyword semantics, counters as rules objects and
   static/layer effects that this slice does not implement. Those are the
   `missing_interactions` entries in the manifest.
2. **Commanders.** Three target commanders remain unsupported: Ayula, Elas and
   Zada. Isamaru, Jasmine, Krenko, Talrand and Tatyova have named bounded support,
   not complete desired starter lists.
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
4. **Certification.** `certified` now means: the exact declared snapshot parses
   as-is (no bundled replacement), every one of its 100 cards is supported,
   every non-vanilla card is proven castable by a conformance test, and the
   manifest's certificate record matches the real file bytes and the recorded
   four-seat games. Exactly one of the eight lists meets that bar
   (`jasmine-vanilla-stompy`); the seven planned lists do not, so the eight-list
   release gate stays open. Certification also says nothing about merged CI,
   release acceptance or playing strength.

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

## Slice 17: death-history package
Slice 17 implements three real sourced cards: Friendly Teddy, Ulvenwald Bear and Caller of the Claw. Death history is tracked only while a morbid/count card is present, so pre-slice-17 saves and replays keep their digest. Ayula remains planned with 31 of its 34 desired nonland cards still missing; the eight-list gate is open.
