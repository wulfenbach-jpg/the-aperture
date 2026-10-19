# THE APERTURE — path to v1

## Product contract

Build a FOSS, native Rust **single-player Commander simulator**: one human against up to three AI seats, original graphical table, exact Archidekt imports, deterministic rules/replay and trace-backed rules assistance. Keep the rules engine, masked observations, policy/search and renderer separate. Models choose legal actions; they do not perform state-based actions or decide what the rules are.

**Today:** a 15-card two-seat, 20-life practice slice. **Target v1:** four-seat Commander on an explicitly certified starter corpus, not a promise of arbitrary-card support. Eight certified 100-card decks is the proposed release-corpus gate. Wider card coverage grows through conformance tests and support manifests, not name-based guesses.

“Better in every way” is the ambition, not a current fact. Translate it into measured dimensions: correct supported interactions, import-to-play friction, native readability, responsiveness, accessibility, strategic play, review tools, reproducibility and FOSS distribution. Publish losses and coverage gaps alongside wins. Never compare a restricted corpus with a full engine as though their scope were equal.

## Milestone sequence

1. **M0 — Public foundation** (#1–#5): history-preserving public source, dual-license texts, three-platform CI, contribution/security policy, live issue-backed kanban and pinned rules/conformance provenance.
2. **M1 — Commander-ready multiplayer kernel** (#6–#10): 2–4 live seats, multiplayer priority/elimination, viewer masking, versioned replay and a four-seat headless harness. This milestone alone does not make the app Commander-compatible.
3. **M2 — Playable Commander vertical slice** (#11–#17): profile legality, command zone/tax, commander damage, ability scheduling, effects/layers, counters/tokens/library operations and certified decks.
4. **M3 — Native table and learning** (#18–#23): readable four-seat table, stack/priority teaching, practice QoL, exact per-seat imports, accessibility/responsiveness and study branches.
5. **M4 — Stronger play, measured** (#24–#28): information-safe search, optional local model, paired AI evaluation, comparisons and chess-style review without false confidence claims.
6. **v1.0 — Release candidate** (#29–#33): reproducible packages, actual platform QA, security/content review, determinism soak and an evidence-backed release decision.

[Machine-readable specification](roadmap.json) contains the full acceptance checklists and dependency keys. GitHub issues contain the authoritative live work status. No speculative delivery dates are set.

## Kanban

**Board:** https://wulfenbach-jpg.github.io/the-aperture/

**Issues:** https://github.com/wulfenbach-jpg/the-aperture/issues

The board is a public, token-free view backed by real GitHub issues, not a GitHub Projects v2 board. The current CLI token lacks Projects permission; this avoids blocking the team or exposing a token to browser JavaScript. It provides search, milestone filters, six columns and links to full acceptance criteria. Change status on the issue page or use:

```text
python scripts/github-board.py status 6 in-progress
python scripts/github-board.py snapshot
```

Open issues have one `status:` label: backlog, ready, in-progress, review or blocked. Closed issues are Done. A stale/incomplete browser fetch must not masquerade as current status: fallback mode visibly identifies the snapshot date. The board does not offer fake drag-and-drop; edits happen through authenticated GitHub issue controls. A native GitHub Project can be added after the account grants `project` scope.

`bootstrap` creates missing labels/milestones/ownership-marked issues idempotently, verifies remote titles/checklists/milestones and refreshes the fallback. It does not overwrite issue discussions, progress or labels on existing work.

## Release gates, not launch acknowledgements

* Pinned rules/conformance results and no silent unsupported mechanics.
* Exact certified deck snapshots through import, live play, save/load and verified replay.
* Real four-seat native input, readable stack/priority/choices and background-worker safety.
* At least 1000 declared-seed Commander soak games with failure/cap accounting and replay verification.
* Measured, paired AI evaluation with budgets, seat/deck rotations and uncertainty; no fabricated win probability.
* Clean-machine artifacts for every claimed platform, with checksums, source/license notices and a reviewed asset/model distribution policy.
* Independent review plus explicit remaining limitations. No stable v1 label until the gates pass.

## Agent integration contract

Sol coordinates architecture and integration. Bulk implementation/review uses the existing OMP task role (currently DeepSeek through OpenCode Go). Re-read local provider usage before expanding concurrency. If OpenCode Go actually rate-limits, use an already authenticated Codex Luna route only while its quota remains available; do not buy API credit, redeem reset credits or rewrite global provider configuration. Use isolated worktrees, bounded tasks and independently reviewed PRs. The integration owner owns commits, issue transitions and evidence; no worker can declare the release complete.
