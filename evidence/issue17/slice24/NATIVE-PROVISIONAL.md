# Native provisional slice24 inspection (while original writer active)
Not a verdict; first-cut still evolving. Recheck at writer checkpoint.

1. engine detach_object currently retains/removes entire trigger_queue+stack entries when affected=id leaves. A non-target trigger remains on stack and resolves doing nothing to that departed identity, not silently disappear (CR603.3/400.7). Prefer invalidate affected=None in queued/stack/pending refs without deleting ability. Ensure unrelated queue progress and stack counters maintained; actual identity control.
2. add_haste_grant currently self.state.object(item.object).card can panic after a commander source left/rekeyed. Use captured item.card, as source-left ability remains valid; don't require source live. Need source-left test via actual cast/Unsummon including commander-rekey if applicable, not fixture new source.
3. mana_sources selection had (!o.summoning_sick).then_some while validate_tap_mana now has_haste — actual offered creature mana may still omit hasty source (Werebear). Need actual legal_actions test, not direct submit alone.
4. Added affected references need semantic state/archived-reference validation and legitimate source-left load control. Existing no-target ability shouldn't accept forged target-like binding arbitrarily; avoid broad new state rewrite but use normal reference validation.
5. StateBasedAction message says creature 'enters with' counter, but this is stack trigger resolution after entry, not replacement; event wording should say 'gets a +1/+1 counter'.
