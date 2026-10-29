# Review 294 — concise independent adjudication of the review-290 repair, SAME reviewer history

You are the independent reviewer for issue #17 on the existing draft PR46, branch `aperture/issue-17`. This is a continuation of your own saved history; the writer has stopped. Review **only** the delta `cff319d..b74815d6690d76a7d838bc491fd37b1d791328d0` (code commit `5b60c2c`, evidence commit `b74815d`) at exact head **`b74815d6690d76a7d838bc491fd37b1d791328d0`**.

Rules: no production or test edits, no commits, no push, no merge, no issue closure, no purchases, no subagents, no global config changes. Write your verdict to `evidence/issue17/review294/REVIEW294.md` (and optionally a JSON receipt in the same directory). Keep it to 12 minutes of work and stay practical — DELIVERY_FIRST applies: focused checks on the changed behaviour, not another full-contract audit of approved increments 1–33.

## What changed (claims to test, not to trust)

Review 290 returned CHANGES_REQUESTED with one behavioural defect and two false record statements:

1. **Blocking.** Beorn's Hospitality's landfall trigger used `TargetSpec::CreatureYouControl`, which filtered out the ability's own source (an exclusion that exists for Little Bear's printed "another target creature you control"). Once the animation makes the enchantment a creature, it must be a legal target of its own "target creature you control" trigger, and the auto-chosen one when it is the controller's only creature. The repair splits the spec: `CreatureYouControl` now includes the source, and the new `AnotherCreatureYouControl` is the only one that removes it (used by Little Bear). Five regression tests were added; two existing slice-33 tests were updated because the landfall counter now legitimately lands on the animated source.
2. **R290-2.** `SLICE33-HUMAN-CHECKPOINT.md` said `card_printings` was 13/13; corrected to 12/12.
3. **R290-3.** The same checkpoint said `cff319d` "has no runs of its own"; corrected to 10 SUCCESS runs keyed to `cff319d`.
4. The "with none it does not put the trigger on the stack" wording in the checkpoint and `docs/issue17-starter-corpus.md`, plus a new docs subsection describing the spec split.

## What to verify by running, at this head

* `git diff cff319d..b74815d -- crates/` — is the engine change exactly the spec split (source included in `CreatureYouControl`, `AnotherCreatureYouControl` excluding it, label added), with no other rules behaviour altered? Is Little Bear's printed "another" preserved?
* `cargo test -p aperture-core --test issue17_slice33 --locked` (expect 12 passing) and `cargo test -p aperture-core --test card_printings --locked` (expect 12).
* A discriminating check of your own: revert or reason about the changed filter and confirm the new tests actually fail without the repair (the writer's logs are in `evidence/issue17/slice34/`, including `sha256sum -c` restore receipts — do not take them on faith, and do not leave the worktree modified; you may use an out-of-tree copy).
* The writer reports `cargo test --workspace --locked` 119 suites / 846 passed / 0 failed / 1 ignored, `cargo fmt --all --check` and `clippy -D warnings` clean at this head. Re-run the two cheap gates; do not repeat the whole workspace matrix if you have reason to trust the log plus your focused run.
* The journey: the writer rebuilt `evidence/issue17/slice33-journey-driver` at this head and reports the **same** digest `3dfb14d4…`, 1117 actions, 2967 events, byte-identical demo artifacts, and a matching separate CLI replay. Confirm the claim is consistent (rebuild only if cheap for you) and note that the repair is behaviour-neutral for that package.
* Corpus audit: `evidence/issue17/slice34/corpus-audit.log` — 149 catalog / 459 union = 137 supported + 322 missing, per list `55/0/51/48/16/52/52/53`, 1 of 8 certified.
* CI: `gh api repos/wulfenbach-jpg/the-aperture/commits/b74815d6690d76a7d838bc491fd37b1d791328d0/check-runs` — report exactly what you see (both the `push` and `pull_request` runs matter). Do not describe in-flight runs as passes.
* Confirm `crates/ai` and `crates/ui` are untouched by the delta and the desired deck snapshots are unchanged.

## Contract frame (unchanged)

Original #17 requires eight explicitly supported 100-card Commander lists, full interaction coverage, four-seat games on exact snapshots, and documented exclusions. At this head only **1 of 8** lists is certified and **322 of 459** union names are unsupported, so **only an increment verdict is possible** — never full #17 acceptance, never merge, master push, issue closure or downstream unlock (`t_26a7a4ff`, `t_3c2fe51a` stay parked).

Deliver: verdict (`APPROVED_INCREMENT` / `CHANGES_REQUESTED` / `BLOCK`), exact reviewed head, the checks you ran with their results, any real defect with a file:line and a reproduction, and the residual full-issue gaps. Do not restate the writer's prose as your own verification.
