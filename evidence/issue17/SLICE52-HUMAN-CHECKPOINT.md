# Slice 52 — review-335's two record items closed, and the Ruxa brief vetted anchor by anchor

**ORIGINAL #17 REMAINS UNFINISHED: 1 of 8 lists certified, catalog 161, union 459 = 148 supported +
311 missing, Ayula 5 missing. This slice adds no card and no certificate.**

Head `adeff72` on branch `aperture/issue-17` (same draft PR46). Base `6abd820`.

## 1. R335-1 — the search gate now has a receipt from the command it claims

Review 335 found that the separate `aperture-ai` search gate was published as
`cargo test -p aperture-ai --test search --locked → 9 passed` while the only committed receipt,
`slice51/gates.log:105-115`, was `--lib search` → `running 0 tests` → `EXIT=0`. The number was true
(the reviewer ran the correct command and got 9/0); the receipt was not.

Closed by execution, at both heads:

* **At the slice-51 head itself (`6abd820`)** — `slice52/search-gate-slice51-head.log` records the
  corrected command, its own output and its own exit line:

  ```
  === ai_search (corrected command: --test search) ===
  head=<6abd820…>
  command=cargo test -p aperture-ai --test search --locked
  running 9 tests
  test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 240.47s
  EXIT=0 (ai_search --test search)
  ```

  So the 9-test claim for slice 51 is now backed by the real command at the head it was made about,
  not by a later head.
* **At the slice-52 head** — `slice52/gates.log` runs `ai_search` (the true command) **first**, and
  then runs the historical `--lib search` invocation as its own step named
  `ai_search_lib_zero_match_attempt`, so the empty run stays visible and cannot be mistaken for the
  gate. `slice52/run-gates.sh` is the corrected script; `slice51/run-gates.sh` and `slice51/gates.log`
  are left unmodified as historical artifacts.
* `slice51/RESULT.json` `local_gates.separate_search_suite` now names both receipts, and
  `harness_error_disclosed` states that `gates.log:105-115` is the zero-match attempt.
  `SLICE51-HUMAN-CHECKPOINT.md` §3 states which file holds the nine-test receipt.

## 2. R335-2 — the crate-wide sweep is now recorded, not just performed

Review 333 required a sweep of the retired alone-predicate vocabulary and a statement that it was a
sweep. Slice 51 performed it but recorded only the one comment. The slice-51 checkpoint §1 R333-2 now
carries the sweep: `grep -rn` for `declaration record`, `declared_attackers`, `declared_blockers`
across `crates/`, `docs/`, `README.md`, every hit read, and the only surviving mentions are the
accurate ones — `engine.rs:2062` (the record is **not** the predicate), `engine.rs:2495`,
`engine.rs:5302-5305`, `state.rs:784-796`, `card.rs:256-260`, `docs/issue17-starter-corpus.md:754`
and its §Slice 46 entry, `README.md:112`. No code change and no re-grep was needed; review 335's own
grep agrees.

## 3. Non-blocking note recorded instead of dropped

`docs/issue17-starter-corpus.md` §"Unpaired blockers" now states that `PlayerView::unpaired_blockers`
is projected to both seats but **no UI surface renders combat lists yet** (`ui/app.rs` reads only
`legal_blockers`), so the slice-51 fix is a data-layer fact today. Recorded as a visible gap.

## 4. The Ruxa brief is now a vetted brief — four wrong or imprecise anchors corrected

`slice51/NEXT-CARDS.md` becomes the next run's premise, and review 335 had already caught one wrong
anchor in it (`describe_target` attributed to `view.rs`; it is `engine.rs:9022`). `slice52/RUXA-PREREQ-VERIFICATION.md`
re-reads **every** anchor at this head and records the corrections instead of silently rewriting them:

| Brief said | Verified at `adeff72` |
|---|---|
| `describe_target` in `view.rs` | `engine.rs:9022` |
| AI mapping `ai/src/search.rs ~760-800` | `pending_kind_name` at `ai/src/search.rs:345-365`; the enum is `view.rs:40` |
| commander prediction `state.rs ~2078` | `Pending::CommanderReplacement` validation `state.rs:2026-2050`, answer path `state.rs:1108` |
| `deal_combat_damage ~8827` | `engine.rs:8832`, entered from `advance_step`'s `Step::CombatDamage` arm `engine.rs:5955` |

The same file confirms the three clauses are still unbuilt surface, not data authoring: there is no
graveyard-to-hand effect anywhere in `crates/core/src/` (the only hand mover is `move_to_hand`,
`engine.rs:1971`), no "no abilities" anthem (only `OtherBearsYouControlGet`, `ability.rs:1275`,
computed at `engine.rs:678`), and no combat-damage choice surface at all (the blocked branch at
`engine.rs:8865-8901` assigns lethal-first and exposes no decision). **No Ruxa code was written in this
slice**, deliberately: a clause shipped without the card would be unreachable surface with no card to
exercise it, and the card may only leave `missing_cards` when all three clauses work.

## 5. Executed evidence

Filled from `slice52/gates.log` (`head=adeff72…` recorded inside the log) and `slice52/journey.log`:

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| focused 6 suites | **43 passed / 0 failed** — `issue17_slice46` 14, `issue17_slice48` 6, `issue17_slice49` 8, `starter_corpus` 8, `catalog_pool` 5, `issue17_slice15_corpus` 2 |
| `cargo test -p aperture-ai --test search --locked` | **9 passed / 0 failed** (the real command, run first) |
| `cargo test -p aperture-ai --lib search --locked` | 0 tests, exit 0 — logged as `ai_search_lib_zero_match_attempt`, explicitly *not* a gate |
| corpus audit | **AUDIT OK** 161 catalog / 459 union / 148 supported / 311 missing, per-list `[55,0,51,48,5,52,52,53]`, Ayula 5, **1 certified** |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo build --workspace --release --locked` | exit 0 |
| `cargo test --workspace --all-targets --locked` | **133 result lines / 947 passed / 0 failed / 1 ignored**, exit 0 |

Journey (`slice52/journey.log`, `head=adeff72…`): the committed four-seat 40-life witness driver rebuilt
in release with `--locked` and re-run. `Win{P1}`, **3073 steps, 54 turns, 8098 events**, digest
`da4eacd5363fc16b6242d1c52fe86c2157c556d5e66a01696a6d6ba15b995bde`, `replay_digest_matches: true`,
14 exile events; all three committed artifacts **content-identical after newline normalisation**
(committed CRLF vs fresh LF, the honest form review 333 asked for); separate release CLI replay
`replay of 3073 action(s) -> digest da4eacd5363f | events 8098`, exit 0. This is a 40-card evidence-only
package, **not** a 100-card certificate and not AI-strength evidence.

## 6. Corpus, certification and scope

* Corpus re-derived at this head: **161** catalog cards, **459** union = **148** supported + **311**
  missing, per-list `[55,0,51,48,5,52,52,53]`, Ayula **5**, **1 of 8** certified. No card was enabled,
  so no count moved.
* Scope: records, one docs paragraph and the vetted brief. No engine, AI policy, deck, import or UI
  code changed; no digest can move and none is claimed to.
* No merge, no master push, no issue closure, no downstream unlock, no new team, no global
  provider/model config change. Children `t_26a7a4ff` and `t_3c2fe51a` remain gated backlog, not
  review children.
