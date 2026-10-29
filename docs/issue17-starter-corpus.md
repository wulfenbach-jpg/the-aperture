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
164-card branch-candidate pool. Isamaru itself is supported, and the slice-4 package implemented
one of its cards (Soul Warden), so `isamaru-white-aggro` now lists 55 missing
cards; it stays `planned` because 55 of its chosen cards are still unsupported.
Slice 5 implemented one card in each of four other planned lists:
`Stitcher's Supplier` (elas), `Ruin Crab` (tatyova), `Negate` and `Preordain`
(talrand), `Pyretic Ritual` and `Seething Song` (zada). None of them is close to
complete, so all seven remain `planned`.

## Corpus union and staged gaps

Exact all-name totals over the eight desired lists, including commanders and
basic lands (coordinator evidence
`evidence/issue17/slice54/corpus-audit.py` + `corpus-audit.log`; earlier audits retained as historical):

* union of all referenced names: **459 unique names**;
* **153** are in the supported 166-card catalog and **306** are missing. Slice
  25 adds `Exuberant Wolfbear` (129→130), slice 26 adds `Ursine Champion`
  (130→131), slice 28 adds `Circle of the Moon Druid` (131→132), slice 29
  adds `Stuffed Bear` (132→133), slice 30 adds `Drover Grizzly` (133→134) and
  slice 31 adds `Beorn the Fierce` (134→135) to the previously verified split. Later
  slices bring the current implemented total to **164**; slice 53 adds Ruxa,
  Patient Professor (161→162, so supported 148→149 and missing 311→310, and
  Ruxa leaves the Ayula gap 5→4). Slice 54 adds Ursine Monstrosity (162→163, supported 149→150, missing 310→309, Ayula 4→3). Slice 57 adds Rampaging Yao Guai (163→164, supported 150→151, missing 309→308, Ayula 3→2). The intervening per-slice list is
  historical. The union is unchanged because its deck snapshots have not changed; Snow-Covered Forest is an additional supported card outside that union. The
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
`Stuffed Bear`. Ayula currently has **1** missing name.

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
   `ContinuousKind::GrantsHaste`, Surrak and Goreclaw); no supported card
   *prints* haste yet, so printed haste remains unimplemented even though the
   engine already answers the granted case;
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

1. **Card-support surface.** The implemented pool is 166 cards. Non-vanilla
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
so layers 4, 5, 6, 7b and 7c now exist. Slice 33 added layer 7a
characteristic-defining power/toughness for `Beorn's Hospitality`, so layers 4,
5, 6, 7a, 7b and 7c all exist; layer 7d is applied outside the layer pass by
`Object::power`/`toughness`, which add +1/+1 counters after every layer.) Ayula
had 22 missing names after slice 26, 21 after slice 28, 20 after slice 29,
19 after slice 30, 18 after slice 31, 17 after slice 32 and 16 after slice 33;
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

## Slice 31: Beorn the Fierce
Beorn the Fierce (`{3}{G}{G}`, 6/6 Legendary Creature — Bear Shapeshifter Warrior) is sourced in `fixtures/starter-corpus/slice31-sourced.json` (Scryfall **HOB 119**, immutable `367d5f8b-77ee-47f7-bc71-972d62c280a9`, oracle `30ca8e92-5955-46a1-86c1-094a873f518f`, retrieved 2026-10-27T15:33:31Z). Printed text: "Trample / Other Bears you control get +2/+2. / At the beginning of combat on your turn, put a trample counter on up to one target creature you control. It becomes a Bear in addition to its other types. Then if you control three or more Bears, draw two cards."

* **Printed trample** is carried by the catalog's `card_trample!` slot, so it is the same flag `Game::has_trample` already reads for any other trampling card.
* **"Other Bears you control get +2/+2"** is the pool's first *subtype* anthem: a layer 7c additive static (`StaticAbility::OtherBearsYouControlGet`) whose recipient set is computed from the **effective** subtype, so a creature a layer 4 effect just made a Bear receives it, and the source never pumps itself.
* **"At the beginning of combat on your turn"** is a new `TriggerEvent::BeginningOfCombat`, dispatched once when the beginning-of-combat step starts and only for the active player's permanents (CR 507.1, 603.6a). Nothing is stored, so it cannot double-fire.
* **"Up to one target creature you control"** is a new `TargetSpec::UpToOneCreatureYouControl` plus `Action::DeclineTriggerTarget` (CR 115.6). The engine now always asks when legal targets exist — even one — and offers "choose no target"; declining still resolves the ability, so the conditional draw is unaffected.
* **The trample counter** is the pool's first keyword counter: `CounterKind::Trample` (CR 122.1b). It is a counter, not a continuous effect — `Game::has_trample` reads it, cleanup never removes it, and the permanent keeps trample after the turn ends.
* **"It becomes a Bear in addition to its other types"** is a layer 4 *addition* on the chosen target (`Effect::AddTargetCreatureType` → `ContinuousKind::AddCreatureSubtypes`). The ability states no duration, so the effect lasts until end of turn (CR 611.2a) and cleanup removes it: the counter stays, the subtype does not.
* **"Then if you control three or more Bears"** is evaluated after the earlier instructions of the same resolution (CR 608.2c) against the effective subtype, so the creature this ability just turned into a Bear counts toward the three.

Known limits, disclosed: the anthem's recipient set is recomputed only at the engine's existing layer-recompute points, so a creature that somehow changed subtype without a recompute would be stale (no supported effect does that); the "beginning of combat" event has no other card in the pool, so a second printing would share this single dispatch; and `CounterKind` now has a keyword kind, which the counter display prints as "trample" but which contributes no power/toughness.

Verification retained in `evidence/issue17/slice31/`: four discriminating mutation REDs (trample-counter read dropped, Bear-count gate ignored, optional target made mandatory, subtype anthem disabled — engine restored byte-for-byte, proven by `run-mutations.py` and the pre/post sha256 files), a four-seat 40-life package journey from `evidence/issue17/slice31-journey-driver` (`decks/package-slice31.txt`, separate CLI replay matches) whose driver accepts the printed target on every offer and asserts that the trigger, the counter and the Bear addition really happened, and the usual gates. This slice certifies no starter list: Ayula still has 18 missing names and the eight-list gate remains 1 of 8.

Separately disclosed in slice 31, **repaired in slice 32**: `Surrak and Goreclaw` (slice 24) prints **Trample** on itself, but its catalog row was a `card!` (trample `false`) and its static grants trample only to *other* creatures, so Surrak itself did not trample. Slice 32 moves that row to `card_trample!` and records the printing's keyword list in `fixtures/starter-corpus/slice32-sourced.json`; `crates/core/tests/issue17_slice32.rs` proves both the flag and a real trample damage assignment, and `crates/core/tests/card_printings.rs` ties the row to the MOM 337 printing. The behaviour change moves exactly one previously recorded journey digest (slice 24: `a41bbfd2…`/4127 events → `697a846d…`/4130 events, same 1602 steps and same decisions); every other committed journey digest is unchanged, proven by `evidence/issue17/slice32/replay-drift.sh` and its log.

## Slice 32: Kamahl's Summons and the Surrak printed-trample repair
Kamahl's Summons (`{3}{G}` Sorcery) is sourced in `fixtures/starter-corpus/slice32-sourced.json` (Scryfall **ONS 269**, immutable `0edc37c6-b6a8-424f-95dd-928d03c28542`, oracle `8b6ef0a2-cd90-44c8-8f11-e1983f2b1dfd`, retrieved 2026-10-27). Printed text: "Each player may reveal any number of creature cards from their hand. Then each player creates a 2/2 green Bear creature token for each card they revealed this way."

