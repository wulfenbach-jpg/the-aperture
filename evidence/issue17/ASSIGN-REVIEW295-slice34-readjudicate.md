# Review 295 — re-adjudication of the review-294 correction, SAME reviewer history

You are the independent reviewer for issue #17 on draft PR46, branch `aperture/issue-17`. You are continuing your own saved history. The writer has stopped. Re-adjudicate **only** this narrow question plus the exact head:

* Previous head you reviewed: `b74815d6690d76a7d838bc491fd37b1d791328d0`, verdict **CHANGES_REQUESTED** for one record-integrity defect: `evidence/issue17/slice34/pre-mutation-sha256.txt` did not match the committed `crates/core/src/engine.rs` because it had been written before `cargo fmt --all` reformatted that file, while `SLICE34-HUMAN-CHECKPOINT.md` still claimed a byte-for-byte restore proven by `sha256sum -c`.
* Corrected tip: **`bfbbceb6e023644d080d980e91d922babaaca60c`** (single docs/evidence commit on top of `b74815d`; no `crates/`, `decks/` or fixture bytes in the delta — verify that yourself).

Rules: no production or test edits, no commits, no push, no merge, no issue closure, no subagents, no global config changes. Write your verdict to `evidence/issue17/review295/REVIEW295.md`. Keep it short — 8 minutes.

## What to check (by running)

1. `git diff --stat b74815d..bfbbceb` — is the delta evidence/docs only?
2. `sha256sum -c evidence/issue17/slice34/pre-mutation-sha256.txt` and `sha256sum -c evidence/issue17/slice34/post-mutation-ability-sha256.txt` — do both exit 0 against the current worktree, and do the `pre-`/`post-` pairs agree with each other?
3. Do the regenerated RED logs still show the discriminating failures the checkpoint claims (4 failing tests for the source-exclusion mutation, exactly 1 for the Little Bear spec mutation)?
4. Does `SLICE34-HUMAN-CHECKPOINT.md` now state the correction honestly, including that the earlier receipt was stale and why?
5. Exact-head CI at `bfbbceb`: `gh api repos/wulfenbach-jpg/the-aperture/commits/bfbbceb6e023644d080d980e91d922babaaca60c/check-runs`. Report exactly what you see, with counts and any run still in flight — do not describe in-flight runs as passes. (The writer's completed capture for the previous head `b74815d` is `evidence/issue17/slice34/exact-head-ci-b74815d.json`, 10/10 SUCCESS keyed to that head; the tip's own runs are what matters here.)
6. Spot-check that no product behaviour moved since the head you already approved the behaviour of: `cargo test -p aperture-core --test issue17_slice33 --locked` should still be 12/12.

## Contract frame

Original #17 remains unfinished: **1 of 8** certified 100-card lists, **322 of 459** union names unsupported, Ayula 16 short. Only an increment verdict is possible — never full #17 acceptance, merge, master push, closure or downstream unlock (`t_26a7a4ff`, `t_3c2fe51a` stay parked).

Deliver: verdict (`APPROVED_INCREMENT` / `CHANGES_REQUESTED` / `BLOCK`), the exact head, the checks you ran and their results, any remaining defect with file:line, and a one-line statement of the residual full-issue gaps.
