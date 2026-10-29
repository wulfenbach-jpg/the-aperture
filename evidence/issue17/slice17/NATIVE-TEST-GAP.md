# Slice17 native test inspection at bfaa782

Current morbid test is vacuous in two places and does NOT prove the claimed behavior:
- After drain, trigger_queue is always empty even if an incorrect no-death trigger fired and resolved. Assert the actual entered Ulvenwald has ZERO counters and no staged morbid trigger for no-death case.
- Positive branch creates a new opposing target only AFTER TriggerTarget offer and then ends with target.is_none() || counters==2. If no target branch is reached at all, target None passes. Keep an opposing target present BEFORE the trigger offer, select it from the offered target list, then require its existence and exactly2 +1/+1 counters. Remove permissive None branch. Add token-only death positive (no prior nontoken deaths) to actually prove token death enables morbid.

No source behavior issue proven here; conformance assertions insufficient. Strengthen tests, run them, repair real behavior if exposed. Need not re-audit unrelated suites. Preserve all previous failure logs; do not claim initial suite was stronger than it was.
