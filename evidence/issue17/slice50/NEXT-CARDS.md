# Continue Ayula after slice50 review — same card / branch / PR46

No new card was enabled in slice50. Corpus remains 161 cards / 459 union =
148 supported + 311 missing; 1 of 8 certified, Ayula 5 missing. The next
implementation is **Ruxa, Patient Professor**, not another Cub rewrite or
historical audit. Use the retained source at
`evidence/issue17/slice46/sources/ruxa-patient-professor.{json,stamp.json}`
(C21 66; printing e07b8142-6a49-46e7-b862-41f89a59b894; source retrieval
2026-10-28T13:49:35Z). Re-fetch the exact printing as needed, not a remembered
paraphrase.

## Entire Ruxa text must work before enabling

1. Enter OR attack: mandatory target creature **card with no abilities** in its
   controller's graveyard returns to hand. Zero targets means no valid trigger
   target; an eligible target is not an optional decline. Card/incarnation
   validation must survive the ordinary zone paths and serialization.
2. ALL controlled creatures with no abilities get +1/+1, not just 'other'
   creatures. Ruxa itself has printed abilities, so does not qualify. No-
   abilities is not the same as vanilla printed P/T: equipment granting
   haste/shroud/hexproof, temporary trample, intrinsic mana abilities on animated
   basic lands, and the printed activation of an animated artifact matter.
   Define one effective battlefield predicate; the graveyard card predicate can
   use printed abilities because battlefield grants no longer apply there.
3. Each qualifying creature MAY assign damage as though unblocked. Do not
   silently hard-code yes, grant trample, or mark it unblocked. It remains
   blocked for other rules purposes. The option belongs to that creature's
   controller at damage assignment. Accept a minimal typed pending instruction
   and make real offers, owner refusal, save/load and continuation work before
   calling the card supported. No new policy or strength claim is authorized.

The remaining names after Ruxa are Rampaging Yao Guai, Ursine Monstrosity,
Evercoat Ursine and Dúnedain Rangers; their full sources are in the same source
directory. X-creature casting / multi-target sum cap, forced random attacks /
indestructible, hideaway/play-from-exile and the Ring remain real mechanics,
not data-authoring tasks. Do not enable a half implementation.

After each new card update BOTH missing_cards and the WHOLE missing_interactions
field, audit current counts, retain genuine focused RED/GREEN, run one actual
feature journey + separate replay and required exact-head CI, then one concise
independent practical delta review. Once Ayula really has zero card AND
interaction gaps, immediately run its exact 100-card four-seat certification
with separate replays. Only report 2/8 if that whole-list certification passes.

Slice50's historical declarations remain serialized but do NOT define the
Witness predicate. Witness uses live pre-departure membership; simultaneous
batches share one snapshot. CR509.1g means blockers remain blocking even after
the attacker leaves, represented by unpaired_blockers. Do not reintroduce the
superseded declaration-only interpretation or commander-never-dies guidance.

Keep the same saved writer history (native continuations are co-authors, not
independent reviewers), preserve untracked reviewer evidence and the modified
raw session file. No merge/master push/issue closure/dependent unlock/new team
or provider/model changes. Original #17 stays unfinished until all 8 lists pass.