* **"Each player may reveal any number of creature cards from their hand"** is the pool's first *per-seat arbitrary hand subset*. `Pending::ApnapReveal` holds the queue (`order` = every live seat in APNAP order from the active player, CR 101.4/608.2f) and the public answers; `Action::RevealCreatureCards` names any subset of that seat's own creature cards. An empty list is a real answer because the instruction is "may reveal any number". Revealing never moves a card: the cards stay in hand (CR 701.9a), so the only thing a reveal buys is tokens.
* **Validation is typed, not trusted**: a card the seat does not hold, a non-creature card, or the same card twice is refused with `EngineError::Illegal` and leaves the pending choice untouched; the action is also refused when no reveal is pending.
* **Nothing is applied per answer.** Every seat's count is fixed by its own reveal and the Bears are created together, in APNAP order, only after the last seat answers (`apply_apnap_reveal_batch`), so no seat's result depends on the order the table answered in. The offering surface enumerates two deterministic extremes ("reveal no creature cards" and "reveal every creature card in hand"); the action itself accepts any subset, which is a bounded-offer limitation of the CLI/UI, not of the rules.
* **Privacy**: the pending reveal is projected only to the seat that owes it (like the APNAP sacrifice queue), because the offer text names that seat's hand; the answers themselves are public and logged as `Event::CardsRevealed`.
* **Surrak and Goreclaw's own printed Trample** is now modelled (see the paragraph above): the same `card_trample!` slot every other trampling card uses, so combat assignment needs no special case.


