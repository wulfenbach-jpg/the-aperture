# Issue 17 — slice 15 checkpoint (costs paid with cards; first enchantments)

Full issue17 is **UNFINISHED**. This is a bounded implementation increment on the
same draft PR46 / branch `aperture/issue-17`, not eight certified starter lists.
No merge, master push, issue closure or successor activation was performed.

## 1. Required record corrections from run 240 (R240-1 … R240-4) — applied

| finding | correction | where |
|---|---|---|
| R240-1 | Ayula's missing count is **39**, not 42 | `docs/issue17-starter-corpus.md` |
| R240-2 | slice-14 checkpoint now discloses that its driver injects **Krenko, Mob Boss** as commander on all four seats, and the cast table is reconciled with `slice14/demo-journey.json` (Signet 7, Fellwar 7, Sphere 7, Werebear 18, Grizzly Bears 7, Krenko 4) | `evidence/issue17/SLICE14-HUMAN-CHECKPOINT.md` |
| R240-3 | every landwalk **enforcement site** now cites CR **702.14c** (`card.rs` keeps 702.14b only for the definitional "landwalk is an evasion ability" sentence); slice-11 checkpoint says **6** focused tests and cites 702.14c | `engine.rs`, `tokens.rs`, `issue17_slice11.rs`, `SLICE11-HUMAN-CHECKPOINT.md` |
| R240-4 | the slice-14 fixture note says **four records**, matching its contents | `fixtures/starter-corpus/slice14-sourced.json` |

Commit `a6c980c` is that correction-only change; no behaviour touched.

## 2. What slice 15 adds (sourced, printed text only)

Three Ayula-list cards whose cost is paid **with cards**, all producing 2/2 green
Bear tokens:

| card | printing | printed text | engine |
|---|---|---|---|
| **Ayula's Influence** | mh1 156 | `Discard a land card: Create a 2/2 green Bear creature token.` | first card paid from **hand**; the activator names the land |
| **Bearscape** | ody 229 | `{1}{G}, Exile two cards from your graveyard: Create a 2/2 green Bear creature token.` | first cost paid from the **graveyard** (two named cards), offered only when two exist |
| **Mother Bear** | mh1 171 | `{3}{G}{G}, Exile this card from your graveyard: Create two 2/2 green Bear creature tokens. Activate only as a sorcery.` | first ability **activated from the graveyard**, exiling itself as the cost, and first "activate only as a sorcery" restriction (CR 307.5) |

Records (oracle text, mana cost, type line, P/T, printing URL, retrieval stamp)
are in `fixtures/starter-corpus/slice15-sourced.json`, copied verbatim from the
Scryfall fetch in `evidence/issue17/slice15/sources-ayula-raw.json` (30 of 39
missing Ayula names were also fetched on the way; `Rampaging Yao Guai` hit a
rate limit and is **not** claimed as sourced).

New reusable engine surface (no approximation of the printed cost):

* `Kind::Enchantment` — the pool's first noncreature permanent cards; they cast
  and resolve through the same permanent-spell path as creatures/artifacts.
* `Action::ActivateCostedAbility { …, cost_cards }` — the activator names the
  cards paying the cost. `Action::ActivateAbility` keeps its old shape, so every
  earlier replay and every existing test literal still deserializes/compiles.
* `AbilityCost::{discard_land, exile_graveyard_cards,
  exile_source_from_graveyard}` and `ActivatedAbility::sorcery_timing`.
* `CardId::BearToken` (2/2 green Bear, engine-only, never importable) and
  `Effect::CreateBearTokens`.
* Legal actions enumerate every legal cost-card combination (`collect_combinations`)
  and every offered action is validated before it is offered.

## 3. Corpus movement (native audit, `evidence/issue17/slice15/corpus-audit.py`)

