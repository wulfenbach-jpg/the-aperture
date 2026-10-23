# THE APERTURE — a practice slice of Magic: The Gathering

**practice slice / supported card pool / headless 2–4 seats / native 2-seat practice / 20 life / NOT Commander**

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
* **Supported branch-candidate pool (21 real cards)**: the 16-card practice
  corpus (Island, Mountain, Forest, Grizzly Bears, Runeclaw Bear, Centaur
  Courser, Craw Wurm, Hill Giant, Giant Growth, Shock, Lightning Strike,
  Counterspell, Divination, Opt, Consider, and **Crack the Earth**) plus the
  five sourced command-zone practice cards **Isamaru, Hound of Konda**,
  **Plains**, **Unsummon**, **Time Ebb**, and **Swords to Plowshares**.
  Isamaru is commander-only and is refused in every ordinary starting library
  because the legend rule is not implemented. Crack the Earth is described here: (`{R}`, Sorcery — Arcane; Oracle: “Each player sacrifices a permanent of their choice.”) from the official Gatherer page recorded in [docs/rules-reference.md](docs/rules-reference.md). The two bundled 40-card practice decks are unchanged and do not contain Crack the Earth. No other card or ability is added. This is a branch candidate, not an accepted M1 release.
* **Two bundled 40-card practice decks** (`decks/temur-beats.txt`,
  `decks/izzet-control.txt`).
* **Headless CLI** (`aperture-cli`): `selfplay`, `replay` (verify), `harness`,
  `import`, `import-txt`, `commander-check`, `pool`, `decks`, `coach`,
  `decider-health`.
* **2–4-seat headless candidate (M1, partial and unreleased):** ordered seats,
  turn wraparound/skipping after elimination, all-live priority pass tracking,
  masked per-seat projections, bounded one-defender combat declarations,
  replay v4/save v5 and a `harness` command that runs
  paired 2/3/4-seat seeds and verifies every replay digest. Terminal, capped and
  failed runs are reported separately. This is **not** Commander support: the
  native table remains a two-seat practice surface. This branch implements
  CR 103.5 starting-player mulligan rounds and one bounded, real fixed-set
  public sacrifice producer (Crack the Earth); independent review and exact-head
  CI remain delivery gates, not an automatic M1 acceptance. Generic APNAP hidden-zone choices,
  dynamic/restarting APNAP, native 3/4-seat tables and Arcane/splice
  interactions remain unsupported.
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
* **Versioned persistence:** replay v4 embeds ordered seat deck snapshots, the
  explicit commander setup (empty for ordinary practice), and the
  recorded first player; save v4 embeds the game, replay, seat snapshots, AI
  settings and digest, including the mulligan round and sacrifice queue.
  Older replay/save versions are rejected explicitly rather than silently
  reinterpreted, and a recorded `rules_revision` must match this build before a
  replay runs. The pinned rules revision is unchanged.
* **Native desktop UI** (`aperture`): dark charcoal/brass card table with both
  battlefields, stack view, own hand, legal-action buttons, coaching panel,
  viewer-masked log, pending-choice controls, sandbox tools, **two-seat deck
  import**, save/load, and deterministic digests. Every model, network and file
  operation runs on a background worker with a visible busy state; the frame
  callback never blocks, and a worker result is applied only while its session
  generation, engine revision, decision serial and seat all still match. The
  native table intentionally remains the two-seat practice surface; 3/4-seat play
  is headless/CLI only in this candidate.
  An accepted action invalidates any in-flight load, so a delayed file read can
  never overwrite the board the player just changed.
* **Two-seat import / play** (UI): each seat has its own Archidekt URL box, a
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
is suspended for the owner's CR 903.9b replacement choice. Still 20 life. It is
**not** Commander legality or full Commander: Isamaru is the only supported
commander, there is no color-identity/100-card profile, no 40 life, no commander
damage, no legend rule, no partner/multi-commander, and the native table refuses
command-zone-profile saves. Isamaru is refused in every ordinary starting
library. The five sourced cards are Isamaru, Plains, Unsummon, Time Ebb and
Swords to Plowshares.

## What is explicitly NOT implemented

Full Commander legality and gameplay — the bounded command-zone practice
profile above is not Commander: 40 life, 21 commander damage, color identity,
100-card singleton construction, the legend rule, partners/multi-commander and
native command-zone play are absent; the offline deck-profile analyzer above is
analysis only;
Commander multiplayer are absent — networked multiplayer or native multiplayer beyond the
two-seat table (3/4 seats are headless/CLI only), generic hidden-zone APNAP or
dynamic/restarting APNAP choices beyond the single public fixed-set Crack the
Earth sacrifice candidate, triggered abilities, activated abilities other
than basic-land mana abilities, static abilities, replacement/prevention effects,
tokens, planeswalkers, counters as rules objects, milling/tutoring/libraries as
targets, multi-face cards (transform/split/meld/modal), sideboard/companion play,
layers beyond a single until-end-of-turn pump, Arcane/splice interactions, and any
card outside this branch candidate's 21-card pool.
Unsupported decks are refused at import rather than silently substituted.
The 40-card practice file loader also refuses `Commander:`/`Companion:`/
`Sideboard:` text instead of flattening it into a practice deck. `pool` prints
the implemented list.

**Combat damage assignment is narrowed.** CR 510.1c lets the attacking player
divide a blocked creature's damage among its blockers as they choose; this slice
assigns it automatically in lethal-first order and exposes no assignment choice.
The engine comment, the coaching panel and README all label this as a bounded
simplification rather than the rule (`crates/core/tests/combat_teaching.rs`).
