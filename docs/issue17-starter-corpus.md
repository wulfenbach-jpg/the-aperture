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
139-card pool. Isamaru itself is supported, and the slice-4 package implemented
one of its cards (Soul Warden), so `isamaru-white-aggro` now lists 55 missing
cards; it stays `planned` because 55 of its chosen cards are still unsupported.
Slice 5 implemented one card in each of four other planned lists:
`Stitcher's Supplier` (elas), `Ruin Crab` (tatyova), `Negate` and `Preordain`
(talrand), `Pyretic Ritual` and `Seething Song` (zada). None of them is close to
complete, so all seven remain `planned`.

## Corpus union and staged gaps

Exact all-name totals over the eight desired lists, including commanders and
basic lands (coordinator evidence
`evidence/issue17/slice25/native-corpus-audit.py` + `native-corpus-audit.json`):

* union of all referenced names: **459 unique names**;
* **133** are in the supported 146-card catalog and **326** are missing. Slice
  25 adds `Exuberant Wolfbear` (129→130), slice 26 adds `Ursine Champion`
  (130→131), slice 28 adds `Circle of the Moon Druid` (131→132) and slice 29
  adds `Stuffed Bear` (132→133) to the previously verified split. The union is unchanged because every one of them
  was already a referenced name. The
  five basic land types are part of the union, which is why the union exceeds the
  sum of nonbasic names.
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
(slice 17). Slice 28 adds `Circle of the Moon Druid` and slice 29 adds
`Stuffed Bear`. Ayula currently has **19** missing names.

