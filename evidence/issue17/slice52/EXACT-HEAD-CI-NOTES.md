# Exact-head CI readback for `cd19bef` — what is green and what is not

Read back from the GitHub API at 2026-10-28T21:47Z into
`evidence/issue17/slice52/exact-head-ci-cd19bef.json` (`watch-ci.py`,
`total: 10`, `all_keyed: true`).

Two workflow runs cover this exact commit, because the push and the pull-request events both fired:

| Run | Event | Checks | Result |
|---|---|---|---|
| `40770725440` | `pull_request` | 5 | **5/5 completed SUCCESS** (`rust` ubuntu/macos/windows, `windows package`, `board and roadmap`) |
| `40770718159` | `push` | 5 | 4 completed SUCCESS; `rust (macos-latest)` **cancelled** after 15 min with no step marked cancelled, so the run concluded `failure` |

**No clean 10/10 claim is made for this head.** What is true: every one of the ten check runs is keyed
to `cd19bef`, and the same tree is fully green on the pull-request run's five checks, including macOS.
The push run's macOS job was terminated by the runner rather than failing an assertion (no failing step,
no failing test output), which is the same class of infrastructure flake a prior reviewer hit and
re-ran; it was **not** re-run by this slice, so the `failure` conclusion stands as recorded and is not
reinterpreted as a pass.

For completeness: slice 51's evidence tip `6abd820` and code head `b4880b5` each carried their own
passing runs (`40763597738`, `40763607748`, `40760723473`, `40760711870`), and this slice changes no
product code, no test and no fixture — only `docs/issue17-starter-corpus.md`'s known-gap paragraph and
`evidence/` records — so no behavioural difference exists between the slice-51 heads and this one.
