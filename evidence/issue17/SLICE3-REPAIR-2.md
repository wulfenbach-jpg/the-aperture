# Slice3 repair-2 — reviewer truthfulness findings only
SAME writer,10minute hardcap. Review2 requested changes on0d8a1ee for exactly two real stale statements. No code behavior changes expected; do not touch tests/engine. Preserve all evidence and failed review.

1. manifest krenko-goblin-tokens.missing_interactions says remaining52; Guttersnipe reduced exact Krenko missing to51. docs/issue17-starter-corpus.md line ~58 repeats52. Update both truthfully; preferably phrase from verified count51.
2. manifest elas-sacrifice.missing_interactions still says black basic Swamp unsupported; docs lines ~74-75 repeat no-black-basic claim although Swamp mdd/296 is now supported with black mana. Replace with actual remaining Elas gaps: its nonland names unsupported + sacrifice/tutor/reanimation interactions, no stale Swamp claim.
3. Handoff changed-files lists docs/issue15-effects.md but no delta in range; remove that false claim.

Do not hardcode new counts elsewhere unless programmatically verified from manifest/script. Re-run python evidence/issue17/slice3-native-corpus.py to confirm93/459/81/378 and per-list Krenko51/Elas58; focused doc/manifest check + git diff --check only. No commit/push. Update handoff with repair evidence and exact commands. Stop.
