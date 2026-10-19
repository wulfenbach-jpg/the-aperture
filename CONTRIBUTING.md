# Contributing to THE APERTURE

Original Rust source is offered under **MIT OR Apache-2.0**, at your option. Contributions are accepted on those same terms unless separately agreed. Third-party content keeps its own terms; see [ASSETS.md](ASSETS.md).

## Scope first

Read [README.md](README.md), [the roadmap](docs/ROADMAP.md), the relevant GitHub issue and its dependencies. The current application is a 15-card, two-seat practice slice, **not Commander**. Do not make an unsupported deck appear playable, silently substitute defaults, or promote planned support to implemented support.

## Branch and diff discipline

* Preserve the existing `master` history. Use a focused branch and PR per coherent change; no force pushes to the default branch.
* Separate concurrent agents with worktrees. One integration owner coordinates shared interfaces and reviews before merging; workers must not race in one checkout.
* Keep changes reviewable. Do not mix generated evidence, broad refactors and unrelated fixes in a feature commit.
* Never commit credentials, local auth state, private model records, model weights, caches or build directories. Test credential strings must be visibly synthetic.

## Executable evidence

1. Write a failing regression or conformance test before changing a rule or behavior. Retain the real RED and GREEN output.
2. Use the pinned official rules revision and quote rule identifiers in relevant conformance fixtures. Models select legal actions; they never supply rules code, priority scheduling or state-based actions.
3. Run `cargo fmt --check`, `cargo test --all-targets --locked`, and `cargo clippy --all-targets --locked -- -D warnings`. New board tooling also requires the Node and Python tests in `.github/workflows/ci.yml`. The optional [real-browser smoke procedure](docs/FOUNDATION.md#roadmap-and-real-browser-checks) requires Node 22+, an isolated Chromium CDP endpoint and a loopback server serving `docs/`.
4. Verify the actual vertical path: input → certification → live state → visible choice → persistence/replay. For UI work, exercise real input on the built app at supported sizes; a screenshot alone is not an interaction test.
5. Preserve viewer-masked information boundaries and validate revision, decision serial, seat and UI session generation before applying asynchronous proposals.
6. Request independent review. Include changed paths, commands/results, raw report paths, limitations and the exact tested revision. A worker's self-report is not independent proof.

## Workflow

The [kanban](https://wulfenbach-jpg.github.io/the-aperture/) reads GitHub issues. Open issues have exactly one `status:` label: backlog, ready, in-progress, review or blocked. Close only after acceptance evidence is attached; closed issues appear in Done. Use `python scripts/github-board.py status NUMBER STATUS` for a verified transition, or edit labels on GitHub. `python scripts/github-board.py snapshot` refreshes the dated fallback; it does not commit or push it.

A bootstrap rerun creates only missing ownership-marked roadmap issues and does not overwrite work in progress. Keep [docs/roadmap.json](docs/roadmap.json) as the source specification and link scope changes in the issue.
