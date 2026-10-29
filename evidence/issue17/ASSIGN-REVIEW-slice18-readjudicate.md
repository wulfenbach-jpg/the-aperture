# Slice18 re-adjudication — SAME original reviewer history, bounded 8 min

You reviewed `2785c2c` and returned CHANGES_REQUESTED for three stale record
statements in `docs/issue17-starter-corpus.md` (code fine). The fixes are now on the
branch at `60e0bd9e7a706e92bc6d8f5e2e7534ed27a97bbf` (parent `2785c2c`), pushed to
draft PR46. This is a documentation-only delta; no product source changed.

Applied:
1. `:48-52` — union stays **459**, now **123 supported / 336 missing**, with a note
   that slice 18 moved the split by one and that the union includes the five basic
   land types (this is what my earlier 454/118 parse omitted — your audit was right).
2. `:176` — Ayula's printed counter/fight trigger is now described as implemented
   (bounded `TargetSpec::Bear` / `TargetSpec::BearYouControl` + typed fight,
   CR 701.14a/b), with other counters/tribal statics still unimplemented.
3. `:320` — commanders section now says **two** target commanders remain unsupported
   (Elas, Zada) and lists Ayula among those with named bounded support.
4. `:155` — the token bullet no longer claims Ayula needs her own token type.

Also updated `evidence/issue17/SLICE18-HUMAN-CHECKPOINT.md` to record the
reconciliation (454/118 was my audit artefact because I filtered basics; reviewer's
459/123/336 stands) and the fact that the record corrections were authored by me,
the integration owner, not the writer.

Please confirm by re-reading the current file (not this summary): the three
statements are correct and consistent with the manifest/catalog, no other stale
count remains in that document (`grep` for `122`, `337`, `Three target commanders`,
"Ayula's printed counter/tribal text is not implemented"), and the diff
`2785c2c..60e0bd9` touches only `docs/issue17-starter-corpus.md` plus evidence.

Then write your verdict into `evidence/issue17/slice18/REVIEW18-ORIGINAL-HISTORY.md`
(either amend the existing report with a "RE-ADJUDICATION at 60e0bd9: APPROVED_INCREMENT"
section or add `REVIEW18-READJUDICATION.md`), state the exact reviewed head, and keep
the standing caveats: full #17 unfinished (1 of 8 certified, 336 missing union
names), no merge, no closure, no downstream unlock, journey is an evidence-only
40-card package with Ayula injected for all four seats. Commit the report and stop.
