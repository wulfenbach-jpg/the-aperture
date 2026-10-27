"""Finalize the slice-23 checkpoint: real head, CI receipt and review outcome."""
import io
import json
import pathlib

root = pathlib.Path("C:/AI/aperture-worktrees/issue-17")
p = root / "evidence/issue17/SLICE23-HUMAN-CHECKPOINT.md"
s = p.read_text(encoding="utf-8")

ci = json.loads((root / "evidence/issue17/slice23/exact-head-ci-632c5e7.json").read_text(encoding="utf-8"))
success = [r for r in ci if r["c"] == "success"]
keyed = all(r["sha"].startswith("632c5e7") for r in ci)

s = s.replace(
    """Committed this slice: `SLICE23_HEAD` (see the commit subject
"Issue 17 slice 23: Goreclaw cost reduction and granted trample").
Local == remote after the push readback recorded in
`evidence/issue17/slice23/push-readback.txt`.
**No merge, no master push, no issue closure, no certified-downstream unlock.**""",
    """**Final head of record: `632c5e7df413169f009119510cbb3cd9930443d5`** (commit
"Issue 17 slice 23: Goreclaw cost reduction and granted trample"), parent
`89247d6`. Local == remote == PR46 head (readback after push: `89247d6..632c5e7`).
PR46 is OPEN / draft / MERGEABLE. **No merge, no master push, no issue closure,
no certified-downstream unlock.**

**Exact-head CI: 10/10 check runs completed SUCCESS, every check keyed to
632c5e7** (`slice23/exact-head-ci-632c5e7.json`; rust ubuntu/macos/windows ×2,
windows package ×2, board and roadmap ×2). No green is inherited from any
earlier head.

This checkpoint and `slice23/REVIEW23.md` are delivered as card attachments rather
than committed, so the branch tip stays at the head its CI covers.""",
)

s = s.replace(
    """## Known gaps (unchanged by this slice)""",
    """## Independent review

A fresh delegated reviewer (not one of the original OMP histories — disclosed)
executed the delta `89247d6..632c5e7` itself: 11/11 focused, 10 pinned suites,
fmt/clippy/release, the full workspace suite (110 binaries, 775 passed / 0 failed
/ 1 ignored), the corpus audit, replays of slices 17–22 still matching, a rebuilt
journey with byte-identical regenerated artifacts, and **15 probes of its own** in
an out-of-tree scratch crate (noncreature/other-seat non-reduction, pip floor,
power 3 vs 4 threshold, source off the battlefield, exact {2} cut, no charge on a
refused attempt, real combat 40→31 versus a 40 control, below-threshold creature
untouched, cleanup expiry, no grant for a reused object id after departure,
commander-tax interaction, layer ordering, grant survives save/load, printed-only
`Object::has_trample`). **Verdict: APPROVED, no blocking defect.**
Record: `evidence/issue17/slice23/REVIEW23.md`.

It raised four **non-blocking** stale-comment findings that this delta created and
that are carried forward rather than patched into an evidence-only head:

1. `README.md:303` still says layers 4–6/7b/7d are unimplemented although a layer
   6 ability grant now exists.
2. `crates/core/src/layer.rs:3-8`, `:31-37` — same staleness.
3. `crates/core/src/engine.rs:446-448` — "Only layer 7c exists in this starter
   set" is stale.
4. `crates/core/src/state.rs:543-546` — `Object::has_trample`'s comment claims
   nothing grants trample; combat uses `Game::has_trample`, so no behaviour bug,
   but a future UI/coach/AI caller would silently miss a granted keyword.

## Known gaps (unchanged by this slice)""",
)

s = s.replace(
    """* `view`/projection does not display a granted keyword; combat enforces it, and
  no UI change is made in this lane.""",
    """* `view`/projection does not display a granted keyword; combat enforces it, and
  no UI change is made in this lane (confirmed at projection level by the
  reviewer).
* Four stale layer/comment statements listed under "Independent review" above.""",
)

s = s.replace(
    """* `slice23/corpus-audit.log` — `AUDIT OK: catalog 140, union 459 = 128 supported
  + 331 missing, Ayula 25, 1 certified`, per-list `55/0/51/48/25/52/52/53`.""",
    """* `slice23/corpus-audit.log` — `AUDIT OK: catalog 140, union 459 = 128 supported
  + 331 missing, Ayula 25, 1 certified`, per-list `55/0/51/48/25/52/52/53`.
* `slice23/workspace-tests.log` — full workspace suite exit 0 (108 suites) at the
  committed bytes; `slice23/native-gates-at-632c5e7.log` re-runs the gates script
  at the committed head.
* `slice23/exact-head-ci-632c5e7.json` — 10/10 SUCCESS keyed to 632c5e7.""",
)

p.write_text(s, encoding="utf-8", newline="")
print("checkpoint finalized; ci success", len(success), "keyed", keyed)