| metric | before | after |
|---|---|---|
| catalog | 126 | **129** |
| eight-list union names | 459 | 459 (unchanged) |
| supported union names | 114 | **117** |
| missing union names | 345 | **342** |
| `ayula-bear-counters` missing | 39 | **36** |
| certified starters | 1 | **1** |

New guard: `crates/core/tests/issue17_slice15_corpus.rs` now **recomputes** every
list's `missing_cards` from the compiled pool and requires equality with the
manifest, and pins each desired library at 99 cards. Nothing did that before, so
a stale `missing_cards` entry could have survived a slice.

## 4. Verification actually run

* `crates/core/tests/issue17_slice15.rs` — **6** behaviour tests: printed text of
  all three cards and the Bear token; Influence offers one option per land in
  hand, refuses a creature card with no state change, discards exactly the named
  land and then makes one Bear; Bearscape isn't offered with a single graveyard
  card, exiles exactly the named pair, makes one Bear, and refuses a duplicated
  cost card; Mother Bear isn't offered from the battlefield, is refused during an
  opponent's turn, is offered in its owner's main phase, exiles itself and makes
  two Bears; an enchantment casts from hand and stays on the battlefield; a Bear
  token entering is a real creature for `Soul Warden`.
* `crates/core/tests/issue17_slice15_corpus.rs` — 2 tests (manifest recompute).
* `crates/core/tests/card_printings.rs` — new
  `every_slice15_card_prints_the_cost_its_ability_actually_pays`: the typed
  ability cost must equal the printed cost, and only Mother Bear may carry the
  sorcery-timing restriction.
* **Four genuine REDs** (`evidence/issue17/slice15/red_cycle.py`, logs beside it):
  removing the Bear-token effect fails 4 tests; removing the sorcery-timing guard
  fails exactly the Mother Bear test; removing cost-card validation fails the
  Influence and Bearscape tests; removing the graveyard offer fails exactly the
  Mother Bear test. The suite is green again after each restore.
* `cargo test --workspace --all-targets --locked` → see
  `evidence/issue17/slice15/workspace-tests.log`.
* `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D
  warnings`, `cargo build --workspace --release --locked` → exit 0
  (`slice15/{fmt,clippy,release-build}.log`).
* **Real executable four-seat journey** — `evidence/issue17/slice15/journey-driver`
  (evidence-only masked demonstrator, **not** product AI), exact
  `decks/package-slice15.txt` in all four seats, 40 life, seed 1515: terminal,
  winner P3, **1529 actions / 36 turns / 4107 replay events**, digest
  `0bf033d9bb37…`. All three new cards were cast (Influence 6, Bearscape 8,
  Mother Bear 4), card-costed abilities activated (Influence 6, Bearscape 7),
  Mother Bear's graveyard ability activated twice, and **17 Bear tokens** were
  created. A separate release-CLI replay of the recorded file reproduces
  `0bf033d9bb37` with the same 4107 events.
  The driver injects **Jasmine Boreal** as the commander of all four seats; that
  is stated in `demo-journey.json` and matters only for legality, not for any
  claim here.
* This is a **40-card package, not a desired 100-card starter**. No starter is
  certified by this slice.

## 5. Honest gaps

* No additional desired starter is certified. Nearest gaps: Ayula 36, Krenko 48,
  Talrand 51, Elas 52, Tatyova 52, Zada 53, Isamaru 55.
* The product heuristic does not activate card-costed abilities and casts no
  enchantments; no AI file was touched.
* `Kind::Enchantment` has no continuous-effect support: an enchantment whose
  printed text is a static or triggered ability is still refused at import.
* Legend rule, equipment/Aura, tutors, reanimation, multi-mode text, "choose
  two", and most tribal payoffs remain unimplemented.
* Only the three named cards were sourced for this slice; the other 36 Ayula
  names (and the rest of the 342 missing union names) remain unsupported.
* No arbitrary Archidekt compatibility, full Commander, playing-strength or v1
  claim.
