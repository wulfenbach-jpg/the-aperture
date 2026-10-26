# Issue21 evidence — import start flow

All commands were run in `C:/AI/aperture-worktrees/issue-21` on branch `aperture/issue-21`.

## 1. Focused certification tests (offline)

```
cargo test -p aperture-import --locked --test start_flow
```

14 passed. Covers: a certified command seat (list + commander), the format and support
axes disagreeing on purpose, an unsupported card refused by name and reason, the practice
profile refusing a commander deck instead of dropping the commander, a wrong deck size
refused with the numbers, provider color identity normalized and checked (legal, illegal,
colorless-known, and unknown-data cases), commander eligibility judged from the provider
type line (`legal` for a legendary creature, `not_certified` otherwise), a commander
outside the pool refused, multiple commanders refused without inflating the main-deck
counts, the URL guard running before any network work, and sideboard/excluded cards never
reaching the seat list.

## 2. UI start-flow tests

```
cargo test -p aperture-ui --locked
```

The session-level journey (`crates/ui/tests/session.rs`) certifies two command seats,
starts the imported match, and checks the engine actually has one imported commander per
seat at 40 life with 40-card libraries; a refused import leaves the bundled table intact;
a stale import is discarded and its busy flag cleared. The app-level journey
(`crates/ui/src/app.rs` viewport tests) clicks `Start imported match` and
`Confirm imported match` in the real renderer, and asserts the panel shows the format
axis, the support axis, every unsupported card and the refusal before start is enabled.

## 3. Real provider deck (network)

```
APERTURE_EVIDENCE_DIR=evidence/issue21 \
APERTURE_IMPORT_CACHE=<scratch>/i21cache \
cargo test -p aperture-import --locked --test start_flow_network -- --ignored --nocapture
```

Deck: `https://archidekt.com/decks/27701327` ("Menace", public, Commander).

The recorded run is `real-deck-27701327-certification.json`; the numbers below are read
from that file, not paraphrased:

* provider response SHA-256: `60d1a8a1f05765df0e760601b13209cf216bedebb796a51214d2086f718374fe`;
* commander: `Seifer, Balamb Rival` — recorded from the provider, **unsupported** in this
  build;
* format axis: one commander (legal), singleton (legal), legendary-creature eligibility
  (legal), color identity inside `[BR]` (legal), deck size **illegal** (the deck itself was
  over the 100-card limit at capture time);
* engine support: 8 of 100 main-deck cards supported, 92 unsupported, each with its exact
  reason (not in the 31-card pool, or a multi-face card the engine does not model);
* seat fit: the bounded command-zone profile can hold the library, but start is **refused**
  because engine support fails, and the refusal states that unsupported cards are never
  substituted.

Caveat, recorded honestly: this deck belongs to a third party who was actively editing it
during development — consecutive fetches saw 99, 100 and 101 library cards and a changing
`deck_size` verdict. That is precisely why the axes are separate and the numbers are
reported per fetch instead of being folded into one verdict. The offline tests pin the
behaviour; the live test asserts the flow's invariants and records the exact run.

The provider payload itself is not committed (third-party deck content); its hash is
recorded above so the fetch can be re-verified.

## 4. Workspace gates

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo build --workspace --release --locked
```

Results are recorded in the task handoff rather than duplicated here.

## Known limitations (unchanged by this work)

* The supported pool is 31 cards, so a real 100-card Commander deck is always refused on
  engine support; that is a card-implementation gap (#17), not an import gap.
* Per-card banned/errata status from the provider is not re-certified.
* Partners, multiple commanders, the legend rule and a native 100-card Commander table are
  still out of scope; they are refused, not approximated.