Slice 16 is the first time "each player who controls ..." creates the token
(the token's controller is the permanent's controller, not the spell's), and
the first simultaneous mass destruction (`Fade from History` destroys every
artifact and enchantment in one event with one dies-watcher batch). `Little
Bear` reads its counter condition off the *target's* effective subtype, so a
non-Bear target is untapped and left uncountered — and since slice 26 an
activated "becomes a Bear ..." type change counts as that subtype
(`Layer::Type`, CR 613.1d).

Slice 26 adds `Ursine Champion` (M19 304): `{5}{G}` for +3/+3 and "becomes a
Bear Berserker until end of turn". The subtype change is the pool's first
layer 4 (CR 613.1d) continuous effect: it replaces the printed subtype line for
every rules question until cleanup (CR 514.2), so the Bear target filters, the
Ayula fight revalidation and `Little Bear`'s counter branch all answer from the
effective line through `Game::has_subtype`. The printed once-each-turn limit
reuses the restriction `Rootwalla` already exercises. Not implemented: any other
type-changing effect and control-changing effects (so the
once-each-turn key is the object, not the player).

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
   blue Drake with functional flying for Talrand, Sky Summoner. Zada and the
   remaining Ayula-list Bear makers still need their own token types (Ayula
   herself needs no token); the exact Krenko starter is still planned
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
   Haste is implemented as a *granted* keyword (`Game::has_haste` /
   `ContinuousKind::GrantsHaste`, Surrak and Goreclaw); cards that print haste
   are not in the pool, so printed haste and wider keyword semantics remain
   unimplemented;
5. typed counters exist in the core (issue #16); slice 18 implements Ayula's
   printed counter/fight trigger as the bounded `TargetSpec::Bear` /
   `TargetSpec::BearYouControl` modes with a typed fight effect (CR 701.14a/b).
   Other counters-matter and tribal statics in these lists remain unimplemented;
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

1. **Card-support surface.** The implemented pool is 136 cards. Non-vanilla
   archetypes need token creation, activated abilities beyond land mana,
   triggered abilities, keyword semantics, counters as rules objects and
   static/layer effects that this slice does not implement. Those are the
   `missing_interactions` entries in the manifest.
2. **Commanders.** Two target commanders remain unsupported: Elas and Zada.
   Isamaru, Jasmine, Krenko, Talrand, Tatyova and (slice 18) Ayula have named
   bounded support, not complete desired starter lists.
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
Slice 17 implements three real sourced cards: Friendly Teddy, Ulvenwald Bear and Caller of the Claw. Death history is tracked only while a morbid/count card is present, so pre-slice-17 saves and replays keep their digest. Ayula remains planned with 29 of its 34 desired nonland cards still missing; the eight-list gate is open.

## Slice 18: Ayula, Queen Among Bears
Ayula is a supported commander (`{1}{G}`, 2/2 Legendary Bear) with sourced printing data in `fixtures/starter-corpus/slice18-sourced.json`. Her trigger fires only for another Bear its controller controls; token entries count and opposing Bear entries do not. The controller chooses counters or fight before targets. Counters may target any Bear (including an opponent's). Fight requires a Bear you control and a creature you don't control, revalidates both at resolution, and deals damage simultaneously through the existing prevention/replacement pipeline. Ayula has 29 missing desired-list cards after slice 19; the eight-list gate remains open.

## Slice 19: Spectral Bears and the first attack trigger
`Spectral Bears` (`{1}{G}` Creature — Bear Spirit 3/3, me1 131, sourced in `fixtures/starter-corpus/slice19-sourced.json`) is the pool's first attack trigger. Its printed intervening-if — "if defending player controls no black nontoken permanents" — is evaluated in the declare attackers step before the trigger is queued and again at resolution, so an attack into a black nontoken permanent never reaches the stack, while a black *token* does not satisfy the exclusion. The payoff marks the attacker so that its controller's next untap step leaves it tapped exactly once (CR 502.3); the marker is serde-skipped when empty, so every previously recorded digest is unchanged. Black is read from the printed mana cost (CR 202.2a) or the token's own colors. Attack triggers are now infrastructure rather than a card patch: `TriggerEvent::Attacks` plus `TriggerCondition`, the latter evaluated before queueing and re-checked at resolution. Ayula has 29 missing names; the eight-list gate remains open and no list is certified by this slice.

## Slice 20: Vivien's Grizzly
Vivien's Grizzly (`{2}{G}`, 2/3 Bear Spirit) is sourced in `fixtures/starter-corpus/slice20-sourced.json` (Scryfall WAR 182, retrieved 2026-10-27T03:25:27Z). Its `{3}{G}` ability looks at the anchored top card: a creature/planeswalker card gets a private reveal-and-take or bottom choice; a nonqualifying card goes to the bottom without being revealed; an empty library is not a draw. Old replay digests are unchanged because no state field was added.

## Slice 21: Dragon-Scarred Bear
Dragon-Scarred Bear (`{2}{G}`, 3/2 Bear) is sourced in `fixtures/starter-corpus/slice21-sourced.json` (Scryfall DTK 183, retrieved 2026-10-27T04:44:50Z). Formidable `{1}{G}` creates a repeatable, non-tapping regeneration shield only when current total creature power is at least 8. Shields are object-anchored, expire at cleanup, and replace only destruction (lethal-damage SBA or a destroy effect): damage is removed, the creature taps and leaves combat. Exile, bounce and zero-toughness moves do not regenerate.

## Slice 22: Dancing from Dark to Dawn
Dancing from Dark to Dawn (`{3}{G}{G}` Enchantment, Scryfall HOB 123, immutable
printing `550cd0b6-ca61-4db7-9d20-0b68c48066f9`, oracle
`5ba482e9-fbb0-4d9f-a3a9-414892bcdfed`, retrieved 2026-10-27T06:31:42Z, sourced
in `fixtures/starter-corpus/slice22-sourced.json`) is
the pool's first **"you cast a creature spell"** trigger and the first effect
sized by the triggering spell's mana value. `TriggerEvent::CreatureSpellCast`
is deliberately separate from the existing instant/sorcery `SpellCast`: a
noncreature permanent spell triggers neither, and a creature spell never
triggers the instant/sorcery wording. `CardDef::mana_value()` derives CR 202.3
mana value from the printed cost (`{X}` counts 0, a hybrid symbol counts its
numeric half, every other symbol 1). The value is captured when the spell is
cast and travels on the trigger (`TriggerRef::spell_mana_value`) and the stack
item, so resolution never re-derives it; both new fields are serde-skipped when
zero, so every previously recorded digest is unchanged. The landfall half reuses
`Effect::CreateBearTokens(1)`. Ayula has 26 missing names after this slice, the
eight-list gate remains open, and no list is certified by this slice.

**Defect found and repaired inside this slice:** the cast path granted priority
after casting unconditionally, so a cast trigger waiting for a target produced a
state with both a pending choice and a priority holder — the session validator
refused to load it (retained RED
`evidence/issue17/slice22/focused-2.log`). The caster now receives priority only
when no choice is pending (CR 603.3d).

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
pool has no second copy); at that slice the pool still had no layer 4/5/7b
(type, colour, base power/toughness) effects. (Slice 25 added the layer 7b base
power/toughness setter, slice 26 the layer 4 subtype line, slice 28 the first
*printed* layer 4 + 7b ability and slice 29 the layer 5 colour set plus the
additive layer 4 animation that turns a non-creature permanent into a creature,
so layers 4, 5, 6, 7b and 7c now exist. Still missing: layer 7a
characteristic-defining power/toughness, which is why `Beorn's Hospitality`
remains unsupported, and layer 7d, which belongs to the counters lane.) Ayula
had 22 missing names after slice 26, 21 after slice 28 and 20 after slice 29;
the eight-list gate remains open at 1 of 8 certified, and no new list is
certified by this slice.

Three mutation REDs are retained in `evidence/issue17/slice23/`: disabling the
reduction fails exactly the two cost tests, ignoring the printed power threshold
fails exactly the qualifying-creature test, and dropping the grant lookup from
`has_trample` fails exactly the trample combat test.

## Slice 24: Surrak and Goreclaw
Surrak and Goreclaw (`{4}{G}{G}`, 6/5 Legendary Human Bear) is sourced in `fixtures/starter-corpus/slice24-sourced.json` (Scryfall MOM 337, immutable 9c10934d, retrieved 2026-10-27T08:44:35Z). Printed trample; other creatures its controller controls have trample through a typed static scan. Its nontoken-own-creature entry trigger captures the entered object as a non-target identity, adds a +1/+1 counter, and grants haste until end of turn in the ability layer. Tokens, self-entry and opponents do not qualify. Losing the battlefield incarnation removes pending/stack affected references; haste expires at cleanup.

## Slice 25: Exuberant Wolfbear
Exuberant Wolfbear (`{3}{G}`, 4/4 Wolf Bear) is sourced in `fixtures/starter-corpus/slice25-sourced.json` (IKO 151, immutable 8c8ff0da, retrieved 2026-10-27T09:19:21Z). Its optional attack trigger targets a Human you control and sets that Human's base power/toughness to the Wolfbear's effective power/toughness captured at resolution. Base setting is layer 7b before counters and 7c modifiers; source LKI is captured on battlefield departure for source-left resolution. Target/source departure and cleanup follow normal continuous-effect rules.

## Slice 28: Circle of the Moon Druid
Circle of the Moon Druid (`{2}{G}`, 2/4 Creature — Human Elf Druid) is sourced in `fixtures/starter-corpus/slice28-sourced.json` (Scryfall AFR 177, immutable `700c9ed8-6bfe-4f1e-855a-5f31e0d2fc9b`, oracle `90569b99-8b63-4d2e-b7de-b578b8adbdcb`, retrieved 2026-10-27T12:07:11Z; both of its printings, AFR 177 and HBG 204, carry identical wording). Printed text: "Bear Form — During your turn, this creature is a Bear with base power and toughness 4/2. (It loses all other creature types.)"

* It is the pool's first **printed** continuous ability conditioned on whose turn it is (CR 611.3a). One printed ability spans two layers: the subtype line is layer 4 (CR 613.1d, `StaticAbility::SelfBecomesSubtypeDuringYourTurn` + `SubtypeLine::Bear`) and base power/toughness is layer 7b (CR 613.4b, `StaticAbility::SelfBasePowerToughnessDuringYourTurn`).
* Neither half creates a `ContinuousEffect`: the condition is answered when the characteristic is read, so nothing has to expire in cleanup and no effect object can be orphaned, bounced or re-used. `Game::has_subtype` consults the printed line before it applies any layer 4 effect, and `recompute_layers` applies the 7b base before the timestamp-ordered one-shot setters and the 7c modifiers, so the printed 4/2 is a *base* that pumps and counters sit on top of.
* Because the ability starts applying the moment its source arrives, `enter_battlefield` recomputes characteristics when the entering card prints a layer 4 or 7b static. The additive printed layer 7c statics (`Wight of Precinct Six`) keep their existing recompute points; that staleness window — a Wight that enters mid-turn reads its printed 1/1 until the next pump or untap — is pre-existing debt, disclosed here rather than silently changed.
* Known limit, disclosed: the printed setter is applied before every one-shot setter in layer 7b regardless of timestamp (CR 613.7). With one printed base-setting ability and no mass base-power/toughness setter in the pool this ordering cannot be observed, but a future card that sets another creature's base power/toughness before the Druid enters would be ordered wrongly, and that card will have to carry a timestamp rather than reusing this static path.
* Consequences the engine now answers with the effective subtype: Little Bear's "if that creature is a Bear" counter branch, Ayula's `TargetSpec::Bear` / `TargetSpec::BearYouControl` offer lists, and Exuberant Wolfbear's "target Human you control", which correctly *loses* the Druid during its controller's turn because the Bear line replaces the printed subtypes.

Verification retained in `evidence/issue17/slice28/`: three discriminating mutation REDs (turn condition ignored, layer 7b half ignored, layer 4 half ignored — each restored byte-for-byte, `run-mutations.py` proves it), a four-seat 40-life package journey from `evidence/issue17/slice28-journey-driver` (`decks/package-slice28.txt`, terminal `Win{P3}`, 2065 actions, 50 turns, digest `48bf3ae5…`, separate CLI replay matches) whose driver asserts the printed 4/2-or-2/4 invariant after *every* accepted action (636 on-turn and 1404 off-turn sightings), and the usual gates. This slice certifies no starter list: Ayula still has 21 missing names and the eight-list gate remains 1 of 8.

## Slice 29: Stuffed Bear
Stuffed Bear (`{2}` Artifact; Scryfall **MID 259**, immutable `4cd40276-016b-475f-906f-74d31dceaf70`, oracle `1402c116-194e-49ca-a8c1-09ca659c220c`, retrieved 2026-10-27T13:14:48Z; MID 259 and DBL 259 print identical wording) is sourced in `fixtures/starter-corpus/slice29-sourced.json`. Printed text: "{2}: This artifact becomes a 4/4 green Bear artifact creature until end of turn."

* It is the pool's first permanent that is **not a creature until an ability animates it**, and one printed ability spans three layers: **layer 4** (CR 613.1d) adds the creature card type and the Bear subtype *in addition to* the printed Artifact type (`ContinuousKind::AddCreatureSubtypes`), **layer 5** (CR 613.1e) sets the colour to green (`ContinuousKind::SetColor`, the pool's first layer 5 effect) and **layer 7b** (CR 613.4b) sets base power/toughness 4/4 (`SetBasePowerToughness`). All three are ordinary until-end-of-turn `ContinuousEffect`s, so cleanup ends them (CR 514.2) and departure prunes them (CR 400.7) through the existing pipeline; nothing bespoke expires.
* The rules layer now asks `Game::is_creature` instead of reading the printed card type. That single layered query is what makes the animation real: the animated artifact is offered as a creature target, can attack and block, deals and receives combat damage, is hit by creature-only removal, and its death is recorded as a *creature* death — `move_to_graveyard_inner` captures the creature answer at the moment the permanent leaves, so dies-watchers (Zulaport Cutthroat's drain is the test) fire for an animated artifact.
* `Game::has_color` is the layered colour query and `CardDef::is_color` its printed baseline; the Spectral Bears intervening-if now asks it rather than `is_black`. Known limit, disclosed: green is the only colour any supported effect sets, and no rule in the pool discriminates a *green* permanent from a colourless one, so layer 5's observable effect today is the query itself (and the animated permanent's colour identity in tests), not a rules outcome.
* CR 302.6: a permanent that prints an ability which can animate itself records summoning sickness when it *arrives* (`ability::can_become_creature`), even though it is not a creature yet, and the untap step clears it for every permanent. An artifact animated on the turn it entered therefore cannot attack; one animated on a later turn can, which is the only turn its until-end-of-turn animation could ever attack.
* Known limits, disclosed: the animation adds only the Bear subtype line the pool prints (a future "becomes a <type> in addition" card adds a `SubtypeLine` variant); the colour set is a single colour, not a set of colours; the engine has no way for a *second* permanent to animate another player's artifact, so only self-animation exists.

Verification retained in `evidence/issue17/slice29/`: four discriminating mutation REDs (layer 4 addition ignored, layer 5 ignored, layer 7b ignored, entry sickness dropped — each restored byte-for-byte, `run-mutations.py` proves it), a four-seat 40-life package journey from `evidence/issue17/slice29-journey-driver` (`decks/package-slice29.txt`, terminal `Win{P3}`, 1912 actions, 44 turns, digest `ac16e510…`, separate CLI replay matches) whose driver asserts the animated-or-plain invariant after *every* accepted action (1301 animated and 3741 plain sightings across 119 activations), and the usual gates. This slice certifies no starter list: Ayula still has 20 missing names and the eight-list gate remains 1 of 8.

## Slice 30: Drover Grizzly and reusable saddle
Drover Grizzly (`{2}{G}`, 4/2 Bear Mount) is sourced in `fixtures/starter-corpus/slice30-sourced.json` (OTJ 161, immutable 560062cd, retrieved 2026-10-27T14:40:04Z). Saddle 1 is a reusable typed cost/action: at sorcery timing, tap any bounded subset of other untapped creatures you control with total effective power at least 1; helpers need not be untapped since turn start. Saddle is a designation effect until cleanup and ends on battlefield departure. An attack while saddled grants trample to creatures controlled then, through the existing layer-6 grants; unsaddled attacks do not trigger.
