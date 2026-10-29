## Partial starter corpus — issue #17 (draft, not acceptance)

Eight desired 100-card Commander snapshots are published. Only `jasmine-vanilla-stompy` is certified end to end; seven remain planned/unsupported. This is an increment toward the original eight-list gate, NOT full Commander or v1.

### Current state

- Catalog **149 real cards**.

- Desired eight-list union **459 names = 137 supported + 322 missing** (per-list missing `55/0/51/48/16/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **16 missing names**.

- Last **behavior** change: slice 33, `Beorn's Hospitality` (first layer 7a characteristic-defining power/toughness, code head `01fdc3b`), repaired in slice 34 (`5b60c2c`) after review 290; `Kamahl's Summons` plus `Surrak and Goreclaw`'s printed trample (slice 32) remain the previous increments.

### Latest increment: slice 33 — Beorn's Hospitality, with the review-290 target repair in slice 34

`Beorn's Hospitality` (`{1}{G}` Enchantment; Scryfall **HOB 120**, immutable `153ca57e-30f0-4ad7-ae9d-c55cbf0fd4c9`, oracle `d9ed7252-11d1-432b-9101-ac08cd28826d`, retrieved **2026-10-27T18:21:29Z**; every printing checked for one identical text) prints "Landfall — Whenever a land you control enters, put a +1/+1 counter on target creature you control. / {5}{G}{G}: This enchantment becomes a Bear creature in addition to its other types and gains \"This creature's power and toughness are each equal to the number of lands you control.\" (This effect doesn't end.)"

* **Layer 7a now exists.** `Layer::CharacteristicDefining` (CR 613.4a) with `ContinuousKind::PowerToughnessEqualToLands`, recomputed from the board on every layer pass, so a land entering after the animation raises the P/T with no second activation (CR 604.3). A 7b snapshot would fail the test that proves it.
* **A duration that does not end.** `EffectDuration::WhileSourceOnBattlefield` on an activated ability, because the card prints "(This effect doesn't end.)" (CR 611.2b): cleanup leaves it, departure ends it (CR 400.7).
* **Reviewed repair (slice 34).** Review 290 showed the landfall trigger reused `TargetSpec::CreatureYouControl`, which removed the ability's own source to model Little Bear's "another". The spec is split: `CreatureYouControl` is the printed set **including** the source, and the new `AnotherCreatureYouControl` is the only one that excludes it (Little Bear keeps it). Five regressions cover the animated self-target, the source offered beside another creature, the unanimated source not being legal, the CR 603.3d removal when no creature exists, and Little Bear's excluded source.
* **Verified at `bfbbceb`** (review 295 APPROVED_INCREMENT): `issue17_slice33` 12/12, `card_printings` 12/12, workspace 119 suites / 846 passed / 0 failed / 1 ignored, fmt and clippy `-D warnings` clean, two discriminating mutations (source re-excluded: 4 failures; Little Bear pointed at the plain spec: 1 failure) with `sha256sum -c` clean pre/post receipts, a four-seat / 40-life exact-snapshot journey rebuilt at the repaired head (terminal `Win{P3}`, 1117 actions, 2967 events, digest `3dfb14d4ce76…`, demo artifacts byte-identical, separate CLI replay matching) and 10/10 exact-head CI SUCCESS keyed to `b74815d` and to `bfbbceb`.
* The journey is a 40-card evidence package driven by a masked demonstrator, **not** a 100-card certificate and not product AI or strength evidence. Original #17 remains unfinished: 1 of 8 lists certified, 322 of 459 union names unsupported.

### Latest increment: slice 32 — Kamahl's Summons (ONS 269) and Surrak's printed Trample

`Kamahl's Summons` (`{3}{G}` Sorcery; Scryfall **ONS 269**, immutable `0edc37c6-b6a8-424f-95dd-928d03c28542`, oracle `8b6ef0a2-cd90-44c8-8f11-e1983f2b1dfd`) prints "Each player may reveal any number of creature cards from their hand. Then each player creates a 2/2 green Bear creature token for each card they revealed this way." (fixture `fixtures/starter-corpus/slice32-sourced.json`, enforced by `crates/core/tests/card_printings.rs`).

Reusable machinery, not a one-card patch:

* `Pending::ApnapReveal` is the pool's first **per-seat arbitrary hand subset**: every live seat answers in APNAP order from the active player (CR 101.4, 608.2f), and `Action::RevealCreatureCards` names any subset of that seat's own creature cards (CR 701.9a). An empty list is a real answer, because the card prints "may reveal **any number**".
* Revealing never moves a card: the revealed cards stay in their owner's hand, so the only thing a reveal buys is tokens. Validation is typed — a card not in that seat's hand, a non-creature card, a duplicate, or a reveal when nothing is pending is refused with `EngineError::Illegal` and leaves the pending choice untouched.
* **Nothing is applied per answer.** Every seat's count is fixed by its own reveal and the Bears are created together, in APNAP order, only after the last seat has answered, so no seat's result depends on the answering order.
* The pending reveal is projected only to the seat that owes it (like the APNAP sacrifice queue) because the offer names that seat's hand; the answers are public and logged as `Event::CardsRevealed`.
* `Surrak and Goreclaw` now prints its **own** Trample: the row moved from `card!` (trample false) to `card_trample!`, so `Game::has_trample` answers it exactly like every other trampling card. Its MOM 337 printing (`keywords: ["Trample"]`) is recorded in the same fixture.

Verification: 4/4 focused slice-32 tests (three-seat APNAP reveal with counts 1/2/0 and cards still in hand, every refusal, mandatory-target decline neither offered nor legal, Surrak's real trample assignment) + 2 provenance tests; four discriminating mutation REDs with every touched file restored byte-for-byte; workspace **118 suites / 833 passed / 0 failed**; fmt, `clippy -D warnings` and locked release clean; a real four-seat/40-life exact-snapshot journey (`decks/package-slice32.txt`, seed 3232, terminal `Win{P3}`, 1362 actions, 32 turns, 3660 replay events, digest `f6e65209…`, 12 APNAP reveal answers, 23 revealed cards -> 23 Bear tokens, 10 Surrak combat-damage events) with a byte-identical rerun and a separate release CLI replay matching. The Surrak repair moves exactly one previously recorded journey: slice 24 `a41bbfd2…`/4127 events -> `697a846d…`/4130 events (same 1602 actions and decisions); every other committed journey digest is unchanged (`evidence/issue17/slice32/replay-drift.log`).

### Previous increment: slice 31 — Beorn the Fierce (HOB 119)

`Beorn the Fierce` (`{3}{G}{G}`, 6/6 Legendary Creature — Bear Shapeshifter Warrior; Scryfall **HOB 119**, immutable `367d5f8b-77ee-47f7-bc71-972d62c280a9`, oracle `30ca8e92-5955-46a1-86c1-094a873f518f`, retrieved **2026-10-27T15:33:31Z**) prints "Trample / Other Bears you control get +2/+2. / At the beginning of combat on your turn, put a trample counter on up to one target creature you control. It becomes a Bear in addition to its other types. Then if you control three or more Bears, draw two cards." (fixture `fixtures/starter-corpus/slice31-sourced.json`, enforced by the provenance guard in `crates/core/tests/card_printings.rs`).

Reusable machinery, not a one-card patch:

* `TriggerEvent::BeginningOfCombat` (CR 507.1) is dispatched once when the beginning-of-combat step starts, only for the active player's permanents, before any player gets priority; nothing is stored, so it cannot double-fire.
* `TargetSpec::UpToOneCreatureYouControl` + `Action::DeclineTriggerTarget` (CR 115.6) make "up to one target" an honest choice: the engine asks whenever a legal target exists — including exactly one — and always offers "choose no target". Declining still resolves the ability, so the conditional draw is unaffected; decline stays illegal for mandatory-target triggers (Ayula, Little Bear, Wolfbear).
* `CounterKind::Trample` is the pool's first keyword counter (CR 122.1b). `Game::has_trample` reads it, cleanup never removes it, and the permanent keeps trample after the turn ends — the subtype does not.
* `Effect::AddTargetCreatureType` starts a layer 4 *additive* subtype effect on the chosen target (CR 613.1d); because the ability prints no duration it lasts until end of turn (CR 611.2a) and cleanup ends it.
* `StaticAbility::OtherBearsYouControlGet` is a layer 7c subtype anthem (CR 613.4c) whose recipients are computed from the **effective** subtype — a creature a layer 4 effect just made a Bear receives it — and which never pumps its own source.
* `Effect::DrawIfControlsBears` evaluates the Bear count after the earlier instructions of the same resolution (CR 608.2c), so the creature this ability just turned into a Bear counts toward the three.

Verification: 8/8 focused slice-31 tests; four discriminating mutation REDs (trample-counter read dropped, Bear-count gate ignored, optional target made mandatory, subtype anthem disabled) with `engine.rs` restored byte-for-byte; workspace **117 suites / 827 passed / 0 failed**; fmt, `clippy -D warnings` and locked release build clean; a real four-seat/40-life exact-snapshot journey (`decks/package-slice31.txt`, seed 3131, terminal `Win{P2}`, 1917 actions, 46 turns, 4930 replay events, digest `36e18185…`, 13 triggers, 13 trample counters, 13 Bear additions) with a separate release CLI replay matching; and 10/10 exact-head CI SUCCESS all keyed to `e53b987`. Independent review: **APPROVED_INCREMENT**, no functional defect (`evidence/issue17/review287/REVIEW287.md`).

Disclosed limits: the journey is a 40-card package, not a 100-card starter certificate, and the driver is an evidence-only masked demonstrator, not product AI or strength evidence. Newly discovered and **not** fixed here: `Surrak and Goreclaw` prints its own Trample but its catalog row is a plain `card!` (trample `false`) and its static grants trample only to other creatures, so Surrak itself does not trample; repairing an approved card's behavior would also move the digests of the journeys that used it, so it is recorded for a separately reviewed increment. This slice certifies no starter list: Ayula still has **18** missing names and the eight-list gate remains **1 of 8**.

### Previous increment: slice 30 — Drover Grizzly and the pool's first saddle

`Drover Grizzly` (`{2}{G}`, 4/2 Creature — Bear Mount; Scryfall **OTJ 161**, immutable `560062cd-34f8-4d30-9e25-099b03961724`, retrieved **2026-10-27T14:40:04Z**) prints "Whenever this creature attacks while saddled, creatures you control gain trample until end of turn." and "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)" (CR 702.171a-c; pinned rules text `evidence/issue17/slice2-rules-source.txt` lines 5280-5286).

This is reusable machinery rather than a one-card patch: `Action::SaddleMount { source, creatures }` types the cost as *other untapped creatures you control* (summoning-sick helpers legal, source excluded, duplicates/opponents/tapped/insufficient power refused, helpers tap on payment), the ability resolves through the ordinary stack into a `Saddled` designation effect that cleanup ends and battlefield departure prunes, `TriggerCondition::SourceIsSaddled` types the printed intervening condition so an unsaddled attack produces no trigger at all, and resolution grants trample to every creature controlled *then* through the existing layer-6 grants with captured source identity. The saddle stack item carries `affected: Some(source)`, so a source that leaves resolves with no effect and no transfer to a later same-ID incarnation. All new rules sites ask `Game::is_creature`, so an animated Stuffed Bear can saddle and receive trample while a plain artifact cannot.

Native inspection of the first cut found two real defects (printed-only creature queries in the new rules; unbound saddle stack item) and returned them to the same writer, repair `dea6fab` — disclosed as GREEN-strengthening test additions, not strict TDD. Verification: 61 focused tests / 0 failed with a committed `run-native.sh` that prints every exit code (fmt, workspace Clippy `-D warnings`, locked release build, journey, separate replay, audit all `=0`), a real four-seat/40-life package journey on exact snapshots (`decks/package-slice30.txt`, terminal `Win{P1}`, 2653 actions, 55 turns, 6968 replay events, digest `b711d966…5677`, 9 Drover casts, 116 offered saddle activations, 22 trample grants) with a separate CLI replay matching, and an independent review **APPROVED_INCREMENT** in the same original reviewer history (`evidence/issue17/slice30/REVIEW30-ORIGINAL-HISTORY.md`). Disclosed limit: saddle *offers* enumerate at most 10 eligible helpers, so above that cap no saddle action is offered although direct validation accepts a legal set — a bounded-practice limitation, not whole-card conformance. This slice certifies no starter list: Ayula still has **19** missing names and the eight-list gate remains **1 of 8**.

### Previous increment: slice 29 — Stuffed Bear (first animation of a noncreature permanent)

`Stuffed Bear` (`{2}` Artifact; Scryfall **MID 259**, immutable `4cd40276-016b-475f-906f-74d31dceaf70`, oracle `1402c116-194e-49ca-a8c1-09ca659c220c`, retrieved **2026-10-27T13:14:48Z**; both printings MID 259 / DBL 259 carry identical wording) prints "{2}: This artifact becomes a 4/4 green Bear artifact creature until end of turn." It is the pool's first permanent that is *not* a creature until an ability animates it, and one printed ability spans three layers: **layer 4** (CR 613.1d) *adds* the creature card type and the Bear subtype to the printed Artifact type, **layer 5** (CR 613.1e) sets the colour to green, and **layer 7b** (CR 613.4b) sets base power/toughness 4/4. All three are ordinary until-end-of-turn `ContinuousEffect`s, so cleanup ends them (CR 514.2) and departure prunes them (CR 400.7) through the existing pipeline.

The rules layer now asks `Game::is_creature` everywhere instead of reading the printed card type: an animated artifact can be targeted as a creature, attack, block, deal and receive damage, and die as a *creature* death (the death record captures the creature answer at the moment it leaves, so dies-watchers drain correctly). `Game::has_color` is the layered colour query with the printed colour as its baseline, and `CardDef::is_color`/`is_black` are its printed predicates. A permanent that prints an animation records CR 302.6 summoning sickness when it *arrives*, so an artifact animated on the turn it entered cannot attack, while one animated on a later turn can.

Verified: 9 focused slice-29 tests plus 197 focused tests across 22 suites, workspace **115 suites / 812 passed / 0 failed**, fmt / `clippy -D warnings` / locked release build clean, four discriminating mutation REDs (layer 4 addition ignored, layer 5 ignored, layer 7b ignored, entry sickness dropped) each with `engine.rs` restored byte-for-byte, and a real four-seat 40-life package journey (`decks/package-slice29.txt`, terminal `Win{P3}`, 1912 actions, 44 turns, digest `ac16e510ca4691a69981beafcb577f3aa8f01bd6973710f86b89275899030024`, separate CLI replay matches) whose driver asserts the animated-or-plain invariant after every accepted action (1301 animated and 3741 plain sightings, 119 activations). This slice certifies no starter list: Ayula still has **20** missing names and the eight-list gate remains **1 of 8**.

### Latest increment: slice 28 — Circle of the Moon Druid (first turn-conditioned printed ability)

`Circle of the Moon Druid` (`{2}{G}`, printed 2/4 Creature — Human Elf Druid; Scryfall **AFR 177**, immutable `700c9ed8-6bfe-4f1e-855a-5f31e0d2fc9b`, retrieved **2026-10-27T12:07:11Z**, both printings AFR 177 / HBG 204 identical) prints "Bear Form — During your turn, this creature is a Bear with base power and toughness 4/2. (It loses all other creature types.)" It is the pool's first printed continuous ability conditioned on whose turn it is (CR 611.3a), and one printed ability spans two layers: **layer 4** (CR 613.1d) replaces the printed subtype line, and **layer 7b** (CR 613.4b) sets base power/toughness 4/2. Neither half is a stored continuous effect — the condition is answered when the characteristic is read, so nothing expires in cleanup and no effect object can be orphaned by a bounce or a reload. `Game::has_subtype` now answers the printed line before effects, so Little Bear's Bear branch, Ayula's Bear offer lists and Exuberant Wolfbear's "target Human you control" all read the effective subtype. `enter_battlefield` recomputes characteristics when the entering card prints a layer 4/7b static; additive printed 7c statics keep their existing recompute points (disclosed debt, not changed here).

Verified: 5 focused slice-28 tests plus 172 focused tests across 18 suites, workspace **114 suites / 803 passed / 0 failed**, fmt / `clippy -D warnings` / locked release build clean, three discriminating mutation REDs with `engine.rs` restored byte-for-byte, a real four-seat 40-life package journey (`decks/package-slice28.txt`, terminal `Win{P3}`, 2065 actions, 50 turns, digest `48bf3ae5…`, separate CLI replay matches) whose driver asserts the printed 4/2-or-2/4 invariant after every accepted action, and an independent read-only review (**APPROVED**, `evidence/issue17/review283/REVIEW283.md`). CI on `6f58f75`: **10/10 check runs SUCCESS keyed to that head**, including the pull_request-triggered run (the earlier `crates/ai/tests/review.rs` merge-ref failure was master drift and is fixed on master by `73820d7`). Checkpoint `evidence/issue17/SLICE28-HUMAN-CHECKPOINT.md`.

### Latest increment: slice 27 — record repair for review 280 (no behavior change)

Round-2 review of the slice-26 delta found **no functional defect** and four record defects, three of them in files slice 26 itself edited. All four are corrected on this head:

1. `crates/core/src/layer.rs` (module header and the `Layer` enum doc) and `engine.rs::recompute_layers` still described the engine as implementing **only layer 7c** and called layers 4-6/7b unimplemented. `Layer::Type` (4), `Ability` (6), `Setting` (7b) and `Modifying` (7c) all exist and are exercised by slices 23-26; the documents now state the four implemented layers and scope the gap to layers 5 (colour change), 7a and 7d.
2. `docs/issue17-starter-corpus.md` described "7b colour changes" as missing. Colour change is layer 5; 7b (base power/toughness setting) has been implemented since slice 25. The sentence now reads layer 5 colour changes plus non-creature animation as the remaining layer gap.
3. `crates/core/src/card.rs` still said Little Bear's counter is decided by the target's *printed* subtype. The branch reads `Game::has_subtype` (effective, layer-4 aware); the comment now says so.
4. `evidence/issue17/slice24-journey-driver/src/main.rs` asserted the pre-`c2d9b35` event wording (`"enters with a +1/+1 counter and gains haste"`), so the shipped driver panicked at the recorded head and the slice-24 evidence was not reproducible from the record. The one-line correction is now **committed** at `27d8f48` (no tracked file under `crates/` or in the driver is modified at this head), and the driver re-run from the committed bytes reproduces **terminal `Win{P3}`, 1602 actions / 37 turns / 4127 events, digest `a41bbfd2…`** with `demo-save.json`/`demo-replay.json` byte-identical (`slice27/slice24-driver-head-run.log`, `slice27/slice24-artifact-hashes.txt`).

Also corrected: `80fff73` is described as **comment-only source edits with no behavior change** (`crates/core/src/{ability,catalog,engine}.rs`, `tests/issue17_slice26.rs`), not "evidence-only".

Verification at `27d8f48` (all run this round): `fmt --check` clean, `clippy --workspace --all-targets --locked -D warnings` clean, **122 focused tests pass**, workspace **113 suites / 798 passed / 0 failed**, and an independent corpus audit recomputed from `catalog.rs` + the deck files with a manifest-equality assertion: **143 / 459 = 131 supported + 328 missing**, per list `55/0/51/48/22/52/52/53`, **1 of 8 certified** (`slice27/corpus-audit.py`, fails on any mismatch).

**Worktree state, stated precisely.** Four tracked evidence files from the slice-23 finalisation still carry uncommitted edits that predate this repair and touch no code or driver (`SLICE23-HUMAN-CHECKPOINT.md`, two session `.jsonl` append-only logs, `slice23/native-gates.log`). They are disclosed here rather than folded into `27d8f48`, so the reviewed and CI-green head does not silently absorb unrelated history appends.

**Merge-ref CI, stated precisely.** The `push` run keyed to `27d8f48` is **SUCCESS 5/5** (`evidence/issue17/slice27/ci-27d8f48.json`). The `pull_request` run for the same commit is **FAILURE** on all three rust jobs, and it is not caused by this delta: the PR run compiles the synthetic merge with `origin/master` `51dffda` (#28 move review, PR56), whose new `crates/ai/tests/review.rs:60` passes a `u8` to this branch's `crates/core/src/mana.rs:86` `add(&mut self, Color, u32)`. That widening is branch-only commit `e300bdd` (approved slice-9 work) and is absent from master, so the merged tree does not compile until the two lanes reconcile the mana API. This is an integration dependency owned by the supervisor, not a slice-27 defect, and it is flagged as a hotspot on the card.

### Behavior increment: slice 26 — Ursine Champion (record corrected, code `9cd0b68`)

- Exact M19 304 printing `e60b6c02-c1f1-4ab9-bd62-ae3b447a898f`, oracle `4f142d99-8146-4b16-a2cc-ac03da19b960`, retrieved `2026-10-27T09:59:58Z`; raw response, stamp and fixture `fixtures/starter-corpus/slice26-sourced.json` retained. Text: `{5}{G}: This creature gets +3/+3 and becomes a Bear Berserker until end of turn. Activate only once each turn.`
- The pool's first **layer 4** (CR 613.1d) effect: `Layer::Type`, a closed `layer::SubtypeLine`, `ContinuousKind::SetCreatureSubtypes`, `Effect::SetSourceCreatureSubtypes`, and `Game::has_subtype` as the effective-subtype query now used by the Bear/Human target filters, Little Bear's counter branch, Ayula's fight revalidation and the Surrak entered-Bear filter. The subtype line is replaced, not added to; the +3/+3 applies after it by layer order and both expire in cleanup (CR 514.2). The once-each-turn limit reuses the restriction `Rootwalla` already prints - no new activation-limit machinery.
- 6 slice-26 tests (196 focused across 15 suites), two mutation REDs regenerated in byte mode with a byte-for-byte restore proof, and a real four-seat/40-life 40-card package journey: terminal `Win{P3}`, 1366 actions / 34 turns / 3613 events, digest `86bbd60b…`, 9 casts, 14 type-change events, separate release-CLI replay matching. Slice-25 replay still `4d6ea58e`.

### Historical increment: slice 25 — Exuberant Wolfbear (independently approved increment)

- Exact IKO151 printing `8c8ff0da-fbe3-4c95-bc39-c975c55b6aea`, oracle `40f5f8b9-5c2c-41d4-8b83-55a2e51ced64`, retrieved `2026-10-27T09:19:21Z`; exact response and stamp retained in `slice25/source-raw.json` / `source-retrieval.json`, printing fixture `fixtures/starter-corpus/slice25-sourced.json`. Full optional attack trigger: target Human you control has its base power/toughness set to the Wolfbear's effective P/T until end of turn.
- Reusable layer7b `SetBasePowerToughness`, applied in timestamp order BEFORE counters and layer7c modifiers. Source is read at resolution; a first-departure `source_pt_lki` snapshot preserves actual last-known effective P/T when it leaves, without later same-ID incarnations overwriting it. New serialized fields skip None; no new pending kind or AI policy change.
- Native added three GREEN-strengthening controls after writer code823680d: post-trigger real Giant Growth responses distinguish current-vs-trigger-time P/T; source7/7 plus target's own counter+pump yields11/11 and cleanup restores3/2; pumped source then real Unsummon uses7/7LKI; actual multiple-Human target offers reject nonHuman/opponent/wrongseat submissions without state/digest changes. Earlier writer test name overclaimed a forged-target assertion; native renamed it and supplied the actual test. Initial missing-card RED preceded registration; no blanket strictTDD claim.
- Native100focusedtests pass (7slice25,10slice24 plus affected suites), fmt/workspaceClippy-Dwarnings/releaseCLI all exit0. Native rebuilt/reran exact4seat40life40cardpackage: terminalP3,1760actions/41turns/4471events,8Wolfbearcasts/9attacktriggers/9base-PT-setevents; digest `4d6ea58ee735c74a89e142eca90656dbf22faeaf49c2de0fb33c0ec9952c5d52`, separateCLIreplay matches, freshsave roundtrips, exact snapshot SHA256 `1bd229439656be174278cd958eb7083ec495c43be3eefa8a17d4ee6fff881a0d`. Evidence-only masked offered-action demonstrator, NOT productAI/strength or a desired100cardcertificate. Slice24 replay still a41bbfd2.
- SAME original-history independent reviewer **APPROVED_INCREMENT at ea10693**, independently ran7focusedtests and release replay, inspected actual delta and native evidence. Report `evidence/issue17/slice25/REVIEW25-ORIGINAL-HISTORY.md`. Native authored test/docs-only follow-up ea10693; writer823680d contains engine changes. No blocking finding. Final exact-head CI: **10/10 completed SUCCESS**, every check keyed to `ea106932d649da6876004fdf5c1a2ce6e441d19c`; all strict required contexts pass with Actions app15368. No inherited-green claim.
- At that head: 142catalog/459union=130supported+329missing; per-list55/0/51/48/23/52/52/53; only1of8certified, Ayula23missing (current totals are in Current state above). Full#17UNFINISHED. No reachable mid-optional save/reload, dedicated target departure/recast or source-recast-overrides-LKI test is claimed; those paths are code-inspected. No full workspace rerun in this increment; exact-headCI owns that gate.

### Historical increment: slice24 — Surrak and Goreclaw (approved3b06249, 10/10exact-headCI PASS)

The publication-era pending statements below are superseded by the exact3b06249 original-history approval and10/10CI receipt in `evidence/issue17/SLICE24-FINAL-HUMAN-CHECKPOINT.md`.

- Exact MOM337 printing `9c10934d-9016-43c4-a7ab-56cc7d8f671f`, retrieved `2026-10-27T08:44:35.433520+00:00`, fixture `fixtures/starter-corpus/slice24-sourced.json`. Full printed trample, static trample to other controlled creatures, and another own nontoken creature entry grants that entered permanent a +1/+1 counter and haste until end of turn. Non-target captured identity, not a target prompt.
- Reusable `OtherControlledCreaturesHaveTrample`, `ContinuousKind::GrantsHaste`, and serde-skipped `affected` identity on trigger/stack entries. Haste affects attack legality, tap-cost checks and the actual offered creature-mana-source list.
- First cut `c457e49` was NOT accepted: native actual REDs proved a hasty Werebear missing from legal offers, and departure silently deleting the non-target trigger. Repair `c2d9b35` fixes offers and invalidates the affected binding while retaining the mandatory trigger; uses captured card identity for source-left resolution. Failed evidence retained, assertions not weakened.
- Native focused checks: 93 passed (including slice24 10); writer's fmt, workspace warnings-denied Clippy and release CLI gates exit0. Native independently rebuilt/reran real four-seat/40-life/40-card exact package: terminal P3, 1602 actions, 4127 replay events, digest `a41bbfd2760034f791a3616bd5d6faa0fd79ad8e9cee300eb69cc38c5a1ec631`; 6 actual Surrak casts, 11 counter and haste events; separate CLI replay matches and fresh save roundtrips. Evidence-only masked demonstrator with Jasmine Boreal in four command zones, NOT product AI, strength evidence or a desired 100-card starter certificate.
- Native per-list audit: 141 catalog /459 union =129 supported+330 missing; Ayula24, still1/8 certified. Desired snapshots and certificate statuses unchanged. Midtrigger save test is sandbox/direct fixture, not a reachable action-history claim. Full original #17 remains unfinished. Candidate `3b06249` published; independent original-history review and exact-head CI are pending, not claimed green.

### Historical increment: slice23 — Goreclaw, Terror of Qal Sisma (cost reduction + granted trample)

- Card: **Goreclaw, Terror of Qal Sisma**, `{3}{G}` Legendary Creature — Bear 4/3. Scryfall **CMM 293**, immutable `ee47f23a-3ba9-4615-b170-c89d8ab99d78`, oracle `befb211f-37ca-4083-98d4-9ff1f28be3f2`, retrieved `2026-10-27T07:38:22Z`; raw collection bytes `evidence/issue17/slice23/raw/cards-collection.json`, normalized record `fixtures/starter-corpus/slice23-sourced.json`, catalog text byte-equal to that printing.
- Printed behavior: "Creature spells you cast with power 4 or greater cost {2} less to cast." / "Whenever Goreclaw attacks, each creature you control with power 4 or greater gets +1/+1 and gains trample until end of turn."
- New reusable surface: `StaticAbility::CreatureSpellsYouCastWithPowerCostLess` applied inside `effective_cost` after commander tax — **generic part only, never below zero** (CR 601.2f, 118.7a), controller's own creature spells only; `Layer::Ability` (CR 613.4d) with `ContinuousKind::GrantsTrample`, read by combat damage assignment through `Game::has_trample`; `Effect::BoostAndTrampleEachCreatureYouControlWithPowerAtLeast`, whose power filter reads **current** power at resolution. Both grants are ordinary `ContinuousEffect` rows, so cleanup expiry and battlefield-departure invalidation come from the existing pipeline (the slice-21 stale-shield defect is not reproduced).
- Three discriminating **mutation REDs**, each failing exactly its target assertions and the engine restored byte-for-byte (sha256 before == after): cost reduction disabled → exactly the 2 cost tests fail; printed-power threshold ignored → exactly the qualifying-creature test fails; grant lookup dropped from `has_trample` → exactly the trample combat test fails (`evidence/issue17/slice23/red-mutation-*.log`).
- Executed: `issue17_slice23` **11/11**, plus `card_printings` 8, `catalog_pool` 5, `issue13_commander_combat` 43, `issue17_slice15_corpus` 2, `abilities` 13, `issue17_slice20` 9, `issue17_slice21` 8, `issue17_slice22` 9, `projection` 4, `timing_mana` 12, `m1_multiplayer` 28; `cargo test --workspace --all-targets --locked --no-fail-fast` exit 0 (108 suites); `fmt --check`, workspace `clippy -D warnings`, locked release build exit 0 (`evidence/issue17/slice23/native-gates-at-632c5e7.log`, `workspace-tests.log`).
- One real **4-seat / 40-life / 40-card package** journey on the exact `decks/package-slice23.txt` snapshot (`sha256 5f933f49…e71fb7`), seed 2323: terminal **Win{P3}**, **1322 actions / 32 turns / 3495 replay events**, digest `e2ccc113bceb…`; separate release-CLI replay reproduces it and a fresh save roundtrips. Goreclaw cast 5×, its attack trigger staged 12×, 45 trample grants and 45 pumps started. Evidence-only masked demonstrator, **not product AI or strength evidence**; Jasmine Boreal injected into all four command zones; **not** a 100-card certificate.
- Legacy digests unchanged: slice 17–22 demo replays still verify (`85fabffc`, `09e6bb25`, `5fd841b2`, `72cd69fb`, `0d20c759`, `17f03348`).
- Correction disclosed: my first test cut asserted Craw Wurm as a 6/6 and predicted 35 life; the card is a printed **6/4** and **both** attackers trample (3 + 5 = 8), so 32 is correct. The engine was right and the tests were corrected — no engine change was made for that.
- Not claimed: any second certified starter, playable Commander, or AI strength. The cost reduction reads printed power (no supported effect changes a card's power outside the battlefield), and `view` does not display granted keywords. **Full original #17 acceptance stays open: 1 of 8 certified, Ayula 25 names short.**

### Historical increment: slice21 — Dragon-Scarred Bear / Formidable regeneration

Full sourced DTK183 printing `689a6390-894a-4d31-87c2-f9cec3b187a7`, retrieved `2026-10-27T04:44:50.249896+00:00`, fixture `fixtures/starter-corpus/slice21-sourced.json`. Printed {1}{G} repeatable/no-tap activation checks current controlled-creature power >=8 only at activation. Typed one-shot regeneration replaces destruction, removes damage, taps and removes from combat; multiple shields/cleanup expiry/accepted-state save reload. Exile/bounce/sacrifice/zero toughness are not destruction.

- Same original-history review **CHANGES_REQUESTED at d9d2c94**: ordinary-card ObjectId persists through bounce/recast, so a shield transferred. The initial test inserted a fresh object instead of recasting the same card. Real reviewer RED retained; canonical battlefield departure now invalidates shields. Real offered same-card recast then Shock dies. **APPROVED_INCREMENT repaired f59d29b**, no blocking findings, failed verdict preserved (`slice21/REVIEW21.md`, `REVIEW21-REPAIR.md`). Dedicated queued-source-left test is not claimed; code-inspected only.

- Native own68focused tests/gates pass; repaired8new-card+1cleanup tests pass; fmt/Clippy-Dwarnings/locked release pass. Native rebuilt/reran exact4seat40life40card package: terminalP3,1906actions/45turns/4909events,digest0d20c7597183; actual25activations/25shield creations/1regeneration,Bear cast7; separate release replay+fresh save match. Prior slice20 replay still72cd69fb1143.

- Evidence-only offered-action demonstrator with public battlefield/shield instrumentation, NOT productAI/strength; JasmineBoreal injected4seats. NOT another100card certificate. AI mechanical field initialization only (`search.rs:727`, `tests/common.rs:83`), no policy change.

- Native fixed wrong Fuse citation and preferred-branch activation undercount before approval. Initial registration RED precedes source; later behavior tests mostly GREEN strengthening, not blanket strictTDD.

- Final e9b986c differs from approved f59d29b in exactly5evidence files, no product source. Its own **10/10 exact-head CI checks SUCCESS**, every check keyed to e9b986c and all strict required contexts matching the configured Actions app pass. No inherited-green claim. Checkpoint `evidence/issue17/SLICE21-FINAL-HUMAN-CHECKPOINT.md`.

- CURRENT:139catalog/459union=127supported+332missing,Ayula26,**still1of8certified**. Full original acceptance open; no merge/closure/certified-downstream unlock. Desired snapshots unchanged; bulk candidates unintegrated,0release-enabled/0certified.

### Historical increment: slice20 — Vivien's Grizzly / private top-card look

Full sourced `{3}{G}` activated ability, not an approximation: privately look at the top card; optionally reveal/take a creature or planeswalker, otherwise put it on the bottom. Eligible cards offer take/bottom; ineligible cards still show their identity only to the owner and require a mandatory bottom acknowledgement. Empty library is not a draw/loss. Anchored `Pending::TopCardChoice` preserves mid-resolution save/load; spell-container validation permits multiple abilities sharing one source while retaining malformed-container refusal.

- Printing WAR182, immutable id `37f2571b-6756-462c-9e71-3121fa458160`, retrieved `2026-10-27T03:25:27.464708+00:00`, `fixtures/starter-corpus/slice20-sourced.json`; native live readback matches.

- Native125focused tests pass (nine new-card tests plus independent two-activation probe and affected existing suites), fmt/Clippy-Dwarnings/locked release CLI pass. Native rebuilt/reran the exact4seat/40life/40card package: terminalP3,2064actions/46turns/5281events, digest `72cd69fb1143...`, Vivien cast6/activated22/take5/voluntarybottom5/mandatorybottom16; separate releaseCLI replay and fresh-save roundtrip match.

- Evidence-only masked demonstrator, NOT product AI/strength, Jasmine Boreal injected eachseat; package is NOT another100-card certificate. Planeswalker branch code-inspected only (none inpool).

- Firstcut `cb7a18a` NOT accepted: native proved legal double-activation save refusal (retained RED101), and found controller's ineligible-card look lost. Writer repaired but omitted the engine hunk from its repair commit; native caught and committed that exact tested hunk before publication. Existing tests strengthened rather than waived.

- Initial original-history approval at f4a02a2 is **SUPERSEDED**: native full-projection noninterference RED proved the public pending wording leaked private top-card eligibility (creature vs land), despite hiding card identities. Repair41d55ee filters TopCardChoice details to its owner; permanent unequal-hidden-state test compares entire opposing PlayerViews and masked digests. SAME original reviewer re-adjudicated the repaired delta: **APPROVED_INCREMENT at41d55ee**,14focused tests pass, unchanged release replay. Report `evidence/issue17/slice20/REVIEW20-PRIVACY.md`; no blocking finding remains. Native independently reran10changed tests/fmt/Clippy/locked release/replay. Finalcdfc32e differs from repaired approval only in3evidence paths. Finalcdfc32e has **10/10 completed SUCCESS exact-head CI checks**, all bound to that SHA; all four strict required contexts match their configured GitHub Actions app and pass. This is increment evidence, NOT eight-list acceptance.

- Scope-only AI compatibility: slice20 `crates/ai/tests/common.rs` pending exhaustiveness arm; slice19 `crates/ai/src/search.rs:724` clone / `crates/ai/tests/common.rs:80` Default initialization; **no policy change**.

- Authoritative checkpoint `evidence/issue17/SLICE20-FINAL-HUMAN-CHECKPOINT.md`. Original eight-list acceptance remains UNFINISHED:1of8certified,334missing union names,Ayula28. No merge/closure/certified-downstream unlock. Three bulk lanes remain processed/candidate evidence only, zero release-enabled/certified cards from that fanout.

### Slice 22 — Dancing from Dark to Dawn (cast trigger sized by mana value)

- Card: **Dancing from Dark to Dawn**, `{3}{G}{G}` Enchantment, Scryfall **HOB 123** (immutable `550cd0b6-ca61-4db7-9d20-0b68c48066f9`, oracle `5ba482e9-fbb0-4d9f-a3a9-414892bcdfed`, retrieved `2026-10-27T06:31:42Z`), recorded in `fixtures/starter-corpus/slice22-sourced.json`; the catalog text is byte-equal to that printing.
- Printed behavior: "Whenever you cast a creature spell, put X +1/+1 counters on target creature you control, where X is that spell's mana value." / "Landfall — Whenever a land you control enters, create a 2/2 green Bear creature token."
- New reusable surface: `TriggerEvent::CreatureSpellCast` (deliberately separate from the instant/sorcery `SpellCast` wording — a noncreature permanent spell fires neither), `CardDef::mana_value()` (CR 202.3 from the printed cost), the triggering spell's mana value carried on `TriggerRef::spell_mana_value` and `StackItem::spell_mana_value` (both serde-skipped at zero, so every recorded digest is unchanged), and `Effect::PutPlusOneCountersOnTargetForTriggerSpellManaValue`. The landfall half reuses `Effect::CreateBearTokens(1)`.
- **Real defect found and repaired inside the slice:** `apply_cast` granted priority unconditionally after casting, so a cast trigger waiting for a target left a pending choice alongside a priority holder and the session validator refused to load the state. The caster now receives priority only when no choice is pending (CR 603.3d). Retained RED: `evidence/issue17/slice22/focused-2.log`.
- Genuine REDs: card unsupported (3 failing tests), the save-validator refusal, and a mana-value mutation that fails exactly the three size assertions while the discriminations stay green.
- Executed: 8 slice-22 tests (target choice, 2- and 6-mana sizing, opponent's cast ignored, noncreature permanent ignored, landfall token, opponent's land ignored, save/reload while targeting, mana-value unit matrix); `card_printings` 8, `catalog_pool` 5, `issue13_commander_combat` 43, `issue17_slice15_corpus` 2, `abilities` 13, `issue17_slice20` 9, `issue17_slice21` 4, `projection` 12, `timing_mana` 28, `m1_multiplayer` — all pass; `cargo test --workspace --all-targets --locked --no-fail-fast` exit 0; `fmt --check`, workspace `clippy -D warnings` and locked release build exit 0 (`evidence/issue17/slice22/native-gates.log` records every step's own exit code).
- One real **4-seat / 40-life / 40-card package** journey on the exact `decks/package-slice22.txt` snapshot (`sha256 aede2055…6c8b81`), seed 2222: terminal **Win{P3}**, **1500 actions / 33 turns / 3891 replay events**, digest `17f03348d81c…`; the replay verifies and a fresh save roundtrips. Dancing cast 6×, 27 triggered abilities staged (11 cast triggers, 16 landfall), 11 counter events, 16 Bear tokens. The driver is an evidence-only masked cast/land-preferring demonstrator, **not product AI or strength evidence**, and injects Jasmine Boreal into all four command zones. The package is **not** another desired 100-card starter certificate.
- Not claimed: any second certified starter, playable Commander, AI strength, or a release ETA. Full original #17 acceptance stays open.

### Historical increment: slice18 — Ayula, Queen Among Bears

The target starter's commander is now implemented from its printed text (mh1 155, `https://api.scryfall.com/cards/c046dfb0-9ca3-4219-a6b0-d7503bc2fbd0`, retrieved `2026-10-26T23:40:57Z`, recorded in `fixtures/starter-corpus/slice18-sourced.json`):

> Whenever another Bear you control enters, choose one — • Put two +1/+1 counters on target Bear. • Target Bear you control fights target creature you don't control.

- Triggers only for **another Bear its controller controls** — Bear tokens count, Ayula's own entry and an opponent's Bear entry do not.

- The mode is chosen **before** targets (CR 603.3c / 700.2b) and a mode with no legal full target set is not offered.

- Counters mode targets **any Bear**, including an opponent's; the printed first mode is not narrowed to "you control".

- Fight mode needs a Bear you control plus a creature you don't control, is revalidated at resolution (CR 701.14b), and deals both damage instances through the existing damage pipeline as one noncombat pair (CR 701.14a/d).

- Registered as a supported commander in the command zone/catalog/setup/cast path; ordinary-library refusal follows the existing supported-commander policy.

Coverage added: mode/target offers, token-vs-opposing trigger distinction, unavailable fight mode, fight target departure via a real Unsummon, fight damage prevention through Mending Hands with the other direction still lethal, source-left resolution, queued-batch preservation, pending save/reload (mode chosen and half-answered two-target fight) through the real session loader, wrong-seat refusal, and forged submissions refused with state/decision/digest unchanged.

### Verification and review

- **CHANGES_REQUESTED then APPROVED_INCREMENT on the true original reviewer history** (`evidence/issue17/review-sessions/…f17d.jsonl`). At `2785c2c` the code was fine but three record statements were stale (`122`/`337`, "Ayula's printed counter/tribal text is not implemented", Ayula listed as unsupported); the reviewer's own audit also settled the union arithmetic at **459 / 123 / 336** (an integration-owner parse that had filtered basics produced a spurious 454/118 and is recorded as such). Corrections were applied as a docs-only commit `60e0bd9` authored by the integration owner, then re-adjudicated **APPROVED_INCREMENT** at that exact head.

- Native prepublication inspection of the first cut caught a genuine regression: the new `apply_trigger_choice` consumed the pending unconditionally, so ordinary optional triggers were silently dropped. Proved by running the then-committed head — `cargo test -p aperture-core --test abilities` → `optional_trigger_is_declined_or_used_by_its_controller` FAILED (12 passed / 1 failed) — and repaired on the same writer history, with a second real RED for the queued-trigger loss and a citation fix (Fight is CR 701.14a/b in the pinned rules text, not 701.12a).

- Independently re-run at the code head: `command_zone_setup` 44, `abilities` 13, `rules_conformance` 5, `issue17_slice17` 6, `card_printings` 8, `catalog_pool` 5, `issue17_slice15_corpus` 2, `issue13_commander_combat` 43 → **126 focused tests, 0 failed**; `fmt --check` and workspace `clippy -D warnings` exit 0; release CLI built.

- One real **4-seat / 40-life / 40-card package** journey on the exact `decks/package-slice18.txt` snapshot (`sha256 40028e6e…4490ec`), seed 1616: terminal **Win{P2}**, **1961 actions / 43 turns / 5028 events**, digest `09e6bb25c505…`; a separate `replay` run with the release CLI reproduces that digest. Ayula cast 3×, counters chosen 5×, fight chosen 4×, 95 fight-damage events.

- The driver is an evidence-only masked card-preferring demonstrator, **not product AI or strength evidence**, and it injects **Ayula into all four command zones**. The package is **not** another desired 100-card starter certificate.

Current checkpoint: `evidence/issue17/SLICE18-HUMAN-CHECKPOINT.md`; review: `evidence/issue17/slice18/REVIEW18-ORIGINAL-HISTORY.md`.

### Exclusions / original work remains

Equipment, Auras, soulbond, hideaway, mounts/saddle, Ring temptation, tutors, reanimation, X-spells, other modal text and most tribal payoffs (including Beorn the Fierce, Druid's Familiar, Lightning Greaves, Stuffed Bear) remain outside the supported corpus as recorded per list. Arbitrary Archidekt compatibility remains an explicit non-goal. No eight-list acceptance, merge, issue closure or certified-evaluation/soak unlock is requested.

## Historical slice19 — Spectral Bears and the pool's first attack trigger

One sourced card, and it is a subsystem rather than a stat line. **Spectral Bears**

`{1}{G}` Creature — Bear Spirit 3/3, printing **me1 131**, sourced in

`fixtures/starter-corpus/slice19-sourced.json` (oracle id, printing id, retrieval

stamp, and oracle text byte-equal to the catalog string).

Printed: *"Whenever this creature attacks, if defending player controls no black

nontoken permanents, it doesn't untap during your next untap step."*

New engine surface, each piece required by that text:

- **`TriggerEvent::Attacks`** — the pool's first attack trigger, queued and staged in

  the declare attackers step (CR 508.1) through the existing trigger queue/stack.

  Five other Ayula-list cards and much of the corpus need attack triggers.

- **`TriggerCondition` / `ability::trigger_condition`** — a printed intervening-if

  (CR 603.4) evaluated **before** the trigger is queued (a false clause never reaches

  the stack; a `TriggerRemoved` event records why) and **re-checked at resolution**.

  Held per card so all 31 pre-existing trigger literals are untouched.

- **`CardDef::is_black`** — printed color from the mana cost (CR 202.2a) or a token's

  own colors.

- **`Effect::SkipNextUntapStep` + `GameState::skip_next_untap`** — one-shot

  "doesn't untap during your next untap step" (CR 502.3): consumed by that seat's

  next untap step only, no summoning sickness side effect, other seats' markers kept,

  markers for departed objects dropped. Serde-skipped when empty, so every previously

  recorded digest is unchanged (verified by re-running the slice-17 and slice-18 demo

  replays).

### Verification

- 6 focused tests (`crates/core/tests/issue17_slice19.rs`): registration against the

  printed text; trigger firing against a black-free defender with the attacker held

  tapped through exactly one untap step while a bystander untaps and then untaps

  normally; a black **nontoken** permanent stopping the trigger; a black **token** not

  stopping it; a nonattacking bear never marked; and the resolution-time re-check

  removing the ability when the defending board changes while the trigger is on the

  stack.

- Three mutation REDs (each fails exactly the intended test and nothing else):

  intervening-if forced true, untap step ignoring the marker, resolution re-check

  disabled.

- `fmt`, workspace `clippy -D warnings`, locked release build, and

  `cargo test --workspace --all-targets --locked --no-fail-fast` → **103 suites,

  736 passed, 0 failed** (`evidence/issue17/slice19/gates.log`, produced by

  `run-gates.sh` with per-step exit codes).

- Real **4-seat / 40-life / 40-card package** journey on `decks/package-slice19.txt`

  (seed 1919): terminal **Win{P3}**, **1773 actions / 42 turns / 4709 events**, digest

  `5fd841b2…a19f5d`, separately reproduced by the release CLI; Spectral Bears cast 5×,

  its trigger staged 14×, the untap restriction observed in 13 real untap steps.

### Review

- Round 1, independent, execution lens → **CHANGES_REQUESTED** with six real findings:

  the reviewed head did not compile its test suite (`catalog_pool` had no arm for the

  new effect), `card_printings` had no slice-19 record, two previously green suites

  (`issue13_commander_combat`, `issue17_slice15_corpus`) pinned the old pool/gap

  counts, `README` still said 135, the resolution re-check had no test, and the gate

  log was not honest evidence.

- All six fixed; round 2 re-review → **APPROVE**, 147 focused tests it ran itself, no

  source change after the approved delta.

- Records: `evidence/issue17/slice19/REVIEW19.md`,

  `evidence/issue17/slice19/REVIEW19-REVALIDATION.md`,

  checkpoint `evidence/issue17/SLICE19-HUMAN-CHECKPOINT.md`.

### Corpus state and what remains

**136 catalog cards; union 459 = 124 supported + 335 missing**; per-list missing

`55 / 0 / 51 / 48 / 29 / 52 / 52 / 53`, recomputed this slice by

`evidence/issue17/slice19/corpus-audit.py`. Desired snapshots unchanged. **Still 1 of

8 desired starters certified.** Ayula has 29 unsupported names left; equipment,

soulbond, hideaway, saddled mounts, Ring temptation, keyword grants and library search

remain unimplemented. The journey package is explicitly **not** a 100-card

certificate, and no eight-list acceptance, merge, closure or certified-downstream

unlock is requested.










