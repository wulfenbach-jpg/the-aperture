# Issue17 — slice42 checkpoint

- Branch `aperture/issue-17`, base `728e776` (independently approved increment, review 313).
- No push in the writer's run; the integration owner publishes and captures exact-head CI.
- Full #17 unfinished: **1 of 8 certified**. Slice 42 moves Ayula from 12 to 10 missing names.

## What landed

Shared Equipment package, both cards sourced from Scryfall with recorded printing/oracle ids and
retrieval stamps (`slice42/sources/`, chained into `fixtures/starter-corpus/slice42-sourced.json`
and `crates/core/tests/card_printings.rs`):

* **Lightning Greaves** `{2}` — "Equipped creature has haste and shroud. … Equip {0}"
* **Swiftfoot Boots** `{2}` — "Equipped creature has hexproof and haste. … Equip {1}"

New reusable surfaces: ordinary Artifact — Equipment permanent cast path; a typed attachment
relation (`ContinuousKind::Equipped`) created by `Effect::AttachSourceToTarget`; printed equip as a
targeted activated ability at sorcery timing for the source's controller; live `haste`, `shroud`
and `hexproof` queries funnelled through the canonical `legal_targets` filter; attachment cleanup on
battlefield departure, loss of creature type, and ordinary-card re-incarnation.

## Authorship and state (read this before trusting any number)

* Product code and tests 1–9 in `issue17_slice42.rs`: the resumed OMP Go/DeepSeek writer.
* The animated-permanent defect, its RED, its fix, and `issue17_slice42_equip_controls.rs`:
  the **integration owner** in this same run. That is implementation, not review.
* **No independent verdict exists for slice 42.** Nothing here is approved.

## Native defect found and fixed after the writer's first commit

`recompute_layers` emptied `state.effects` *before* asking `is_creature` about the attached
target, so a legally animated permanent (Beorn's Hospitality) looked noncreature and its Boots
were dropped the moment an attachment was evaluated. RED, executed and retained:
`slice42/native-red-animation.log` (`exit 101`, "equipping an animated enchantment must retain its
attachment"). Fix: collect the invalid attachment ids first, then retain — types are evaluated with
animation effects still present. Journey digest is unchanged by the fix (`1750a81e…f551`).

## Gates actually executed

`slice42/native-verify.log` (exit receipts in-file):

| step | exit |
|---|---|
| issue17_slice42_equip_controls | 0 (3 passed) |
| issue17_slice42 | 0 (9 passed) |
| issue17_slice33 | 0 (15 passed) |
| card_printings | 0 (15 passed) |
| catalog_pool | 0 (5 passed) |
| starter_corpus | 0 (8 passed) |
| issue17_slice15_corpus | 0 (2 passed) |
| issue13_commander_combat | 0 (43 passed) |
| replay_digest | 0 (6 passed) |
| clippy `--workspace --all-targets -- -D warnings` | 0 |
| release CLI build (`--locked`) | 0 |
| workspace `cargo test --workspace --locked` | 0 — **995 passed / 0 failed** |

`fmt` exited 1 in that first pass **only** on the coordinator's new test file; it was formatted and
`slice42/native-verify-final.log` records `fmt_exit=0`, `clippy_exit=0`,
`slice42_equip_controls_exit=0`, `slice42_exit=0`. Both receipts are committed as-is, including the
fmt failure, rather than rewritten.

## Journey (evidence, not certification)

`decks/package-slice42.txt`, 4 seats, 40 life, seed 2323, commanders injected by the driver,
terminal `Win{P2}`, 1565 decisions, 36 turns, 3949 replay events, both Equipment cast (7 + 7),
17 equip activations, 17 successful attachments, 48 combat-damage events, digest
`1750a81eb981f0112472e8c6908e0942ea3942a6d6f13afc722002199158f551`, snapshot SHA256
`05cb383d0882cad0b572155a2a1be3c2abc8500f085b944afd82f24ca01e6dcb`, own replay + fresh
`SessionSave` reload agree, separate release-CLI replay matches. This is a 40-card evidence
package driven by a masked legal-action demonstrator — **not** product AI, **not** a 100-card
certificate, **not** strength evidence.

## Known gaps (unchanged)

* No resolution-time protection probe: no supported instant attaches Equipment, so a creature
  cannot gain shroud while a spell is already on the stack. Documented, not worked around.
* The journey does not prove a *same-turn equipped haste attack* as an event; haste is proven at
  focused level (summoning-sick Prodigal Sorcerer `{T}` activation offer).
* No generic Equipment, Aura, ward or protection support is claimed beyond these two cards.
* Exact-head CI is owned by the integration owner's publication step.

## Corpus counts

Catalog 155, union 459 = 143 supported + 316 missing, Ayula 10 missing, 1 of 8 lists certified
(`slice42/corpus-audit.log`, fresh slice-local audit; historical audits untouched).

## Next

Independent review of the published head, then the remaining Ayula names:
Dúnedain Rangers, Evercoat Ursine, Lumra Bellow of the Woods, Owlbear Cub, Rampaging Yao Guai,
Ruxa Patient Professor, Spirit of the Aldergard, Thijarian Witness, Titania's Command,
Ursine Monstrosity. Ayula-to-zero is the only step that moves certification from 1 of 8 to 2 of 8.
