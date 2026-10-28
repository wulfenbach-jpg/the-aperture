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
* **135** are in the supported 147-card catalog and **324** are missing. Slice
  25 adds `Exuberant Wolfbear` (129→130), slice 26 adds `Ursine Champion`
  (130→131), slice 28 adds `Circle of the Moon Druid` (131→132), slice 29
  adds `Stuffed Bear` (132→133), slice 30 adds `Drover Grizzly` (133→134) and
  slice 31 adds `Beorn the Fierce` (134→135) to the previously verified split. The union is unchanged because every one of them
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
one except the layer 4 forms already supported; nothing in the pool grants or removes hexproof,
shroud or protection, so none of those interacts with the token. This slice certifies no starter
list: Ayula has 13 missing names and the eight-list gate remains 1 of 8.

## Slice 41: The Earth King
The Earth King (`{3}{G}`, 2/2 Legendary Human Noble Ally) is sourced in `fixtures/starter-corpus/slice41-sourced.json` (TLA 172, immutable a8d5dca6, retrieved 2026-10-28T07:02:29Z). Its ETB creates a distinct 4/4 green Bear token (CR 111.4 name, nonimportable). Its attack trigger fires once when one or more of its controller's current 4+ power creatures attack and opens an incremental private basic-land search with a captured count; any number up to that count may be chosen, chosen lands enter tapped through the canonical land-entry path, and the library shuffles even for zero choices. Search choices are owner-only in projections. A land put onto the battlefield by any effect now fires existing landfall watchers once per land (CR 603.6a) without spending the land play.

**Exclusion, not a convention.** The legend rule (CR 704.5j) is not implemented. Until this slice every legendary card in the pool was gated as commander-only by `DeckList::parse_practice`, so no legendary permanent could reach a library and the state-based action had no reachable input. The Earth King is not in `SUPPORTED_COMMANDERS`, so it *is* admitted into practice libraries, and its 40-card evidence package carries four copies: two copies of the same legendary permanent can legally coexist on this engine's battlefield. A real Commander list is singleton, so the Ayula certification path cannot reach that state, but the gap is real for ordinary practice decks and is recorded here rather than papered over as an existing convention. Fixing it means implementing the CR 704.5j controller choice, which is its own slice.

## Slice 42: shared Equipment — Lightning Greaves and Swiftfoot Boots
Lightning Greaves and Swiftfoot Boots are ordinary artifact Equipment with typed attachment effects. Equip is a targeted activated ability with its printed mana cost, sorcery timing, source-controller restriction and target creature you control; attachment changes only when the equip ability resolves, and the previous attachment persists while the new equip is on the stack. Greaves grants haste/shroud; Boots grants haste/hexproof. Shroud blocks all targeting, hexproof blocks only opponents; player targets and nontarget effects are unaffected. Attachments end when either object leaves the battlefield, when the creature ceases to be a creature, or on a later incantation of the same ObjectId. No broader Equipment/Aura/protection support is claimed.
