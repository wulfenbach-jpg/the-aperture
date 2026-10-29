# Review 278 — slices 24–25, contract lens (round: original #17 gate, not slice re-audit)

Reviewed head: `ea106932d649da6876004fdf5c1a2ce6e441d19c` (PR46, local = remote, draft/OPEN).
Approved increments 1–23 were not reopened. This review checks the two newest increments
against the ORIGINAL eight-list acceptance contract and re-tests the delivered claims by running
them, not by reading the handoff.

## What I ran (native, this session)

| Check | Command | Result |
| --- | --- | --- |
| Focused suites | `cargo test -p aperture-core --test issue17_slice24 --test issue17_slice25 --test abilities --test timing_mana --locked` | 10 + 7 + 13 + 12 = **42 passed, 0 failed** |
| Format | `cargo fmt --all --check` | exit 0 |
| Whitespace | `git diff --check -- crates` | exit 0 |
| Full workspace | `cargo test --workspace --all-targets --locked` | **110 suites, 792 passed, 0 failed**, `workspace_exit=0` |
| Journey (rebuilt, independent output dir) | own copy of the slice25 driver, same source, writing to `evidence/issue17/review278/` | **1760 actions / 41 turns / 4471 events**, seed 2323, terminal `Win{P3}`, digest `4d6ea58ee735…` — byte-identical to the handoff's digest, snapshot sha `1bd22943…` reproduces |
| Separate replay | `aperture-cli replay --file evidence/issue17/review278/demo-replay.json` | `replayed digest 4d6ea58e… matches recorded | events 4471` |
| Exact-head CI | `capture-ci.py ea10693… evidence/issue17/review278` | **10/10 completed SUCCESS, every head_sha = ea10693**, required contexts (ubuntu/windows/macos rust + board and roadmap, app 15368) all PASS, `green: true` |
| Corpus audit | own script over `catalog.rs` + `manifest.json` (asserts 99+1 per list, `missing_cards` equality) | **142 catalog / 459 union = 130 supported + 329 missing**; per list 55/0/51/48/23/52/52/53; certified **1 of 8** |

The journey is a 40-card package with Jasmine Boreal in all four command zones, run by an
evidence-only masked demonstrator. It is **not** a product AI, not strength evidence, and **not**
a 100-card starter certificate. The driver emits one unused-import warning (pre-advised, non-blocking).

## Contract mapping (original #17 body)

1. **Eight explicitly supported 100-card lists — NOT MET.** 1 of 8 (`jasmine-vanilla-stompy`) is
   certified. The seven others remain `planned` with 55/51/48/23/52/52/53 missing names.
2. **Every included card and required interaction implemented + conformance covered — NOT MET.**
   130 of 459 union names are supported; 329 are not.
3. **Four-seat games on exact deck snapshots, never bundled replacements — PARTIALLY MET for the
   tested package.** This run proves snapshot-exact four-seat play + replay parity on
   `decks/package-slice25.txt`; it is not one of the eight desired 100-card lists.
4. **Document per-card/interaction exclusions; arbitrary Archidekt remains a non-goal — MET as a
   standing disclosure** (`docs/issue17-starter-corpus.md`, README unsupported-surface paragraph),
   though two of its lines are now stale (below).

## Delta findings

No blocking defect in slices 24–25. Both cards are implemented from their recorded printed text
(`fixtures/starter-corpus/slice2{4,5}-sourced.json`, Scryfall MOM 337 / IKO 151 with retrieval
stamps) and the mechanics I re-exercised behave as claimed: static trample reaching real combat
and real mana offers for a hasty creature, the mandatory "enters" trigger surviving its
recipient's departure (resolving with no effect rather than being deleted), the layer 7b base
setter ordering before counters and 7c modifiers, and last-known P/T after the source is bounced.

Non-blocking, recorded rather than blocking:

- `evidence/issue17/slice25-journey-driver/src/main.rs:4` still says the driver prefers to
  "cast Surrak first, then the biggest creature spell" and cites trample/cost reduction — that is
  the slice-24 text; this deck casts Wolfbear first and has no Surrak or Goreclaw.
- `README.md:303` still lists "layers 4–6/7b/7d" as unimplemented after layer 7b and layer 6
  grants landed, and `docs/issue17-starter-corpus.md` item 4 still says "haste … remain
  unimplemented" after slice 24. Prose only; the engine behaviour is the implemented one.
- Layer 7b LKI capture in `detach_object` is keyed on `CardId::ExuberantWolfbear`; a future
  source-stat card will need its own branch instead of a shared rule. Maintenance note only.

These are documentation/maintenance debt on already-approved behaviour, not acceptance gates.

## Verdict

**Approved increment; the original card is returned to implementation, not completed.**
Nothing is merged, master is untouched, no issue is closed, and no downstream certified work
(`t_26a7a4ff`, `t_3c2fe51a`) is unlocked.
