# Issue 17 — slice 14 checkpoint (dynamic mana / threshold)

## Result

Four sourced cards added on the SAME draft PR46 / branch `aperture/issue-17`:

* **Arcane Signet** `{2}` — `{T}: Add one mana of any color in your commander's
  color identity.`
* **Fellwar Stone** `{2}` — `{T}: Add one mana of any color that a land an
  opponent controls could produce.`
* **Commander's Sphere** `{3}` — `{T}: Add one mana of any color in your
  commander's color identity.` / `Sacrifice this artifact: Draw a card.`
* **Werebear** `{1}{G}` 1/1 Human Bear Druid — `{T}: Add {G}.` plus threshold
  +3/+3 while you have seven or more cards in your graveyard.

New behaviour: explicit `ActivateColorManaAbility` legal actions with a stable
WUBRG order, refusal of forged/wrong-state colors before any tap, commander
color-identity mana, opponent-land-derived mana, creature tap mana with
summoning sickness, and a real layer 7c threshold static.

Two genuine product defects in the first cut were caught by native inspection
and repaired with RED/GREEN in the same history:

1. tapped dynamic artifacts were offered colors that `validate` then refused;
2. a ready Werebear was never offered because `mana_sources` omitted creature
   mana (direct-submit tests alone passed).

Corpus: **126 catalog; 459 union names = 114 supported + 345 missing; 1 of 8
desired starters certified (jasmine-vanilla-stompy)**. All eight desired
100-card snapshots are unchanged. Full #17 is unfinished.

## Reviews

- `slice14/REVIEW14.{md,json}` — APPROVED_INCREMENT at `472aeba`, zero blocking
  findings; nonblocking dead-field/docs items fixed in the following commit.
- `slice14/REVIEW14-CLEANUP.md` — APPROVED_INCREMENT at the cleanup head;
  behaviour-neutral field removal confirmed, no engine/refusal change.

## Evidence

- Focused: `issue17_slice14` 10, `issue17_slice13` 4, `issue17_tokens` 10,
  `catalog_pool` 5, `card_printings` 7, `issue13_commander_combat` 43 (each run
  by the writer/reviewer in this slice); native re-run of slice14 10, slice13 4,
  tokens 10, catalog 5.
- `fmt`/`clippy -D warnings`/locked release CLI clean (`slice14/clippy.log`,
  `slice14/release-build.log`).
- Evidence-only masked legal-action driver (not product AI): exact four seats,
  40 life, `decks/package-slice14.txt`, seed 1414, terminal P0, 2371 actions /
  42 turns / 6665 events, digest `028cc2e33168…`. Casts, read back from
  `slice14/demo-journey.json`: Arcane Signet 7, Fellwar Stone 7, Commander's
  Sphere 7, Werebear 18, Grizzly Bears 7, **Krenko, Mob Boss 4**. Mana
  activations: Signet 51 / Fellwar 39 / Sphere 7 / Werebear 82 (plus Forest 21,
  Mountain 110). Sphere sacrifice-draws: 7. Separate release-CLI replay and a
  fresh save reload both reproduce the digest (`slice14/native-demo-replay.log`).
- **The journey driver injects Krenko, Mob Boss as the commander of all four
  seats** (`slice14/journey-driver/src/main.rs:15`; the replay's top-level
  `commanders` records four Krenko entries, and the four Krenko casts in the
  table above are that injection). This matters for reading the result: Arcane
  Signet and Commander's Sphere derive their mana from the *commander's* color
  identity, so every identity-based activation in this journey is red. The
  package fixture itself carries no commander and the driver is an evidence-only
  demonstrator, not product AI; nothing here is a claim about a desired 100-card
  starter, and no certification rests on this journey.
- Default product heuristic trace retained unchanged at seed 1401 (terminal,
  digest `fdc9b089f8f9…`): 17 Werebear casts, zero artifact casts/activations.
  The product heuristic still does not cast artifacts or auto-activate nonland
  mana; that limitation is unchanged and no AI code was touched.
- Immutable printing readback for all four cards (`native-source-readback.json`,
  `native-sphere-readback.json`, fetched during this slice).

## Honest gaps

- No additional desired 100-card starter is certified. Nearest gaps after this
  slice: Ayula 39, Krenko 48, Talrand 51, Elas 52, Tatyova 52, Zada 53, Isamaru 55.
- Legend rule, equipment/Aura, tutors, reanimation, multi-mode/choose-one text
  and most tribal payoffs remain unimplemented.
- Friendly Teddy and Aetherize remain excluded with recorded reasons.
- No arbitrary Archidekt compatibility, full Commander, playing-strength or v1
  claim.

## Resume

Same card/PR46/branch; saved writer and reviewer histories are preserved under
`evidence/issue17/sessions/…` and `review-sessions/…` with `slice14-backup/`
copies. Current-lane target only; no cleanup, retired builds, global provider
or model changes. No merge, master push, issue closure or #26/#32 unlock.
