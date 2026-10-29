from pathlib import Path

p = Path("evidence/issue17/slice27/pr-body-after.md")
b = (Path("evidence/issue17/slice27/pr-body-readback.md")).read_text(encoding="utf-8")

old = ("The one-line correction is now **committed**, the worktree is clean, and the driver "
       "re-run from the committed bytes reproduces")
new = ("The one-line correction is now **committed** at `27d8f48` (no tracked file under `crates/` "
       "or in the driver is modified at this head), and the driver re-run from the committed "
       "bytes reproduces")
assert old in b, "old sentence not found"
b = b.replace(old, new)

extra = """
**Worktree state, stated precisely.** Four tracked evidence files from the slice-23 finalisation still carry uncommitted edits that predate this repair and touch no code or driver (`SLICE23-HUMAN-CHECKPOINT.md`, two session `.jsonl` append-only logs, `slice23/native-gates.log`). They are disclosed here rather than folded into `27d8f48`, so the reviewed and CI-green head does not silently absorb unrelated history appends.

**Merge-ref CI, stated precisely.** The `push` run keyed to `27d8f48` is **SUCCESS 5/5** (`evidence/issue17/slice27/ci-27d8f48.json`). The `pull_request` run for the same commit is **FAILURE** on all three rust jobs, and it is not caused by this delta: the PR run compiles the synthetic merge with `origin/master` `51dffda` (#28 move review, PR56), whose new `crates/ai/tests/review.rs:60` passes a `u8` to this branch's `crates/core/src/mana.rs:86` `add(&mut self, Color, u32)`. That widening is branch-only commit `e300bdd` (approved slice-9 work) and is absent from master, so the merged tree does not compile until the two lanes reconcile the mana API. This is an integration dependency owned by the supervisor, not a slice-27 defect, and it is flagged as a hotspot on the card.
"""

marker = "\n### Behavior increment: slice 26"
assert marker in b, "marker not found"
b = b.replace(marker, extra + marker, 1)
p.write_text(b, encoding="utf-8")
print("bytes", len(b))
print("clean-claim fixed:", "no tracked file under `crates/`" in b)
print("hotspot section:", "Merge-ref CI, stated precisely" in b)