Verification retained in `evidence/issue17/slice32/`: four discriminating mutation REDs (revealed count ignored, tokens created before the last answer, Surrak's row losing its trample slot, and a mandatory target trigger made declinable — every touched file restored byte-for-byte, proven by `run-mutations.py` and the pre/post sha256 files), the four-seat 40-life package journey `decks/package-slice32.txt` (seed 3232, `evidence/issue17/slice32-journey-driver`) whose report records the reveal answers in APNAP order per resolution, the Bear tokens they created and Surrak's combat damage, plus the refreshed slice-24 journey and the drift log above. Also committed in this slice: the previously untracked `evidence/issue17/slice31/exact-head-ci-e56b138.json` receipt the slice-31 review asked for, and the mandatory-target-decline regression test.

Known limits, disclosed: the reveal offers are the two extremes described above; the tokens are the existing 2/2 green `BearToken`; `crates/ai` and `crates/ui` received only the mechanical plumbing the new `PendingKind` requires (one heuristic arm that takes the dominant "reveal everything" offer, one label arm, two test-helper arms, one `crates/core/src/coach.rs` privacy/rules wording change) — no search, model or import behaviour changed. The `DeclineTriggerTarget` guard predates this slice; correction (3) is coverage, and the mutation set discriminates the refusal half only. This slice certifies no starter list: Ayula has 17 missing names and the eight-list gate remains 1 of 8.

## Slice 33: Beorn's Hospitality — the first layer 7a characteristic-defining ability
Beorn's Hospitality (`{1}{G}` Enchantment) is sourced in `fixtures/starter-corpus/slice33-sourced.json` (Scryfall **HOB 120**, immutable `153ca57e-30f0-4ad7-ae9d-c55cbf0fd4c9`, oracle `d9ed7252-11d1-432b-9101-ac08cd28826d`, retrieved **2026-10-27T18:21:29Z**; every printing was checked and carries one identical text; the raw provider response and the printings list are retained in `evidence/issue17/slice33/sources/`). Printed text: "Landfall — Whenever a land you control enters, put a +1/+1 counter on target creature you control. / {5}{G}{G}: This enchantment becomes a Bear creature in addition to its other types and gains \"This creature's power and toughness are each equal to the number of lands you control.\" (This effect doesn't end.)"

* **Layer 7a now exists.** `Layer::CharacteristicDefining` (CR 613.4a) and `ContinuousKind::PowerToughnessEqualToLands` are the pool's first characteristic-defining power/toughness. The value is **recomputed from the board on every layer pass**, never captured when the ability resolved, so a land that enters afterwards raises the power and toughness with no second activation (CR 604.3). It is applied *before* layer 7b setters and 7c modifiers, and counters are still added afterwards by `Object::power`/`toughness`, which is where 7d lives.
* **A duration that does not end.** `EffectDuration::WhileSourceOnBattlefield` was previously only used by static abilities; the animation uses it for an activated ability because the printing says "(This effect doesn't end.)" (CR 611.2b). Cleanup therefore leaves it alone — the journey observes the animated permanent on *both* the controller's and the opponents' turns — and it ends with the object it applies to (CR 400.7), through the existing departure pruning.
* **Additive, not replacing**: the layer 4 half reuses `ContinuousKind::AddCreatureSubtypes`, so the permanent is an Enchantment *and* a Bear creature; `Game::is_creature` and the Bear target spec both answer for it.
* **The first targeted landfall watcher.** `TargetSpec::CreatureYouControl` + the existing `Effect::PutPlusOneCountersOnTarget(1)`: the target is the printed "target creature you control", which includes the watcher itself once the animation has made it a creature, and the trigger is removed for want of a legal target when that seat controls no creature at all (CR 603.3d). The printed word "another" — Little Bear's wording — is a separate spec, `TargetSpec::AnotherCreatureYouControl`, which is the only one that removes the ability's own source.
* **Printed colour recorded.** `card_green!` is a new catalog macro: the pool records `colors` only where a supported rule can observe colour, and an animated creature's colour is observable.

Verification retained in `evidence/issue17/slice33/`: four-seat / 40-life package journey from `evidence/issue17/slice33-journey-driver` (`decks/package-slice33.txt`, seed 3333, terminal `Win{P3}`) whose driver re-asserts the characteristic-defining invariant after **every** accepted action (the 7a delta equals the current land count and effective power equals land count plus counters), with on-turn and off-turn sightings both non-zero, a separate CLI replay of the recorded replay, the mutation REDs in `run-mutations.py`, and the usual gates.

Known limits, disclosed: the animation's characteristic-defining "you" is read from the permanent's **current controller**; the pool has no control-changing effect, so this cannot be distinguished from the resolving controller. `card_green!` records a colour for this one card only — the other printable cards still carry `colors: &[]`, which is a pre-existing modelling choice, not a claim that they are colourless. The landfall trigger's counter is the ordinary display-and-P/T +1/+1 counter the rest of the pool uses. This slice certifies no starter list: Ayula had 16 missing names when this slice landed (each slice section states its own-time count) and the eight-list gate remains 1 of 8.

### Slice 33 review repair: "target creature you control" vs "another"

Review 290 found that the landfall trigger reused `TargetSpec::CreatureYouControl`, which at that head removed the ability's own source. That exclusion exists only because Little Bear prints "untap **another** target creature you control"; Beorn's Hospitality prints no "another", and once animated it is a creature, so it is a legal target of its own trigger — and, as the card's only creature, the one the engine must choose. The spec was therefore split: `TargetSpec::CreatureYouControl` is the printed "target creature you control" and **includes the source**, and the new `TargetSpec::AnotherCreatureYouControl` is the only spec that removes it. Little Bear keeps the exclusion, Dancing from Dark to Dawn's cast trigger (also printed without "another") now gets the correct set, and the cardinality behaviours are covered by five regression tests: the animated source auto-chosen as its own target and countering itself (5/5 with four lands), the source offered beside another creature with the counter landing on it, the unanimated enchantment not legal (the counter still goes to the only creature), a lone unanimated watcher losing the trigger to CR 603.3d with nothing on the stack, and Little Bear refusing to target itself. Two mutations — reinstating the exclusion in `CreatureYouControl`, and pointing Little Bear back at the plain spec — each fail exactly their own tests, with both touched files restored byte-for-byte.

### Slice 35 — Words of Wilding (ONS 305): the pool's first draw replacement

`{2}{G}` Enchantment, printed verbatim in `fixtures/starter-corpus/slice35-sourced.json`
(retrieved 2026-10-27T21:06:29.666092Z): **"{1}: The next time you would draw a card this turn,
create a 2/2 green Bear creature token instead."** The generic surface is an armed, one-shot,
per-seat draw replacement (CR 614.1a) held in `GameState::draw_replacements` — `Game::draw_with_resume`
consumes it before the top card is looked at, so no card moves, no `Event::Draw` is emitted, and an
empty library is not a loss (nothing was drawn). Cleanup clears an unspent replacement (CR 514.2),
the ability prints no once-per-turn limit (several may be armed, each consuming exactly one draw),
and the replacement is not tied to the source's existence. Six focused tests cover the draw step,
one-of-two draws, two-of-two draws with an empty library, cleanup expiry, controller-only scope and
the missing-cost refusal.

Verification retained in `evidence/issue17/slice35/`: the focused suite, the mutation REDs, the
heuristic selfplay journey (`decks/package-slice35.txt`, seed 3535, `Win{P2}`, 1360 steps, digest
`59f8bf2017…`, separately replayed), **and** the review-297 evidence repair —
`evidence/issue17/slice35-journey-driver`, a four-seat / 40-life game from the same exact deck
snapshot (seed 3535, terminal `Win{P3}`, 2048 steps, 41 turns, 5347 replay events, digest
`1c13d8fdcdce…`, byte-identical on re-run, separately replayed by the release CLI) in which the card
is **cast four times and its `{1}` ability activated 124 times**. 25 draw steps consume an armed
replacement, and each one is asserted from the engine's own events: no `Event::Draw` for that seat,
no `DrewFromEmpty`, library and hand sizes unchanged across the action, exactly one
`Event::PermanentEntered` for a 2/2 green Bear creature token under that seat's control that really
answers `Game::is_creature`/`has_color(Green)`/2-2, the armed count down by exactly one, and no loss.
The driver also exercises the *unspent* half in a real game (94 spare arms end at cleanup, 5 remain
when the game ends, so `124 = 25 + 94 + 5` — every arm is either consumed by a draw or ends with the
turn, CR 514.2). Every substituted action comes from the masked view of the seat that has priority;
nothing is injected, and no deck, seat or life total is changed.

Known limits, disclosed: the **heuristic** selfplay journey (`59f8bf2017…`) drew the card three times
and never cast it, so that particular game does not exercise the replacement — the focused tests and
the dedicated driver above do, and that is why the driver exists. The driver is an evidence-only
masked demonstrator, not a product decider or a strength claim. Commander-draw ordering: the armed
replacement is consumed before the CR 903.9b commander-draw suspension, so it pre-empts the reveal
with no CR 616.1 choice surface; both are replacement effects on the same draw, the engine has no
such choice surface, and it is unreachable with the current pool (nothing puts a commander into a
library), so this is a disclosed simplification rather than modelled behaviour. Adding the field to
the serialized `GameState` required two forced initializer-only edits outside `crates/core`
(`crates/ai/src/search.rs` copies the value, `crates/ai/tests/common.rs` starts empty); no AI-search
behaviour, UI, import or model code changed. This slice certifies no starter list: Ayula has 15
missing names and the eight-list gate remains 1 of 8.

## Slice 37: Druid's Familiar — the first soulbond pair

`Druid's Familiar` (`{3}{G}`, 2/2 Creature — Bear) is sourced from Scryfall **AVR 175**
(immutable printing id `1d8c794a-7964-41f0-bc9c-27af7cf87aaa`, oracle id
`6a5b1cd4-e597-4122-8fc7-4dce241d7c01`, retrieved 2026-10-27T23:16:27Z; printings `avr 175` and
`plst AVR-175` carry one identical text). Printed text:

> Soulbond (You may pair this creature with another unpaired creature when either enters. They
> remain paired for as long as you control both of them.)
> As long as this creature is paired with another creature, each of those creatures gets +2/+2.

**What is modelled.** Pairing is *state of two permanents*, not a continuous effect: each permanent
stores the creature it is paired with (`Object::paired_with`, `#[serde(default,
skip_serializing_if)]` so every earlier save, replay and digest is unchanged), and every reader goes
through `Game::paired_with`, which re-validates that both objects are still on the battlefield,
still controlled by the same player, and still point at each other. The printed "when either
enters" is two triggered abilities — this creature's own entry (CR 603.6a) and another creature
entering — and both offer the same choice: `TargetSpec::UnpairedAnotherCreatureYouControl`, the
printed set of *another* unpaired creature the controller controls with the source excluded.
Declining that choice is the printed "you may" (CR 702.94a), and the choice surface always offers
it. Resolving it pairs both permanents and reports a rules event; the +2/+2 is a layer 7c additive
static that applies to **both** members of the pair, re-read on every layer pass, so two paired
copies of the card each grant their own bonus. The pair ends at the single battlefield-exit point
(`Game::detach_object`), which clears the partner's back-reference: "for as long as you control both
of them".

**Verification.** `crates/core/tests/issue17_slice37.rs` (6 tests): the sourced text and both
triggers; entering alone pairs with nothing and stays 2/2; entering beside one unpaired creature
offers the pairing and both become 4/4; declining leaves both unpaired; a later creature entering
pairs with the already-present familiar; and killing the partner ends the pair, returning the
familiar to 2/2. `card_printings` also asserts the catalog row against the recorded printing.

Known limits, disclosed: soulbond's pairing is a choice made as the trigger resolves, not a target,
so pairing is offered through a dedicated choice spec rather than the targeting system; nothing in
the pool has shroud or hexproof, so no interaction between the two is observable today. No
control-changing effect exists in the pool, so the "as long as you control both" half is enforced at
the exit point and by the validating query rather than by a control-change test. This slice
certifies no starter list: Ayula has 14 missing names and the eight-list gate remains 1 of 8.

## Slice 38: Spider-Ham, Peter Porker — the first artifact (Food) token

`Spider-Ham, Peter Porker` (`{1}{G}`, 2/2 Legendary Creature — Spider Boar Hero) is sourced from
Scryfall **SPM 114** (immutable printing id `41f18f42-b86b-4a12-9f0d-76b761571195`, oracle id
`00b50215-e832-417f-9c80-2bc3050b6ebb`, retrieved 2026-10-28T00:54:14Z; all five printings —
`spm 114`, `spm 201`, `pspm 114s`, `pw25 10`, `om1 118` — carry one identical text, type line, cost
and P/T). Printed text:

> When Spider-Ham enters, create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this
> token: You gain 3 life.")
> Animal May-Ham — Other Spiders, Boars, Bats, Bears, Birds, Cats, Dogs, Frogs, Jackals, Lizards,
> Mice, Otters, Rabbits, Raccoons, Rats, Squirrels, Turtles, and Wolves you control get +1/+1.

**What is modelled.** Two new reusable surfaces rather than a one-card patch.

*The Food token* is the pool's first **artifact token** (CR 111.10, 205.3): a `CardId::FoodToken`
card definition in the catalog's token table (`Token Artifact — Food`, no power or toughness,
never importable, never part of `catalog::all()`), created by `Effect::CreateFoodToken`. It carries
a real activated ability whose cost is the first in the pool printed on a **token** to combine a
mana cost, `{T}` and "Sacrifice this token" (the cost *shape* is not new — Mind Stone's
"{1}, {T}, Sacrifice this artifact: Draw a card." and Hedron Archive's "{2}, {T}, Sacrifice this
artifact: Draw two cards." already pay mana, `{T}` and the source; what is new is that the source
is a token): the engine pays `{2}` from the pool, taps the token, moves it to its owner's graveyard
as part of the cost (CR 602.2b) and only then puts the ability on the stack, so the token has
already left the battlefield when "You gain 3 life" resolves. Because the sacrificed object is a
token, CR 111.7 then makes it cease to exist — the object is removed, it appears in no zone and in
no public graveyard view, and it cannot inflate a graveyard-count condition such as Werebear's
threshold. `CardId::is_token()` is derived from the `TOKEN_CARDS` registry, so registering a new
token variant there is what makes CR 111.7 apply; the pool-wide token registration test keeps the
registry and the two characteristics tables in step, and the four-seat journey below counts the
ceasing events.

*The anthem* is the first layer 7c static filtered by a printed **list** of subtypes instead of
one (`StaticAbility::OtherControlledCreaturesWithAnySubtypeGet`). It asks `Game::has_subtype`, so a
creature made a Bear by a layer 4 effect counts and a creature with none of the listed subtypes
never does; the printed "Other" keeps the source out of its own set, while a *second* copy of
Spider-Ham is a Spider and is pumped by the first copy's anthem, exactly as printed.

