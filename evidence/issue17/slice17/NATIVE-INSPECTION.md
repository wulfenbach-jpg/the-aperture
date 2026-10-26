# Native inspection of in-progress first implementation (not a verdict)

Observed concrete concerns at first diff; resolve before publication:

1. Death history resets inside cleanup_round, but CR514 cleanup is STILL SAME TURN and may repeat after SBAs/triggers/priority (e.g. delayed deaths happen during cleanup). Morbid must see earlier deaths in that cleanup priority. Merely clearing bookkeeping currently sets changed=true, which can also manufacture a cleanup priority round. Reset history only on actual new-turn transition (engine around turn.number +=1), not cleanup effects. Test a prior death through cleanup and reset at next actual turn.
2. EachPlayerDraws uses DrawResume::Effect {cursor:item.cursor+1}, remaining0. If a commander is on an earlier seat's library top and draw suspends for CR903.9b, resume skips remaining seats entirely. Existing state validator matches only Effect::Draw(n), so saving that suspension may also refuse. Introduce explicit serializable all-player draw resume progress (seat/order/counters), preserve existing old resume variants/digests, validate and test mid-instruction commander replacement + subsequent seats exactly once + save/load. If demonstrably unreachable supported setup explain, but do not silently skip legal players.
3. Ulvenwald puts counters by direct mutation with no typed CounterAdded event? Existing counter effects use event path — inspect and preserve authoritative event trace.
4. tracks_death_history scanning any object is a bounded closed-corpus opt-in to preserve old replay digests; document why valid (all playable cards start in engine objects, no outside-card source currently). Test old replay unchanged. Test simultaneous deaths not double counted and Caller ownership vs control (your graveyard text).

These are delta concerns, not permission to re-audit prior increments. Keep package small and correct. No scope expansion/UI/AI/old matrices.
