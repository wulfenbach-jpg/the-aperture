# Slice63 — three demanded Krenko cards enabled; independent review requested

Same task/writer history/branch `aperture/issue-17`/draft PR46. Base `a4ac11c`; **tested published code+driver head `150435b94f2e5282eaf6c5dbc782ffe1c148680e`**. The successor publishing these records is evidence-only and claims no own CI. Original #17 remains **2 of8 certified**, not accepted. No merge/master push/closure/dependent unlock.

## Product and complete sourced text
- **Goblin Warchief**: Goblin spells cost one less generic; controlled Goblins have haste INCLUDING itself.
- **Goblin King**: OTHER Goblins across ALL controllers get +1/+1 and mountainwalk; a second King affects the first. Defending player's Mountain controls the restriction.
- **Dragonlord's Servant**: Dragon spells cost one less generic, not Goblin spells.
- **Volcanic Dragon**: sourced full Flying/haste companion, expressly authorized as an out-of-list real positive control. Does not increase demanded-union support.
- Shared subtype reduction/haste/anthem/landwalk machinery and Ruxa no-abilities recipient semantics. Both Krenko gap fields updated together: **46→43**. Fresh native audit **172catalog/459union=158supported+301missing**, per-list `[55, 0, 51, 43, 0, 52, 52, 53]`, exactly2certified.

- **Dragonlord's Servant** (TDC 210, printing `be6fa0dc-74e7-4fad-9bdd-8e986612577a`, retrieved `2026-10-29T09:40:02Z`).
- **Goblin King** (10E 207, printing `4b266935-25ea-49c5-a1da-57c22a4362fd`, retrieved `2026-10-29T09:40:01Z`).
- **Goblin Warchief** (DOM 130, printing `5bac033c-dc4e-40a0-b103-4892e4b50249`, retrieved `2026-10-29T09:40:01Z`).
- **Volcanic Dragon** (CMR 207, printing `46419d29-21a1-4753-a2f0-1d0d996ec54e`, retrieved `2026-10-29T09:54:11Z`).

Full provider responses and text: `fixtures/starter-corpus/slice63-sourced.json` and `slice63/sources/`. No card implemented from a paraphrase.

## Honest prepublication defects and authorship
Native is a coauthor, NOT the independent reviewer. Native affected check caught an actual regression in local initial c58213a: removing an outer controller restriction for global King's grant accidentally let an opposing Chieftain count as granting haste for Ruxa. Existing slice62 assertion failed6/1, retained in `slice63-native/affected-precheck.log`; same writer repaired the per-ability controller branch at77865fb. Assertion unchanged. No bad candidate was pushed.

Native also required a real castable Dragon, rather than accepting an unreachable positive cost path. New tests prove one/two Servants, colored-pip refusal, opposing-Servant control, resolved flying/haste, and Warchief reducing Krenko's generic commander tax. That last test is an explicit taxed command-zone fixture, not a full-game recast journey. The initial claim 'commander tax inapplicable' is superseded.

Standalone-driver fmt initially failed; native applied formatting-only150435b, with no product-source change. Final native workspace/driver fmt0. Initial receipts remain historical; TDD.md's8 refers to the initial phase, final slice63 suite10. No retroactive test-first claim for follow-up controls.

## Executed gates and actual journey
- Writer focused gates **139passed/0failed across12 result lines**: corelib14; slice62=7; slice63=10; Ruxa15; reduction11; printings22; catalog5; commander43; slice15=2; replay6; import3+1. fmt, locked workspace Clippy-Dwarnings and locked workspace release all0, aggregate0 (`slice63/repair-final-native/gates.log`). No fresh local whole-workspace test total claimed.
- New test delta **11** (10 new core tests +1 printing test), counted base→head, separate from suite totals. Native affected suites **17/0**, overlap writer, NOT additive.
- Native rebuilt/reran actual four-seat40-life exact40-card package journey, seed2323, terminalP3, **954actions/23turns/2455events**, digest`43bbcce4a480a38d810525bf8fa30961616e94c5ee2b1645f7abb231ce448848`, snapshot`b2539164ff5335e672549608b0d17a51a714137981ff99eb6819f2d01f68fa6a`. Warchief6casts/King6/Servant3/Dragon1;14attacks/44combat-damage events. All3artifacts byte-identical to repaired writer's.
- Genuine non-sandbox save/load and **separate shipped release CLI replay**, matches recorded/2455events/exit0. Native trace audit additionally verifies **every954accepted action exactly belonged to that seat's projected offers** and actual Dragon payment was **4mana with2Servants vs printed6** (step864, P2); `slice63-native/offer-audit.json` + executable source.
- This is a projected-offer demonstrator with Jasmine setup, NOT100-card Krenko certification/productAI/strength/GUI evidence. No new canonical state fields or AI/UI/import code.

## Exact-head CI — waited before evidence push
`slice63-native/exact-head-ci-150435b.json`: **10/10completed success**, all keyed`150435b94f2e5282eaf6c5dbc782ffe1c148680e`, both workflows completed: push**40854628109** attempt1, PR merge-ref**40854642665** attempt1. Both finished BEFORE the evidence publication. Evidence tip claims no ownCI.

## Remaining gate
Independent concise PRACTICAL DELTA review of a4ac11c..150435b plus evidence successor, reusing approved1–62. Focus global-vs-controlled grants/Ruxa, effective subtype and defending-land restriction, generic-only reductions/colored pips/Krenko tax, four complete printings, actual offered-action/payment/replay evidence. Then SAME writer/history/card/branch/PR46 continues demanded Krenko packages; **3/8 only on a genuine exact100-card next-list certificate**. Six lists remain planned,301union names missing. Inspected scheduled evaluation t_26a7a4ff and unassigned soak t_3c2fe51a; both left gated, neither an increment-review child. No completion/merge/masterpush/closure/unlock/newteam/globalconfig.
