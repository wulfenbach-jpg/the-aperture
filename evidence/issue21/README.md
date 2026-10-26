# Issue21 evidence — import start flow

All commands were run in `C:/AI/aperture-worktrees/issue-21` on branch `aperture/issue-21`.

## 1. Focused certification tests (offline)

```
cargo test -p aperture-import --locked --test start_flow
```

14 + 4 passed. Covers: a certified command seat (list + commander), the format and support
axes disagreeing on purpose, an unsupported card refused by name and reason, the practice
profile refusing a commander deck instead of dropping the commander, a wrong deck size
refused with the numbers, provider color identity normalized and checked (legal, illegal,
colorless-known, and unknown-data cases), commander eligibility judged from the provider
type line (`legal` for a legendary creature, `not_certified` otherwise), a commander
outside the pool refused, multiple commanders refused without inflating the main-deck
counts, the URL guard running before any network work, and sideboard/excluded cards never
reaching the seat list.

The singleton check got its own four-case repair pass (round 2 review):

* `a_duplicated_unsupported_nonbasic_is_refused_by_the_singleton_check` — provider
  `Sol Ring x2` with a known nonbasic type line is `illegal` even though the engine cannot
  seat the card, while the support axis still names it as unsupported;
* `a_text_repeat_without_a_type_line_is_not_certified_rather_than_legal` — a text/file
  list repeats `Sol Ring x2` with no type data at all, so the answer is `not_certified`,
  never a positive `legal`;
* `the_basic_exception_and_supported_duplicates_keep_their_verdicts` — 39 provider `Basic`
  Plains stay legal, a supported `Grizzly Bears x4` stays illegal;
* `a_supported_legal_hundred_card_list_stays_startable_after_the_singleton_repair` — a
  fully supported legal 100-card list still certifies and plans 99 library cards.

The rule the check now follows: every included main-deck entry is examined regardless of
engine support; the unlimited-basics exemption rests on the provider's own `Basic` supertype
or the catalog's basic-land data, so absence of a type line becomes `not_certified` instead
of a confident verdict.

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
from that file, not paraphrased (round-2 re-capture at the singleton repair):

* provider response SHA-256: `648223ca33b004d7d5320bd65bed45bb5eeec2da3670a61c0b4ff6494a29cf6c`;
* commander: `Seifer, Balamb Rival` — recorded from the provider, **unsupported** in this
  build;
* format axis: one commander (legal), deck size `100 cards including the commander
  (99 + 1)` (legal), singleton (legal — the only repeats are 9 `Swamp` and 8 `Mountain`,
  both carrying the provider's own `Basic` supertype, so they are exempt on a recorded
  fact rather than by skipping them), legendary-creature eligibility (legal), color
  identity inside `[BR]` (legal);
* engine support: 8 of 99 main-deck cards supported, 91 unsupported, each with its exact
  reason (not in the 31-card pool, or a multi-face card the engine does not model);
* seat fit: the bounded command-zone profile can hold the 99-card library, but start is
  **refused** because engine support fails and the commander is not in the pool, and the
  refusal states that unsupported cards are never substituted.

Caveat, recorded honestly: this deck belongs to a third party who was actively editing it
during development — consecutive fetches saw 99, 100 and 101 library cards, different
supported-card counts, and a changing `deck_size` verdict. That is precisely why the axes
are separate and the numbers are reported per fetch instead of being folded into one
verdict. The offline tests pin the behaviour; the live test asserts the flow's invariants
and records the exact run.

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