**Verification.** `crates/core/tests/issue17_slice38.rs` (8 tests): the sourced text and both
printed abilities; entering creates exactly one Food token that is an artifact, not a creature, and
whose ability is offered once `{2}` is available; activating it gains exactly 3 life, spends the
mana, sacrifices the token as a cost and the token then ceases to exist (object gone, absent from
the graveyard list and the public graveyard view, with the CR 111.7 records); the activation is
neither offered nor legal without its cost (no mana, or a tapped token); three real cards plus four
sacrificed Foods leave three graveyard objects and a 1/1 Werebear; every token variant in either
characteristics table is registered as a token and is never importable; the anthem pumps a Bear and
leaves an Elf alone while never pumping the source; and the anthem reads the *effective* subtype, so
Circle of the Moon Druid is 5/3 on its controller's turn (4/2 Bear Form + 1/+1) and 8/6 after Giant
Growth. `card_printings` asserts the catalog row against the recorded printing, and the four-seat
journey below exercises the whole card in a real game.

**Four-seat journey.** `evidence/issue17/slice38-journey-driver` plays `decks/package-slice38.txt`
(4 seats, 40 life, seed 3838, exact snapshot) and casts Spider-Ham 8 times, creating 8 Food tokens
and activating all 8 for 24 life, with every sacrificed token reported and counted as ceasing to
exist (0 Food objects left in any live zone), while asserting the anthem 7 110 times (once per
listed-subtype creature per action boundary) over the game; the game is terminal (`Win{P2}`, 1450
steps, 34 turns, 3861 events, digest `726fe20e…`) and a separate release-CLI replay reproduces it.

Known limits, disclosed: the Food token's ability is modelled with the printed cost and effect
only (it is not a "sacrifice an artifact" outlet for other cards); the anthem's recipient list is
the printed list in singular form, and no pool card can change a creature's subtypes into a listed
one except the layer 4 forms already supported; hexproof and shroud entered the pool with slice 42,
where Lightning Greaves and Swiftfoot Boots grant them to the equipped creature, so shroud (every
player) and hexproof (opponents only) are now enforced against spells, activated abilities and the
equip ability itself — the token/Equipment interaction the Ayula list needs. Protection is still
unsupported and no pool card grants or removes it. This slice certifies no starter
list: Ayula has 13 missing names and the eight-list gate remains 1 of 8.

## Slice 41: The Earth King
The Earth King (`{3}{G}`, 2/2 Legendary Human Noble Ally) is sourced in `fixtures/starter-corpus/slice41-sourced.json` (TLA 172, immutable a8d5dca6, retrieved 2026-10-28T07:02:29Z). Its ETB creates a distinct 4/4 green Bear token (CR 111.4 name, nonimportable). Its attack trigger fires once when one or more of its controller's current 4+ power creatures attack and opens an incremental private basic-land search with a captured count; any number up to that count may be chosen, chosen lands enter tapped through the canonical land-entry path, and the library shuffles even for zero choices. Search choices are owner-only in projections. A land put onto the battlefield by any effect now fires existing landfall watchers once per land (CR 603.6a) without spending the land play.

**Exclusion, not a convention.** The legend rule (CR 704.5j) is not implemented. Until this slice every legendary card in the pool was gated as commander-only by `DeckList::parse_practice`, so no legendary permanent could reach a library and the state-based action had no reachable input. The Earth King is not in `SUPPORTED_COMMANDERS`, so it *is* admitted into practice libraries, and its 40-card evidence package carries four copies: two copies of the same legendary permanent can legally coexist on this engine's battlefield. A real Commander list is singleton, so the Ayula certification path cannot reach that state, but the gap is real for ordinary practice decks and is recorded here rather than papered over as an existing convention. Fixing it means implementing the CR 704.5j controller choice, which is its own slice.

## Slice 42: shared Equipment — Lightning Greaves and Swiftfoot Boots
Lightning Greaves and Swiftfoot Boots are ordinary artifact Equipment with typed attachment effects. Equip is a targeted activated ability with its printed mana cost, sorcery timing, source-controller restriction and target creature you control; attachment changes only when the equip ability resolves, and the previous attachment persists while the new equip is on the stack. Greaves grants haste/shroud; Boots grants haste/hexproof. Shroud blocks all targeting, hexproof blocks only opponents; player targets and nontarget effects are unaffected. Attachments end when either object leaves the battlefield, when the creature ceases to be a creature, or on a later incantation of the same ObjectId. No broader Equipment/Aura/protection support is claimed.

## Slice 43: Lumra, Bellow of the Woods
Lumra (`{4}{G}{G}` Legendary Creature — Elemental Bear) is sourced in `fixtures/starter-corpus/slice43-sourced.json` (BLB 183, immutable `ae4f3aaf-3960-48cd-b34b-32e4ae5ae088`, oracle `97a84e9d-bfc4-4ca2-b1e8-908dba56ccdb`, retrieved 2026-10-28T10:55:27Z), and the catalog string equals that printing line for line. It prints `*`/`*`, which is represented in the catalog as *no printed numbers* plus a characteristic-defining static: `CardId::LumraBellowOfTheWoods` carries `StaticAbility::PowerToughnessPerCount { count: LandsYouControl }`, read from the board on every characteristic pass, so the value follows lands entering or leaving and counts only its controller's lands. This is the second land-count value in the pool (the first is Beorn's Hospitality's animation) and the first printed `*`/`*` card.

Two rule surfaces arrive with it. **Reach** is a blocking permission (`tokens::keywords`), so Lumra can block a flyer a ground creature cannot. **Vigilance** (CR 702.20a) is read at the single place an attacker is tapped (CR 508.1f): attacking leaves Lumra untapped and nothing else about the attack changes. The enters trigger mills four cards (`Effect::MillSelf`, already supported) and then returns **every** land card from its controller's graveyard to the battlefield tapped (`Effect::ReturnLandCardsFromGraveyardTapped`): the set is taken at resolution (CR 608.2c) and each land enters through the canonical entry path, so landfall watchers fire once per returning land although no land play is spent.

Exclusions, unchanged from slice 41 and still true here: the legend rule (CR 704.5j) is not implemented, so two copies of the same legendary permanent can coexist on this engine's battlefield; a singleton Commander list cannot reach that state, and Lumra is the pool's second legendary card admitted into a practice library. Nothing about lands' own abilities is inferred from text: a returned land is a permanent with its printed mana ability, nothing more.


## Slice 44: Spirit of the Aldergard
Spirit of the Aldergard (`{3}{G}`, Snow Creature — Bear Spirit, 0/4) is sourced in `fixtures/starter-corpus/slice44-sourced.json` (Scryfall 220df551…, oracle ff3bc67b…, retrieved 2026-10-28T12:05:47Z). Its ETB search uses the private incremental search machinery with a snow-land filter and reveal-to-hand destination, shuffling even on failure to find. The snow supertype is parsed from the printed type line. Its static is a printed layer-7c additive +1/+0 for each other snow permanent its controller controls; self and opponents are excluded. The narrowly required Snow-Covered Forest basic is sourced and supported to make the snow search/count real; no broad snow set is claimed.

## Slice 45: Titania's Command
Titania's Command (`{4}{G}{G}` Sorcery) is sourced in `fixtures/starter-corpus/slice45-sourced.json` (BRO 194, immutable 753567c7…, oracle 7aae0a3d…, retrieved 2026-10-28T13:02:38Z). Casting uses a typed two-distinct-mode action; resolution follows printed mode order. Exile graveyard, up-to-two land search to tapped battlefield with shuffle, two canonical 2/2 Bears and two +1/+1 counters on each own creature are all implemented. Only the exile mode requires a player target; a modal action with duplicate/insufficient modes or unselected targets is refused atomically.

## Slice 46: Thijarian Witness

**Rules-reading correction (slice 50):** Witness's past-state predicate uses
live combat membership immediately **before the death event**, not the attack
or block declaration. Pinned CR 506.5 distinguishes those predicates; CR 603.10
looks back before the departure. The earlier declaration-only requirement was
wrong and is superseded, not defended by its historical review approval.
Source re-fetch and the two genuine pre-repair assertion failures are retained
in `evidence/issue17/slice50/`.
Thijarian Witness (`{1}{G}`, 0/4 Creature — Alien Cleric, printed Flash) is sourced in `fixtures/starter-corpus/slice46-sourced.json` (WHO 111, immutable `da05e23b-419f-4023-9ad4-1227bb1b53be`, oracle `d273033b-3b8b-4ce9-afe8-208d82dd20f1`, retrieved 2026-10-28T13:49:35Z; both printings of the card carry identical oracle text). It is the pool's first `TriggerEvent::AnotherCreatureDies` watcher and its first `investigate`.

