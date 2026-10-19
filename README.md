# THE APERTURE — a practice slice of Magic: The Gathering

**practice slice / supported card pool / 1v1 / 20 life / NOT Commander**

An original Rust rules engine, a headless CLI, and a native desktop card table.
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
* **Supported pool (15 real cards)**: Island, Mountain, Forest, Grizzly Bears,
  Runeclaw Bear, Centaur Courser, Craw Wurm, Hill Giant, Giant Growth, Shock,
  Lightning Strike, Counterspell, Divination, Opt, Consider.
* **Two bundled 40-card practice decks** (`decks/temur-beats.txt`,
  `decks/izzet-control.txt`).
* **Headless CLI** (`aperture-cli`): `selfplay`, `replay` (verify), `import`,
  `import-txt`, `pool`, `decks`, `coach`, `decider-health`.
* **Native desktop UI** (`aperture`): dark charcoal/brass card table with both
  battlefields, stack view, own hand, legal-action buttons, coaching panel,
  viewer-masked log, pending-choice controls, sandbox tools, Archidekt import,
  save/load, and deterministic digests.
* **Deterministic replay**: seed + deck snapshots + accepted actions reproduce the
  same canonical SHA-256 state digest.
* **Bounded Archidekt import**: `https://archidekt.com/decks/<digits>` only, with
  a host allowlist, numeric-id request construction, timeout, size cap, one
  allowlisted redirect maximum, on-disk caching, and strict refusal of anything
  outside the supported pool. A text-list fallback is included.

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
projection of the acting seat, and any timeout, HTTP error or unparsable answer
falls back to the heuristic with a visible reason. A returned confidence is
option preference concentration, never a win probability. See HANDOFF.md for the
tested/untested status of that path.

## What is explicitly NOT implemented

Commander (any form), multiplayer, triggered abilities, activated abilities other
than basic-land mana abilities, static abilities, replacement/prevention effects,
tokens, planeswalkers, counters as rules objects, milling/tutoring/libraries as
targets, multi-face cards (transform/split/meld/modal), sideboard/companion play,
layers beyond a single until-end-of-turn pump, and any card outside the pool of
15. Unsupported decks are refused at import rather than silently substituted.
`pool` prints the same list.
