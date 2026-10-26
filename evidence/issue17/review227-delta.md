# Run227 independent delta adjudication

**Verdict: APPROVED_INCREMENT**  
Exact head: `7e1eb5f1b8218e24d7a03377c3cb3c5bb0981a4f`

S6-R2-F1 is genuinely repaired. The unchanged external probe re-ran at this head
with full stack/trigger/resolving/pending drainage: **3/3 passed**.
Single dying Zulaport ended **[21,19,19,19]**; two dying creatures ended
**[22,18,18,18]** in both creation orders; the surviving-watcher control also
ended **[22,18,18,18]**. `f1b7ab6` retains the original dying identity and
excludes the separately dispatched self-death from the watcher batch.

The merge/integration checks passed:

- `edd8008` parents are `f1b7ab6` and master `8de0961`.
- `04c39e6` restores real destination/death recording before SBA cessation:
  tokens reach graveyard/hand/exile/library, applicable triggers fire once, then
  `check_sbas` detaches them with no `Zone::Removed` tombstone.
- `7e1eb5f` fixes the contradictory Swords line: the trace says the token
  “is exiled (token cessation awaits the next SBA check, CR 111.7)” and the
  regression forbids “instead of being exiled.”
- Focused run: `issue17_tokens` 10/10, `counters_tokens_library` 16/16,
  `issue17_slice6` 13/13. That includes pre-#16 save load/replay/digest,
  counters/revealed/keyword/library controls, destination-before-cessation,
  no-tombstone, watcher-once and Swords checks.

No desired list changed: `git diff 3987dd6..HEAD -- decks/starter-corpus/planned`
and the Jasmine list are empty. Native audit remains **106 catalog / 459 union
(94 supported, 365 missing) / 1 implemented, 0 certified**.

Fresh handoff evidence for the 639-test workspace, build, fmt, clippy, 10/10
exact-head CI and the package journey/replay is reused; no duplicate matrix.

**Residual full-issue gaps:** seven desired starters remain unsupported, no
starter is certified, and #16 typed primitives are not product acceptance or
release approval. This increment must not close #17, merge it, or unlock
certified/#26/#32 downstream lanes; continuation returns to the same
card/branch/PR and preserved histories.