**The printed intervening-if** ("if it was attacking or blocking alone") is a past-state question: immediately before death, was this the only creature attacking or the only creature blocking? The engine reads `turn.attackers` and distinct `turn.blockers`, not `declared_*`. **Simultaneous deaths** still use one pre-departure `CombatAloneSnapshot` carried into every `DeathRecord` in the batch (CR 704.3), so the final death cannot falsely look alone merely because earlier moves in the same batch pruned combat membership. **Sequential departures differ:** if two creatures attack together, the first dies in one event, and the survivor dies later in that combat, the second death qualifies and the first does not. The blocking mirror also qualifies only the second death. A creature put onto the battlefield attacking counts in live membership despite never having been declared: Cub is no longer attacking alone while its put companion remains. These cases are executed in `issue17_slice46`; the two original declaration-only expectations are explicitly corrected, not silently deleted.

Resolution exiles *that* card from its owner's graveyard where the trigger found it and then creates one Clue token (CR 701.16a) for the trigger's controller; a card that already left the graveyard is simply not found and the investigate still happens, because the printed text has no "if you do" between the two instructions. The Clue token is a real artifact token with the printed `{2}, Sacrifice this token: Draw a card` ability and no `{T}` in its cost, so a Clue created this turn can be cracked at once and its sacrifice goes through the ordinary token-cessation path (CR 111.7). Flash is the catalog's keyword flag; nothing here grants or removes it.

Correction to inherited slice-47 guidance: under pinned CR 903.9a the graveyard/exile return is a state-based action **after** the commander enters that zone. A commander put into the graveyard therefore does die. The former claim that it needs a replacement effect to avoid dying was wrong; that claim is withdrawn, not implemented. The hand/library replacement is the separate CR 903.9b case. If the commander has returned to the command zone when Witness resolves, its graveyard object is no longer present, and investigate is not contingent on a successful exile. This correction changes no commander engine behavior. At slice 46's own checkpoint Ayula had 6 missing names; the current corpus counts are above.

## Slices 47 and 50: declaration bookkeeping versus live combat

Slice 47 added public `Turn::declared_attackers` / `declared_blockers` and
incorrectly made Witness depend on them. Slice 50 corrects the predicate to
live pre-departure membership. The fields remain written at declaration,
unpruned by departures and cleared at end of combat/turn start; they describe
who **attacked**, not who **is attacking**. They are no longer read by Witness.
A put-attacking creature stays outside the declaration record but counts in
the live snapshot. No existing replay expectations or historical evidence are
regenerated to conceal the correction.

The public record stays serialized, so canonical states carrying it retain the
slice-47 digest shape. `SAVE_VERSION` stays 10 and `REPLAY_VERSION` stays 6:
a bump would reject every older artifact, including the pinned pre-#16 fixture.
A pre-record mid-combat artifact can still re-execute to a different digest
because the new execution writes declaration fields. **The former Witness
under-reporting from an absent declaration is fixed**: live `attackers` and
`blockers` survive in those saves and are sufficient. A regression actually
deserializes the absent-field shape and drives the alone death through combat;
this is a state-continuation test, not a claim that every legacy SessionSave
now verifies its stored digest. See README "Versioned persistence".

Known weakness retained for a later hardening pass: `GameState::validate` does
not constrain the declaration lists. Witness no longer consumes them, so a
forged declaration cannot alter this predicate, but structural hardening is
still deferred.

**Unpaired blockers (slice 50, CR 509.1g):** a blocker remains a blocking creature
when its attacker leaves combat; removing the pair must not erase that fact.
A new omitted-when-empty `Turn::unpaired_blockers` retains only these live
unpaired creatures. It joins paired blockers in the pre-death snapshot, is
removed on that creature's own departure or regeneration, and clears at end of
combat/turn start. Pruning a noncreature attacker also retains its surviving
blockers. A real Unsummon then Lightning Strike regression failed before this
repair; actual saved-state load now preserves the orphan and identical
continuation. Additional controls cover paired-plus-unpaired counts and the
same blocker bounced and legally Flash-recast with the same ObjectId: it no
longer blocks. Eight malformed field shapes are refused by validation and the
real save loader. The fixture saves use dummy replay decks and establish load
and continuation, not replay provenance of a constructed initial board. Slice 51
projects the orphan list to both seats (`PlayerView::unpaired_blockers`), but no
UI surface renders combat lists yet — `ui/app.rs` still reads only
`legal_blockers`, so today the projection is a data-layer fact that a renderer
consumer does not yet display (review 335 non-blocking note, recorded here rather
than left in a review thread).

The new field is empty/omitted in unaffected states, but states actually holding
unpaired blockers gain a new digest basis. Older saves written **after** losing
all blocking pairs contain no recoverable membership for those blockers; this
repair cannot reconstruct that already-discarded information. Save/replay
versions remain 10/6 under the explicit best-effort compatibility policy above.
The AI determinization gains only the required public field copy and its test
constructor initializer; no AI policy, budget, sampling or hidden-data behavior
is changed. This narrow shared-state initializer exception is disclosed for
review, not hidden as a core-only diff.

**Projection of the orphan blocker (slice 51, review 333 N-1):** slice 50 kept
the orphan in the engine but `PlayerView` still projected only paired attackers
and blockers, so a seat's public combat state showed no blocking creature while
the engine held one. `PlayerView::unpaired_blockers` now carries the same public
list, separately from `blockers` rather than inventing a pair with a departed
attacker (CR 509.1g: the creature is still blocking; it simply blocks nothing
that is still attacking). Every seat sees it, because combat declarations and
their consequences are public. A temporary removal of the projection made the
committed combat regression fail (`left: [] / right: [ObjectId(8)]`, exit 101);
the committed test asserts it for both seats and that the pair list stays empty.

## Slices 48–49: Owlbear Cub and review329 repairs

Owlbear Cub (CLB 246, `{2}{G}`, 3/3 Bird Bear) is recorded verbatim with
printing/oracle IDs and retrieval stamp in `fixtures/starter-corpus/slice48-sourced.json`
and `evidence/issue17/slice48/sources/`. It attacks a player controlling at least
eight lands **at the attack event**. That restriction is not an intervening-if;
falling to seven lands after the trigger is on the stack does not suppress its
look (CR 603.4). Genuine intervening-if consumers are unchanged.

All top min(8, library size) cards are privately projected to its controller,
including noncreatures. Even when none is eligible, an acknowledgement keeps
the looked-at cards visible before placing them on the bottom. Only a creature
from that look may be selected; declining is allowed. The chosen creature enters
tapped and attacking the defender, but was never declared as an attacker. Only
the unchosen examined cards are randomly ordered on the bottom; the untouched
library suffix keeps its order. No whole-library shuffle is substituted.

The suspended look is bound to the real resolving Cub trigger, its controller,
ability index and cursor. Validation requires the complete exact top prefix,
exact eligible creature list and one-card total capacity. Titania's modal-search
guard runs **before** mode dispatch, so a forged look mode cannot evade it.
Regressions exercise validation **and** `SessionSave::from_json`, a real offered
pick followed by identical loaded continuation, whole-opponent-view equality
across hidden eligibility, atomic ineligible-take refusal, and short/empty looks.

The initial slice-48 "intervening-if" and invisible all-ineligible look claims
are superseded by this repair; historical logs remain intact. This adds no new
card or certificate: catalog 161, union 459 = 148 implemented + 311 missing,
Ayula five missing names, one of eight lists certified. Slice 50 settles the
Witness live-state predicate separately above; no new card or certificate is
claimed by that correction.

## Slice 53: Ruxa, Patient Professor — the pool's first graveyard-card target, no-abilities anthem and combat-damage-assignment choice

