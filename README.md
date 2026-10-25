# THE APERTURE — a practice slice of Magic: The Gathering

**practice slice / supported card pool / headless 2–4 seats / native 2–4-seat practice / named 20-life and bounded 40-life profiles / NOT Commander**

An original Rust rules engine, a headless CLI, and a native desktop card table.

**FOSS:** original source is licensed **MIT OR Apache-2.0**. See [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), and the [separate third-party content boundaries](ASSETS.md).

**Working toward graphical Commander v1:** [public GitHub/Pages kanban mirror](https://wulfenbach-jpg.github.io/the-aperture/) (not the internal Hermes board), [roadmap and release gates](docs/ROADMAP.md), [contributing](CONTRIBUTING.md), and [security reporting](SECURITY.md). Planned support is not current support.

This is a deliberately narrow lab: a small, explicitly implemented card pool with
real rules scheduling (priority, stack, state-based actions, combat), honest deck
import, deterministic replay, and trace-backed coaching. It is not a full Magic
implementation and it says so everywhere it matters.

## What actually works

* **Rules engine** (`crates/core`): typed state, zones and object ids, mana costs
  and payment, London mulligan, priority and stack resolution (consecutive passes
  resolve exactly one stack item), sorcery/instant timing, targets with
  revalidation on resolution, counterspell, scry/surveil pending choices, cleanup
  discard, damage removal, state-based actions, bounded combat with attack/block
  declarations and simultaneous damage, empty-library loss, game win/loss.
* **Supported branch-candidate pool (31 real cards)**: the 16-card practice
  corpus (Island, Mountain, Forest, Grizzly Bears, Runeclaw Bear, Centaur
  Courser, Craw Wurm, Hill Giant, Giant Growth, Shock, Lightning Strike,
  Counterspell, Divination, Opt, Consider, and **Crack the Earth**) plus the
  five sourced command-zone practice cards **Isamaru, Hound of Konda**,
  **Plains**, **Unsummon**, **Time Ebb**, and **Swords to Plowshares**, plus
  the sourced full-combat-prevention instant **Fog** (`{G}`, Oracle: “Prevent
  all combat damage that would be dealt this turn.”), plus the six typed
  activated/triggered-ability practice cards **Prodigal Sorcerer**,
  **Rootwalla**, **Brindle Boar**, **Elvish Visionary**, **Festering Goblin**
  and **Oculus** (see `decks/abilities-practice-a.txt`/`-b.txt`), plus the
  issue-15 effect cards **Mending Hands** (one-shot prevention shield),
  **Angrath's Marauders** (damage-doubling replacement effect) and **Wight of
  Precinct Six** (characteristic-defining power/toughness, layer 7a).
  Isamaru is commander-only and is refused in every ordinary starting library
  because the legend rule is not implemented. Crack the Earth is described here: (`{R}`, Sorcery — Arcane; Oracle: “Each player sacrifices a permanent of their choice.”) from the official Gatherer page recorded in [docs/rules-reference.md](docs/rules-reference.md). The two bundled 40-card practice decks are unchanged and do not contain Crack the Earth. No cards outside this list are supported. This is a branch candidate, not an accepted M1 release.
* **Two bundled 40-card practice decks** (`decks/temur-beats.txt`,
  `decks/izzet-control.txt`).
* **Headless CLI** (`aperture-cli`): `selfplay`, `replay` (verify), `harness`,
  `import`, `import-txt`, `commander-check`, `pool`, `decks`, `coach`,
  `decider-health`.
* **2–4-seat headless candidate (M1, partial and unreleased):** ordered seats,
  turn wraparound/skipping after elimination, all-live priority pass tracking,
  masked per-seat projections, bounded one-defender combat declarations,
  replay v5/save v8 and a `harness` command that runs
  paired 2/3/4-seat seeds and verifies every replay digest. Terminal, capped and
  failed runs are reported separately. The same ordered 2–4-seat engine is now
  exposed by the native table (see the UI bullet below), but this is still
  **not** Commander support. This branch implements
  CR 103.5 starting-player mulligan rounds and one bounded, real fixed-set
  public sacrifice producer (Crack the Earth); independent review and exact-head
  CI remain delivery gates, not an automatic M1 acceptance. Generic APNAP hidden-zone choices,
  dynamic/restarting APNAP and Arcane/splice interactions remain unsupported.
* **Offline Commander profile analysis (#11, analysis only):**
  `aperture-cli commander-check --deck <deck-snapshot.json> --cards
  <cards-snapshot.json> [--json] [--out <report.json>]` validates the
  100-card/singleton profile (with the Basic-Land-supertype exemption), one
  front-face legendary-creature commander, color identity, the basic-land-type
  restriction, recorded ban status, and engine/card support **separately**.
  It is deterministic and offline; snapshots bind their exact embedded bytes
  with SHA-256 provenance. Unknown, incomplete, ambiguous, non-normal or
  unsupported metadata is `not_certified`, never guessed legal; format
  `legal` never means playable. All 2+ commander variants and every non-normal
  layout are refused as `not_certified`. See
  [docs/commander-legality.md](docs/commander-legality.md).
* **Versioned persistence:** replay v5 embeds ordered seat deck snapshots, the
  explicit named profile (`practice-20`, `command-zone-practice-20` or
  `commander-combat-practice-40`), the explicit commander setup (empty for
  ordinary practice), and the recorded first player; save v8 embeds the game,
  replay, seat snapshots, AI settings, the Commander-combat damage ledger and
  digest, including the mulligan round and sacrifice queue. Older replay/save versions are rejected explicitly rather
  than silently reinterpreted, and a recorded `rules_revision` must match this
  build before a replay runs. The pinned rules revision is unchanged.
* **Native desktop UI** (`aperture`): dark charcoal/brass 2–4-seat card table
  with compact 2x2 seat panels, all public battlefields, command zones, life,
  counters, status, stack view, own hand, legal-action buttons, coaching panel
  (category-badged: `[RULE]`, `[SIMPLIFICATION]`, `[STRATEGY]` — see
  `docs/issue19-trace-coach.md`; includes pass-round, response-window and APNAP
  records),
  viewer-masked log, pending-choice controls, sandbox tools, per-seat deck
  import, save/load, text zoom and keyboard navigation. Every model, network and
  file operation runs on a background worker with a visible busy state; the
  frame callback never blocks, and a worker result is applied only while its
  session generation, engine revision, decision serial and seat all still match.
  The human is P0 and every other seat can be driven by the deterministic
  heuristic AI with no model service. The setup controls choose 2/3/4 seats and
  either ordinary 20-life practice or the explicit bounded 40-life
  one-Isamaru-per-seat practice profile; **New game** retains that selection.
  Ordinary defaults use the alternating compiled Temur/Izzet lists. The bounded
  command default uses the compiled command-zone practice library
  (`decks/command-zone-practice.txt`: Plains/Forest/Island plus the supported
  command-zone cards) for every seat, so the supplied white Isamaru is actually
  castable. Explicit `--deck-a`/`--deck-b` and imported lists still replace the
  defaults exactly (or are refused) and are never silently overwritten.
  The renderer uses the engine's masked `PlayerView` for card data, so opponent
  hands and libraries are counts only.
  An accepted action invalidates any in-flight load, so a delayed file read can
  never overwrite the board the player just changed.
* **Per-seat import / play** (UI): each seat has its own Archidekt URL box, a
  pasted-text fallback and a text-file fallback. Each import is certified on its
  own (name, supported/total counts, practice-profile line, refusal reason);
  **Start imported match (P0 vs P1)** is enabled only when *both* seats are
  certified, applies the exact planned `DeckList` with no substitutions, and
  starts a fresh game/replay that embeds both lists as snapshots. An
  explicitly requested `--deck-a`/`--deck-b` file that is missing or invalid is
  refused loudly — the bundled practice deck is never silently substituted.
* **Deterministic replay**: seed + deck snapshots + accepted actions reproduce the
  same canonical SHA-256 state digest. Save/load round-trips preserve the pending
  choice, the RNG position and the AI mode, and **Set branch / Return to branch**
  rewinds to a labelled study point (the hidden-information boundary is stated in
  the message).
* **Bounded Archidekt import**: `https://archidekt.com/decks/<digits>` (plus the
  canonical `/decks/<digits>/<human_slug>` form, whose slug is ignored) with a
  host allowlist, numeric-id request construction, timeout, size cap, one
  allowlisted redirect maximum, on-disk caching, and strict refusal of anything
  outside the supported pool. Normal single-face cards (provider `layout:
  "normal"`, empty `faces`) are recognized from the top-level `oracleCard`
  metadata; multi-face layouts are refused as not modelled; a quantity that is
  missing, non-integer, zero or above 255 is a clear error rather than a silent
  clamp or skip. A text-list fallback is included.

## Quick start (Windows)

```
launch.cmd            :: builds if needed and starts the native UI
```

or, by hand:

```
cargo run --release -p aperture-ui          # native card table
cargo run --release -p aperture-cli -- pool # what is implemented
cargo run --release -p aperture-cli -- selfplay --seed 1 --verbose
cargo run --release -p aperture-cli -- import --url https://archidekt.com/decks/26206384
```

The CLI and UI both load `decks/*.txt`; the same lists are compiled in, so the
binaries also work from another working directory.

## Packaged, reproducible build

```
scripts\package.cmd
```

Builds a version/commit-stamped, unsigned practice bundle under `dist/` —
`aperture.exe`, `aperture-cli.exe`, the bundled decks, the packaged scope README
and launcher — with a SHA-256 manifest and a `verify-package.cmd` that
re-derives every hash and launches the packaged executable in its smoke mode.
Text payloads and the manifest are deterministic, and the release executables
are byte-identical across repeated builds of the same commit and toolchain.
See [docs/packaging.md](docs/packaging.md) for the exact contract, what is and
is not claimed, and the CI artifact job.

## Verification

```
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo run --release -p aperture-ui -- --smoke evidence/screenshots/ui.png
```

`evidence/` holds the TDD log, self-play reports, the live import report, and the
verification log.

## The optional local model

The baseline AI is a deterministic heuristic. A local Decider-2B service can be
opted into (`--decider`, or the UI checkbox) through its documented typed readout
API (`POST /v1/systemone` with `state`/`questions`/`model` on
`http://127.0.0.1:8765`). It is off by default, receives only the masked
projection of the acting seat, and any timeout, HTTP error, unparsable answer or
**response-indicated truncation** (`usage.truncated`, `state_tokens_dropped > 0`
or a non-empty `truncated_questions`) falls back to the heuristic with a visible
reason — a choice the model made without seeing the whole projection is never
used. A returned confidence is option preference concentration, never a win
probability. See HANDOFF.md for the tested/untested status of that path.

## Command-zone practice (explicit profile, not Commander)

`aperture-cli selfplay --command-zone-practice --commander 0:"Isamaru, Hound of Konda" ...`
adds a bounded, explicitly named practice profile: one supported commander per
seat starts in that seat's command zone, command-zone casts pay `{2}` per previous
command-zone cast (CR 903.8), a commander entering a graveyard/exile gets its
owner's one-time CR 903.9a return choice, and a commander moving to hand/library
is suspended for the owner's CR 903.9b replacement choice. This named profile
stays at 20 life. It is **not** Commander legality or full Commander: Isamaru is
the only supported commander, there is no color-identity/100-card profile, no
commander damage, no legend rule, no partner/multi-commander; the native table
supports loading/saving the two named command profiles in addition to starting
the bounded 40-life table, but this is still practice, not Commander.

A separate explicit **bounded Commander-combat practice** profile
(`--commander-combat-practice`, serialized as `commander-combat-practice-40`)
starts at 40 life and keeps the same one-Isamaru-per-seat command-zone behavior.
The per-commander combat-damage ledger and the CR 903.10a 21-damage loss are
implemented for this explicit profile (stable `CommanderId` + recipient,
combat-only credit). It is still **not the Commander format** (no 100-card
construction, partners, legend rule or full elimination/APNAP cleanup). The
native setup offers this as the explicit **bounded 40 life** option with one
Isamaru per seat; its games save/load natively. It remains headless-capable and
is not the Commander format.
Isamaru is refused in every ordinary starting
library. The command-zone sources are Isamaru, Plains, Unsummon, Time Ebb and
Swords to Plowshares; **Fog** adds bounded full combat-damage prevention (this
turn, all players and permanents) with typed prevented events. Partial
prevention, damage redirection and replacement effects are not modelled.

## What is explicitly NOT implemented

Full Commander legality and gameplay — the bounded command-zone and
Commander-combat practice profiles above are not Commander: color identity,
100-card singleton construction, the legend rule and partners/multi-commander
are absent. The native table exposes these explicit profiles for local practice,
which does not make them Commander. The Commander-combat profile implements 40
starting life, the per-commander combat-damage ledger and the CR 903.10a 21-damage
loss and full combat-damage prevention for one turn (Fog); partial prevention,
redirection and replacement effects are not implemented, so the profile is not
Commander gameplay. The offline deck-profile analyzer above is
analysis only;
networked multiplayer is absent (the native table is local 2–4 seats), as are
generic hidden-zone APNAP or
dynamic/restarting APNAP choices beyond the single public fixed-set Crack the
Earth sacrifice candidate, triggered abilities, activated abilities other
than basic-land mana abilities, static abilities beyond the two supported forms,
replacement effects beyond damage doubling, redirection,
tokens, planeswalkers, counters as rules objects (the sandbox counter display is
storage only), milling/tutoring/libraries as
targets, multi-face cards (transform/split/meld/modal), sideboard/companion play,
layers 4–6/7b/7d (counters stay display-only), Arcane/splice interactions, and any
card outside this branch candidate's 31-card pool.
Unsupported decks are refused at import rather than silently substituted.
The 40-card practice file loader also refuses `Commander:`/`Companion:`/
`Sideboard:` text instead of flattening it into a practice deck. `pool` prints
the implemented list.

**Combat damage assignment is narrowed.** CR 510.1c lets the attacking player
divide a blocked creature's damage among its blockers as they choose; this slice
assigns it automatically in lethal-first order and exposes no assignment choice.
The engine comment, the coaching panel and README all label this as a bounded
simplification rather than the rule (`crates/core/tests/combat_teaching.rs`).
