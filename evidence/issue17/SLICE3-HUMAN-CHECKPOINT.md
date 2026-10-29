# Issue17 — run209 slice3 continuation checkpoint

Full issue17 is UNFINISHED. Draft PR46 head **9cecd62ddb56d44e6853f9f9098fd3a473635b66** is independently approved only as a bounded increment; 10/10 exact-head CI checks (both Windows package jobs included, plus configured required checks) pass. No merge, master push, issue closure or successor activation.

## Usable additions this slice
- **Tatyova, Benthic Druid** (commander-only) and **Jaddi Offshoot**: sourced landfall controller-only trigger (gain 1 life then draw / gain 1 life), one canonical catalog entry with real defender enforced in `legal_attackers`, forged direct-declaration refusal with canonical digest equality, and source-left resolution control. Approved at 3661353.
- **Cross-list package**: sourced **Swamp** (black basic, closes the Elas land gap), **Lightning Bolt**, **Forest Bear**, **Striped Bears** (ETB draw), and **Guttersnipe** (spell-cast trigger deals 2 damage to each live opponent, controller excluded, source-left through the real replacement/prevention pipeline using last-known source controller). Repair retained the source controller without serialized/allocator changes. Approved at 9cecd62 after two documentation/manifest truthfulness findings were fixed.

Exact supported 40-card **package** fixtures (never substitutes for the desired 100-card starters):
- `decks/package-tatyova-landfall.txt` — 4 seats, 40 life, seed 21, terminal P0, 2993 actions / 65 turns, digest `b31adfd97c31…`, replay and save parity.
- `decks/package-cross-corpus.txt` — 4 seats, 20 life, seed 5, terminal P2, 1112 actions / 25 turns, digest `4572cd11e893…`, 8 real Guttersnipe triggers, replay parity.

## Exact corpus state (native script `evidence/issue17/slice3-native-corpus.py`)
catalog **93**; eight-list union **459** exact names; **81 supported / 378 missing**; manifest `missing_cards` matches the same union; per list: Isamaru 57, Jasmine 0, Talrand 54, Krenko 51, Ayula 50, Elas 58, Tatyova 56, Zada 57. Desired 100-card list contents unchanged; **1 implemented / 0 certified**; all seven other starters remain planned/unsupported. Legend rule, broader landfall/ramp, other tokens/keywords/counters and tribal/sacrifice payoffs remain unimplemented.

## Evidence
- Review1 (Tatyova increment): `evidence/issue17/slice3-review-1.json/.md`, approved head 3661353.
- Review2 (cross-package): `evidence/issue17/slice3-review-2.json/.md`, request_changes on 0d8a1ee (retained).
- Review3 (repair adjudication): `evidence/issue17/slice3-review-3.json/.md`, approved 9cecd62.
- Gates/focused/game logs: `slice3-*.log`, `slice3-*-game/replay/save`; CI receipts `slice3-final-ci-check-runs.json`, `slice3-required-checks.json`; public approval receipt `slice3-approval-receipt.json`.

Both COMPLETE saved histories backed up before continuation at `evidence/issue17/slice3-backup-run209/` (`sessions/`, `review-sessions/`). Exact resume paths:
- writer `evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl`
- reviewer `evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl`

Resume command shape (next assignment, GoDeepSeek only, run overlay `.starter-run.yml`, tools read,bash,edit,write,grep,glob,todo,web_search):
`omp -p --no-pty --auto-approve --max-time 45m --config C:/AI/aperture-worktrees/issue-17/.starter-run.yml --model opencode-go/deepseek-v4.1-flash --tools read,bash,edit,write,grep,glob,todo,web_search --resume <EXACT_WRITER_JSONL> @<NEXT_ASSIGNMENT.md>`

## Next concrete work (unfinished)
Continue sourced missing names from the exact desired lists, prioritising lists whose remaining cards reuse existing primitives. Suggested next package: Ayula bears (vanilla/ETB) and Zada cheap red spells, plus Isamaru white cheap soldiers only where the printed text is exactly implemented; every card needs authoritative source + genuine RED before production and an exact package journey. Seven whole 100-card lists remain unsupported; do not certify or merge partial work.

Shared core/card/catalog/engine hooks remain an integration hotspot with separate #16; this work does **not** approve #16 integration. No import/start UI or AI-search edits were made. Reporter pointer still targets completed issue20 and was not changed here.
