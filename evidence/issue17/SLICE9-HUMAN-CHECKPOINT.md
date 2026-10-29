# Issue 17 — slices 8–9 rolling continuation checkpoint

Full original #17 is UNFINISHED. This is an independently reviewed implementation increment on the same draft PR46, not eight-list certification or release acceptance. No merge, master push, force push, issue closure or certified downstream unlock occurred.

## Candidate and approvals

- Branch: aperture/issue-17; PR46 remains OPEN/draft.
- Exact current head: ab9d4f50c1cd8b77ad8b358976fcf89a9a000864.
- Slice8 approved at c22b240be304802ed31a7f7f9acb8d656d6218bf, 10/10 exact-head CI SUCCESS.
- Slice9 approved at ab9d4f50c1cd8b77ad8b358976fcf89a9a000864 by the SAME saved independent reviewer, round2, zero findings. Code head4e8a856; subsequent ea17c27/ab9d4f5 changes are documentation/evidence only.
- Final candidate exact-head CI: **10/10 completed SUCCESS**, all bound to ab9d4f50c1cd8b77ad8b358976fcf89a9a000864; configured required checks PASS. Final receipt timestamp2026-10-26T13:53:48Z. Local=remote=PR=independently reviewed head; PR OPEN/draft/MERGEABLE/CLEAN. Body updated and read back exactly. No stale code-head CI substitution.

## Newly usable scope

Eight sourced missing names added across two increments: Soul's Attendant (optional other-creature-entry life), Razorclaw Bear (blocked-once pump), Volley Veteran (opponent-creature-only, resolution-time Goblin damage), Dramatic Reversal (own nonlands untap), Goblin War Strike (Goblin-count player damage), Brightstone Ritual (all battlefield Goblins), Mana Geyser (tapped opponent lands), Rite of Flame (two red plus one per namesake in all graveyards).

Mana quantities/events widened from u8 to u32; ordinary costs/pips remain u8. Payments retain exact remainder above255. Large pool rendering is bounded rather than allocating one symbol per unit. u32MAX saturation is an explicit representational limit, not unbounded Magic arithmetic. Existing numeric JSON/digest behavior remains compatible with the authentic slice8 replay.

All new printings were retrieved live before implementation with timestamp/source in fixtures/starter-corpus/slice8-sourced.json and slice9-sourced.json; native immutable-id readback matched every recorded field. Brightstone's slice9 record supersedes its historical slice8 exclusion. Friendly Teddy and Aetherize remain genuinely unsupported: multi-player draw continuation/APNAP and commander-hand-replacement continuation respectively. Goblin War Strike's printed planeswalker half is unreachable because no planeswalker permanents exist in this pool.

## Verified results and limits

- Slice8 independent affected suites: 101 passing tests, then final9-test slice8 suite after unblocked control; fmt/Clippy/build and exact package game/replay.
- Slice9 independent affected suites: **47**, not101. Actual result lines are6,5,4,4,10,6,12. The round2 review JSON accidentally carries101 from slice8 in its reused-evidence prose; this checkpoint corrects that clerical claim without modifying the independent report. Native final slice8+slice9 tests were9+6 passing.
- Slice8 four-seat40-life exact40-card package, seed801: terminal P0,2377actions/68turns,6087events,digest ffbeaee0db38c4de6a90bed44c5f111ee09cb05e54a3d2b86adf75ffa3a4b082; separate replay matches. Razorclaw/Volley triggers occur. Soul's Attendant casts were countered; Reversal/WarStrike were not cast. Focused tests prove those effects.
- Slice9 exact40-card package, Krenko each seat,40life,seed901: terminal P0,1339actions/32turns,4379events,digest5d20bb4a35e2adbe21eab65eee427cdb9f2726d51fe027d57ae2acbc528a4626; separate replay matches. Heuristic casts none of the three new rituals; focused tests, NOT that terminal trace, prove their resolution behavior.
- Neither package is a desired100-card starter or certification substitute.
- Slice8 initial absent-catalogRED is genuine; mechanics-first implementation and later harness corrections are disclosed, not retroactively strictTDD. Slice9 genuine pre-engineRED covers300-mana saturation and scaffolded no-op spells; actual failing tool output retained. Empty quiet-command logs are not proof: corrected handoff cites independently reproduced focused/Clippy/build results.
- Desired eight deck contents and historical slice3 audit files unchanged. No UI/import/search implementation edits; only one necessary AI heuristic numeric-comparison adapter.

## Current corpus and release gaps

Catalog114; exact union459 =102 catalog-supported names +357 missing. Per-list missing55/0/51/48/46/55/54/53 (Isamaru/Jasmine/Talrand/Krenko/Ayula/Elas/Tatyova/Zada). One implemented, zero certified of eight. Catalog membership alone is not Commander certification or whole-list playability. The existing analyzer gameplay refusal remains unchanged. Legend/equipment/auras, tribal/counter commander text, tutors/reanimation and most desired printed cards remain real gaps. Arbitrary Archidekt compatibility and measured playing strength remain unclaimed. Child t_3c2fe51a remains unassigned TODO; no certified#26/#32 unlock.

## Continuation ownership

All bounded writer/reviewer processes stopped. Slice8 used6 assignments; slice9 used4 (native docs-only repair does not create another OMP writer). Existing automatic15-minute continuous supervisor owns renewal on THIS same unfinished card/PR/branch, not a new team or human/quota block. No additional user approval is required for the authorized continuation. Preserve approvals; next slice continues sourced missing-card/interaction implementation, then one concise practical delta review and exact-head CI. Do not re-audit slices1–9 or mark #17 complete.

Exact saved histories:
- writer: evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl
- reviewer: evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl
- run overlay .starter-run.yml pins existing opencode-go/deepseek-v4.1-flash with no paid fallback; globals preserved.
- backups: evidence/issue17/slice8-backup/, slice9-backup/, slice9-final-backup/.

Resume pattern from this checkout: omp -p --no-pty --auto-approve --no-title --config .starter-run.yml --model opencode-go/deepseek-v4.1-flash --session-dir evidence/issue17/sessions --resume <exact-writer-jsonl> --tools read,bash,grep,glob,write,edit --max-time 30m @<new-scoped-assignment.md>. Reviewer uses its own exact history after writer stops. No unfinished implementation or CI command remains in flight. Resume the next sourced package on the same original unfinished task; final publication and review gates for this increment have passed.