Ruxa, Patient Professor (**C21 66**, `{2}{G}{G}`, legendary 4/4 Bear Druid) is sourced in
`fixtures/starter-corpus/slice53-sourced.json` (Scryfall oracle id
`d760f2f0-6653-47c4-bd7b-aab2f398ddf2`, scryfall id `e07b8142-6a49-46e7-b862-41f89a59b894`,
retrieved 2026-10-28). It prints three separate abilities, and each one needed engine
surface that did not exist:

> Whenever Ruxa enters or attacks, return target creature card with no abilities from
> your graveyard to your hand.
> Creatures you control with no abilities get +1/+1.
> For each creature you control with no abilities, you may have that creature assign its
> combat damage as though it weren't blocked.

**«Enters or attacks» is two trigger entries.** `TriggerRef::ability` indexes the ability
table per event, so one printed sentence with two events is modelled as two typed
`TriggeredAbility` entries carrying the same printed text and the same effect. A single
entry could only ever fire on one of the two events.

**Clause 1 — a card, not a permanent.** `Target::Card` is a new target variant used only
for a card in a non-battlefield zone; the printed set is
`TargetSpec::NoAbilityCreatureCardInYourGraveyard`, read from the controller's graveyard
list and filtered by the shared no-abilities predicate. `Effect::ReturnGraveyardCardToHand`
moves the card with the ordinary zone-change path (`move_to_hand`), never the draw path:
routing it through the draw path would emit `Event::Draw`, fire draw triggers and set
`drew_from_empty`, none of which a return from a graveyard does. The CR 903.9b
command-zone replacement is offered for a commander card, and the
`Pending::CommanderReplacement` validator now predicts the destination from the **ability**
on the stack (not only a spell's effects) and requires `from_zone == Graveyard` with a
`Target::Card` stack target for this effect, so a forged suspension cannot choose its own
destination.

**Clause 2 — one shared predicate.** `GameState::object_has_no_abilities` is the single
answer used by the target set, the anthem and the permission: printed abilities, typed
activated/triggered abilities, static abilities, an intrinsic mana ability (CR 605.1a),
printed keyword abilities, a keyword counter (CR 122.1b), abilities granted by a continuous
effect (including an Equipment's granted keywords) and abilities granted by another
permanent's printed static. The anthem is a new layer 7c additive static
(`ControlledCreaturesWithNoAbilitiesGet`) whose recipient loop is modelled on
`OtherControlledCreaturesWithAnySubtypeGet`; the printed text has no "Other", so the source
itself is a candidate and is never special-cased (it cannot qualify while this static is one
of its abilities).

**Clause 3 — a real choice while damage is assigned.** `Pending::CombatDamageAssignment`
holds the eligible attacking creatures in active-anchored APNAP order, one answer each, and
the whole batch is dealt once after the last answer (CR 510.2). The answer action is
`Action::AnswerCombatDamageAssignment { as_though_unblocked }`; `PendingKind` projects it and
the AI gains a declaration copy (plus a heuristic that takes the permission — a policy
choice for harnesses, not strength evidence). An accepted creature assigns its whole combat
damage to the defending player and none to its blockers, while the blockers still deal their
own combat damage to it. `Event::CombatDamageAssignment` records the choice itself.

**Deliberate limits recorded, not hidden.** (a) The choice is created only for **blocked**
attackers: an unblocked creature already assigns as though it weren't blocked, so asking
would invent a decision the rules do not require. (b) Combat damage assignment for every
other creature remains the pre-existing automatic lethal-first/trample model; arbitrary
splits are still not implemented. (c) The no-abilities predicate enumerates the ability
sources this engine has today, so a future ability-granting mechanism must be added to it —
a maintenance point, stated here so it is not rediscovered as a bug. (d) The command-zone
prediction covers the two hand/library destinations this effect can produce; a
graveyard-to-library "from anywhere" effect is still unimplemented.

Regression coverage is `crates/core/tests/issue17_slice53_ruxa.rs`: the printed text
registration, both trigger events, the graveyard-card target set (with abilities, keyword
abilities and opponent graveyards excluded), no draw path, both anthem directions, both
damage-assignment answers, multi-attacker APNAP ordering, the unblocked and
has-abilities counter-cases, the commander destination, and forged pendings refused by
**both** `GameState::validate` and a real `SessionSave::from_json`.

Whole-field discipline: Ruxa left `missing_cards` **and** its `missing_interactions` entry in
`decks/starter-corpus/manifest.json` in the same change; Ayula is now four missing names and
the corpus is 162 catalog / 459 union = 149 supported + 310 missing, per-list
`[55, 0, 51, 48, 4, 52, 52, 53]`, still one of eight lists certified.

## Slice 54: Ursine Monstrosity — the CR 508.1d maximal attack requirement and indestructible

Ursine Monstrosity (**DSC 36**, `{2}{G}`, 3/3 Bear Mutant) is sourced in
`fixtures/starter-corpus/slice54-sourced.json` (Scryfall printing
`73cc6df4-3564-4ace-bf8a-eac3e62d725a`, oracle id `647feda9-2876-4430-902f-10dc1f4f99a9`,
retrieved 2026-10-28). It prints trample plus one combat trigger:

> At the beginning of combat on your turn, mill a card and choose an opponent at random.
> This creature attacks that player this combat if able. Until end of turn, this creature
> gains indestructible and gets +1/+1 for each card type among cards in your graveyard.

**«If able» is a declaration requirement, not a suggestion.** The random opponent is drawn
from the live opponents at the beginning of combat and stored as the required defender;
declaring this creature against any other player — or not declaring it — is refused while
it is able. Enforcement is CR 508.1d **maximal**: the number of satisfied requirements is
compared against the maximum achievable under the declaration actually submitted, so
attacking a different opponent cannot evade the requirement, and every offered declaration
still validates. A second creature may join the required attacker and unrelated attackers
to other seats are unaffected; the requirement ends with the combat (CR 508.1d's "this
combat") and never carries into the next one.

**Indestructible is a keyword in the state-based-action path**, not a damage-prevention
shield: lethal damage accumulates and is then ignored by the destroy check, while a
zero-toughness permanent is still put into its owner's graveyard (CR 704.5f). The +N/+N is
a layer 7c additive value fixed when the trigger resolves (CR 611.2d), counting the six
card kinds this pool can put in a graveyard, so a later mill in the same turn does not
retroactively grow it; the mill itself happens before the count.

**Deliberate limits recorded, not hidden.** (a) CR 508.1d maximisation is implemented
inside the inherited single-defender combat model, so "maximum requirements" means the
maximum this engine's declarations can express, not full multi-defender Commander combat.
(b) Indestructible does not prevent a zero-toughness graveyard move. (c) The +N/+N counts
only the card kinds this pool can actually place in a graveyard, and it is fixed at
resolution rather than recomputed. (d) A source that has already left the battlefield
when the ability resolves still mills (CR 608.2b) and now logs that its attack
requirement, indestructible and +N/+N are skipped, instead of narrating a grant that
never happened. (e) `ContinuousKind::MustAttackThisCombat`
stores the required defender in the `controller` field, so no future consumer may read
that field as a controller.

Regression coverage is `crates/core/tests/issue17_slice54.rs`,
`crates/core/tests/issue17_slice54_regressions.rs` and
`crates/core/tests/issue17_slice55.rs`: the printed trigger, random opponent choice,
maximal-requirement refusal and acceptance, wrong-opponent evasion refused, the fixed
+N/+N vs a later mill, indestructible through lethal damage and its own cleanup, zero
toughness still dying, the bounced-source control, the whole-field gap rewrite, the
departed-source mill line, and the coach wording for a zero-toughness permanent with and
without indestructible.

Whole-field discipline: Ursine Monstrosity left `missing_cards` **and** its
`missing_interactions` entry in `decks/starter-corpus/manifest.json` in the same change;
Ayula is now one missing name and the corpus is 165 catalog / 459 union = 152 supported
+ 307 missing, per-list `[55, 0, 51, 48, 1, 52, 52, 53]`, still one of eight lists
certified.

## Slice 56: zero toughness is a state-based graveyard move, not a destroy

No card entered the pool in that slice; the later Dúnedain Rangers entry moves the
current count to 165 catalog / 459 union = 152 supported + 307 missing, one of eight
certified.

**The defect.** The coach's state-based-action paragraph filed every creature with
`toughness <= 0` under "has lethal damage and will be destroyed". Both halves are wrong:
CR 704.5f puts a creature with zero or less toughness into its owner's graveyard as a
state-based action, which is **not** a destroy, and such a permanent has taken no lethal
damage. The paragraph now has three lists with their own citations — the destroy list is
restricted to `damage >= toughness > 0` (CR 704.5g, minus indestructible via CR 702.12b),
a separate zero-toughness sentence cites CR 704.5f and states that indestructible does not
prevent it, and the indestructible-and-spared list stays as before.

**Deliberate limits recorded.** The paragraph is a projection over public state only: it
names permanents and their damage/toughness, so it cannot distinguish a creature that will
actually die to a replacement effect this pool does not implement. It describes what the
state-based-action check is about to consider, in the same words the engine's own SBA path
uses; it is not a rules engine of its own.

**Regression coverage** is `crates/core/tests/issue17_slice55.rs`: the indestructible-with-
lethal-damage wording, zero toughness without indestructible (CR 704.5f sentence, no
destroy claim, no lethal-damage claim) and zero toughness **with** indestructible (the
zero-toughness sentence still wins, because CR 704.5f has no destroy to excuse), plus the
departed-source mill line from slice 55.

## Slice 57: Rampaging Yao Guai, the pool's first `{X}` spell and its first aggregate target set

**The card.** Rampaging Yao Guai, PIP 82 (Fallout), `{X}{G}{G}{G}`, 2/2 Bear Mutant with
vigilance and trample. Printed text fetched from Scryfall on 2026-10-29
(`https://api.scryfall.com/cards/63619236-a27a-4fdd-add6-7a295901f906`, printing id
`63619236-a27a-4fdd-add6-7a295901f906`, oracle id `d37e8f75-c7cb-4270-bb2f-0125e972ed15`,
retrieved 2026-10-29T02:29:04Z, recorded verbatim in
`fixtures/starter-corpus/slice57-sourced.json`). The catalog string is that text, so the
three printed clauses are the whole card and nothing is paraphrased.

**The three clauses, each enabled.**

1. *`{X}{G}{G}{G}`* — X is announced through `Action::CastSpellWithX` (CR 107.3a), paid as
   additional generic mana, and carried on the stack item. The plain cast action is refused
   for this card and the X cast is refused for a card that does not print `{X}`, so X is
   never silently zero. The announced value survives resolution, a suspended pending choice
   and a save/load round trip.
2. *"This creature enters with X +1/+1 counters on it"* — an entry replacement (CR 614.1c)
   applied by the entry itself, after the zone change (CR 122.2 clears counters that
   belonged to the previous zone) and before the entry is announced, with X read from the
   resolving spell rather than from the board.
3. *"When this creature enters, destroy any number of target artifacts and/or enchantments
   with total mana value X or less"* — a target *set* under one aggregate restriction
   (CR 115.1, 601.2c). The trigger records X as its budget, offers only artifacts and
   enchantments that fit, keeps choosing until the controller commits with
   `Action::FinishTriggerTargets` (the empty set is legal — "any number" includes zero), and
   destroys the committed set as one instruction (CR 701.7a, 704.3).

**Deliberate limits recorded.**

* The aggregate total is a *targeting* restriction (CR 601.2c), checked as each target is
  added; it is not re-derived at resolution. At resolution each committed target that is
  still an artifact/enchantment is destroyed; one that left the battlefield does nothing
  (CR 608.2b), and if every target is gone the ability does not resolve. The identical
  card in two zones is two objects, so a bounced target is simply no longer a target.
* A permanent's mana value for the restriction is its printed cost with `{X}` read as 0
  (CR 202.3). Copy effects, cost-modifying effects and face-down cards are outside the
  supported pool, so no other reading is claimed.
* `{X}` is paid from the mana pool and untapped lands, with the supported announced range
  `0..=20`; a static cost reduction applies to the *total* generic cost, so an announced X
  is reduced like printed generic mana (CR 601.2f).
* Counters are +1/+1 counters on the permanent and are read by the characteristic path
  (CR 613.4d) exactly like every other counter in the pool; there is still no full ordered
  layer 7d pass, and that pre-existing limit is unchanged by this card.
* The two printed keywords are the two mechanisms the pool already has: vigilance is the
  keyword-table entry the attack path consults before tapping an attacker (CR 702.20a), and
  trample is the `CardDef` flag the damage assignment reads.
* A "you cast a creature spell" trigger that reads the spell's mana value sees `3 + X`
  while the spell is on the stack (CR 202.3), which is the only place X counts.

**Whole-field discipline.** Rampaging Yao Guai left `missing_cards` **and** its matching
`missing_interactions` entry in `decks/starter-corpus/manifest.json` in the same change; the
whole-field audit recomputes every list from the catalog and the deck files. The corpus is
now 164 catalog / 459 union = 151 supported + 308 missing, per-list
`[55, 0, 51, 48, 2, 52, 52, 53]`, Ayula two missing names (Dúnedain Rangers and Evercoat
Ursine), still one of eight lists certified. The pool grew by exactly one
(163→164) and no other list's gaps moved.

**Regression coverage** is `crates/core/tests/issue17_slice57.rs`: the printed text and mana
value, the two cast-action refusals, an unaffordable and an out-of-range X, X=0 as a real
cast with no counters, X counters with the generic actually paid, the offer inside the
budget (both controllers, artifacts and enchantments, MV 4 excluded at X=2), a set that
reaches but never exceeds the budget, a duplicate target refused, the chosen set destroyed
together while an unchosen permanent survives, the empty set destroying nothing, a departed
target costing only its own destruction, a mid-choice save/load continuing to the same
result, and six forged pendings refused by both `GameState::validate` and a real
`SessionSave::from_json` load.

## Slice 58: saved aggregate-mode and announced-X validation repairs

Review 348 found two structural sandbox-save holes in slice 57. The aggregate
target mode is now derived from the printed ability **before** inspecting the saved
mode. A missing aggregate discriminator cannot downgrade Yao Guai to a single-target
trigger. The shared load/answer guard rechecks candidate eligibility through live
targeting rules, individual mana values, duplicates and the chosen set's total;
direct forged answers and commits are refused without changing the state.

The announced-X bound `0..=20` is shared by casting and save validation. Nonzero X
must belong to an X-cost spell or its supported X-reading triggered ability, never
an unrelated spell, activated ability or trigger. Validation covers stack entries
(including a resolving spell's retained entry), the trigger queue, pending triggers
and their queued siblings. This is structural validation, **not** reconstruction of
past mana payments from a sandbox edit; ordinary saves retain replay-integrity checks.

`crates/core/tests/issue17_slice57.rs` adds eight review-348 regressions, including
separate omission/candidate-injection refusals, independent duplicate/aggregate-total
cases, nonzero-X stack and post-first-pick sandbox-load continuation with equal full
states, and normal single-target load controls. No card is added: catalog 164,
Ayula's remaining names Dúnedain Rangers and Evercoat Ursine, still one of eight
lists certified. Ring, linked hideaway/free play and exact-list certification remain
unfinished work, not implicit support from these save repairs.

## Slice 59: Dúnedain Rangers and a real trigger/stack Ring

**Card.** Dúnedain Rangers, LTR 159, `{3}{G}`, Human Ranger 4/4. Source and oracle text
were fetched from Scryfall and recorded in `fixtures/starter-corpus/slice59-sourced.json`.
Its landfall instruction is represented by `Effect::RingTempts` behind a
`NoRingBearer` intervening-if. The condition is evaluated when the landfall trigger is
queued and again on resolution, using the controller captured on the trigger, so a source
that has left the battlefield does not lose the trigger. Two landfall triggers queued with
no Ring-bearer both reach the stack; after the first resolution assigns a bearer, the
second fails its intervening-if and is removed.

**The Ring representation.** `CardId::TheRing` is an engine-only emblem definition, not a
token, permanent, importable card, commander, library card or member of `catalog::all` /
`by_name` / `pool_size`. `catalog::def` serves its canonical metadata so the normal stack
machinery can resolve its four typed triggered abilities:

1. Ring level 1: the Ring-bearer is effectively legendary and cannot be blocked by a
   creature with greater power. The legend SBA counts only legendary permanents with the
   same name (printed legendary, or the controller's Ring-bearer), so one legend beside an
   ordinary same-name card is not a legend-rule choice; only that group's controller answers.
2. Ring level 2: whenever the Ring-bearer attacks, the attack trigger draws a card, then
   asks for a discard through the distinct `Pending::EffectDiscard`. That answer resumes
   the resolving The Ring ability; it is not the cleanup-step discard and cannot end combat.
3. Ring level 3: becoming blocked queues a The Ring trigger; its resolution records a
   delayed marker for the captured blocker. EndCombat turns that marker into a real The
   Ring delayed stack trigger. A bounced and recast blocker has a new ObjectId, so the old
   marker cannot sacrifice the new incarnation.
4. Ring level 4: only positive combat damage to a player queues the life-loss trigger.
   Fog/prevention therefore does not trigger it. The three-life loss is applied when the
   trigger resolves, after the combat-damage SBAs, to every live opponent.

`RingState` remains the per-seat emblem state (`temptations` and `bearer`). Ordinary
blocking no longer materializes an empty Ring vector; only an actual temptation creates
the exactly-seat-count projection. Loader validation and both direct Ring/legend answer
validators require either an empty vector or exactly one slot per seat, levels 0–4, a
bearer on that controller's battlefield with level > 0, a unique bearer, and exact
same-name legend options. The temptation offer must equal the resolving Rangers'
full live controlled creature set; a foreign, short or duplicated offer is refused.
The Ring's abilities are not abilities of the Ring-bearer, so the shared no-abilities
predicate still treats a vanilla bearer as vanilla.

**PlayerView** now carries the public `rings` projection (level and bearer id). AI search
reconstruction copies that public projection; it does not read hidden state and does not
change policy, ranking, model or budget. A bearer id that cannot be represented in the
search world's copied public object ledger is omitted from the search projection only.

**Limitations.** The emblem is not a distinct physical object in a zone: the bearer id on
a trigger is combat-event provenance, and `RingState` is the persisted emblem. There is no
full independent rules implementation of all Ring-bearer replacement/control-change
interactions. The evidence journey reaches Ring level 2 for one seat (a standing Ring-bearer
stops further Rangers temptations by its own intervening-if), so levels 3-4 are exercised by
the focused trigger tests and not by that journey. The final evidence journey is a 40-card
package, not a 100-card starter certification. At that slice the corpus was 165 catalog /
459 union = 152 supported + 307 missing, Ayula one name short, and only the Jasmine list
was certified (1 of 8); slice 60 (below) closes the Ayula gap.

**Legend-rule correction.** The first implementation treated a Ring-bearer's whole
same-name group as legendary, which would have sacrificed an ordinary second Grizzly Bears.
CR 704.5j applies to legendary permanents, so the SBA now groups only printed legendary
permanents or their controller's level-1+ Ring-bearer, and a compiled regression proves one
legend beside an ordinary same-name card is not a legend choice (and that a forged truncated
choice is refused by both `GameState::validate` and a real save load).

**Evidence.** `crates/core/tests/issue17_slice59_repairs.rs` (native seven), 
`crates/core/tests/issue17_slice59_ring_repairs.rs` and
`crates/core/tests/issue17_slice59.rs`; final logs and journey under
`evidence/issue17/slice59/final/`.

## Slice 60 — Evercoat Ursine: hideaway twice, linked face-down exile, free play

The last name in the Ayula list is implemented from its authoritative printing:
**Evercoat Ursine** (BLC 30, oracle `65c32889-…`, printing
`b299d768-b171-47e5-b69d-82ce13917c43`, `{4}{G}`, Creature — Elemental Bear,
6/5, Trample; Scryfall fetch recorded in
`fixtures/starter-corpus/slice60-sourced.json` with retrieval time
2026-10-29T06:58:12Z, 2 printings, 1 distinct rules text, and the card's six
official rulings).

**Printed text and implementation.** `Trample` is a real keyword flag.
`Hideaway 3, hideaway 3` is two instances of the keyword, so the card carries two
separate enters-the-battlefield triggered abilities; entering resolves two
independent looks (CR 702.75a). Each look is private (CR 701.20a), exiles exactly
one of the looked-at cards face down, links it to that permanent
(`GameState::hideaway`, `GameState::face_down_exile`), and puts the *looked-at*
cards that were not chosen on the bottom in a random order (CR 701.20e) — the
rest of the library is never reordered. The third ability is a normal triggered
ability on "this creature deals combat damage to a player" whose intervening-if
"if there are cards exiled with it" is evaluated when it triggers and again at
resolution (CR 603.4); the "you may" is the ordinary CR 603.5 yes/no choice, and
the play itself is a resumable `Pending::PlayExiled` offer with one action per
playable linked card plus an explicit decline.

**The printed ruling fixes four things, and all four are enforced.** The card is
played while the ability resolves, so no normal timing restriction applies; a
land may be played only while the seat still has its land play for the turn
(CR 305.2); a printed `{X}` is 0; and no alternative cost is chosen (the pool has
no alternative-cost cards). Playing the card for free casts it through the same
stack and cast-trigger path as a normal cast with no mana paid, and the card stops
being exiled with anything the moment it changes zone (CR 400.7).

**Deliberate limits.** Hideaway's "any player who has controlled a permanent with
a hideaway ability since a card was exiled with it may look at that card" is
recorded only for the player who controlled the exiling permanent; control
changes of that permanent are not tracked for the look permission. Additional
costs on a free-cast card are not modelled because no card in the pool prints
one. A card whose exiling permanent has left the battlefield stays exiled face
down but is unreachable, which is the correct CR 400.7 outcome. The face-down
flag is a `GameState` set, so a save from before this slice is byte-identical.

**A real defect in the accepted slice-54 increment, fixed here.** Ursine
Monstrosity printed `Trample` but was registered through the plain catalog macro,
so its own keyword was not implemented. It is now a `card_trample!` row and the
regression `ursine_monstrosity_now_really_has_its_printed_trample` keeps it that
way.

**Corpus after this slice.** 166 catalog / 459 union = **153 supported + 306
missing**, per-list `[55, 0, 51, 48, 0, 52, 52, 53]`, and **two of eight lists
certified**. The Ayula list is `implemented` and then `certified`: every card of
the exact 100-card snapshot is supported, both gap fields are empty, and the
exact snapshot was played.

**Certification (2 of 8).** `aperture-cli certify --starter ayula-bear-counters
--seats 4 --seeds 1,2,3` reads the declared snapshot
`decks/starter-corpus/ayula-bear-counters.txt` (99 library cards, sha256
`7a1b3f9c…19c4` over the LF-normalised file), puts that exact list in every
seat with Ayula as the commander, and plays three four-seat 40-life
Commander-combat games with the heuristic. All three reach a terminal outcome
(seeds 1/2/3: winners P1/P3/P2, 1499/1373/1233 actions, digests
`78b301cbef68…`, `8ccf4bc4a753…`, `0adc5f3097ea…`) and every recorded replay
reproduces its digest. The same three games were replayed separately through
`selfplay --replay-out` and `replay --file`, which report `matches recorded` and
exit 0 (`evidence/issue17/slice60/replays/`). 31 of the 59 distinct names were
cast or played in those games; the other 28 are covered by engine conformance
suites, never inferred from the catalog.

The certificate's exclusions are computed from the list itself: this one really
does contain artifacts and Equipment (Sol Ring, Arcane Signet, Fellwar Stone,
Swiftfoot Boots, Lightning Greaves, Stuffed Bear, Friendly Teddy) and
enchantments (Ayula's Influence, Bearscape, Words of Wilding, Beorn's
Hospitality, Dancing from Dark to Dawn), so the certificate states that they are
implemented and covered by suites while these three games only exercise the ones
actually cast. It also records the hideaway look-permission and free-cast
additional-cost limits above.

**Evidence.** `crates/core/tests/issue17_slice60.rs` (nine regressions, including
the two real `SessionSave` probes), the printing chain in
`crates/core/tests/card_printings.rs`, the certificate
`evidence/issue17/slice60/ayula-certificate.json` and the separate replay
verification under `evidence/issue17/slice60/replays/`.
