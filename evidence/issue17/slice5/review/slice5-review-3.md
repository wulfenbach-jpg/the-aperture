# issue 17 — slice 5, round 3 review (S5-R2-F1 only)

- **Reviewed head:** `a8f10a2c82513c910125b5854da5eadbde5f587a` (`aperture/issue-17`)
- **Repair commit:** `a8f10a2 docs(handoff): correct the each-opponent ordering note (S5-R2-F1)`
- **Scope:** the single open finding of round 2 (`S5-R2-F1`). No other claim re-reviewed.
- **Verdict: `approve_increment`** — the corrected documentation is now accurate.

## Finding

### S5-R2-F1 — repaired, confirmed

**Claim checked:** HANDOFF.md's limitation bullet about ordering for "each opponent"/"each player" effects.

**Text after `a8f10a2` (HANDOFF.md:317-323):**

> Effects that act on "each opponent" (`Effect::MillEachOpponent`,
> `Effect::DamageEachOpponent`) iterate seats in seat order, not in APNAP order
> (CR 101.4). Per-player graveyards and life are unaffected, so the deviation is
> only visible in the order the events are logged and no chooser gains
> information from it. Crack the Earth's "each player sacrifices" does **not**
> share this deviation: it walks APNAP order from the active seat
> (`apnap_order_from`) and is covered by `crates/core/tests/apnap.rs`.

**The previous wording** attributed the seat-order deviation to "`Effect::MillEachOpponent`, `Effect::DamageEachOpponent`, Crack the Earth's sacrifice" — wrong for the third item. That clause is gone.

## Line-by-line check against the code

| Documentation clause | Code | Match |
| --- | --- | --- |
| `MillEachOpponent` iterates seat order | `engine.rs:4434-4442` — `for opponent in 0..self.state.seat_count()`, skipping `controller` and non-live seats | yes (seat order from seat 0) |
| `DamageEachOpponent` iterates seat order | `engine.rs:4456-4460` — same `0..seat_count()` loop, same skips | yes |
| Crack the Earth's each-player sacrifice uses APNAP from the active seat | `engine.rs:4334-4337` `Effect::EachPlayerSacrifice` -> `begin_apnap_sacrifice` (`:4043-4058`), `anchor = state.turn.active`, order from `apnap_order_from` (`:4032-4041`) | yes |
| …and the batch is applied in that APNAP order | `apply_apnap_choices` (`engine.rs:4180-4197`) iterates `apnap_order_from(anchor, &eligible)` | yes |
| …and it is covered by `crates/core/tests/apnap.rs` | that suite drives `CardId::CrackTheEarth` end to end, incl. `apnap_order_anchors_at_nonzero_active_and_wraps` (active seat 2 -> `[2,3,0,1]`) and `apnap_skips_an_already_lost_seat` | yes |

No other consumer of the deviation went undocumented: `Effect::EachPlayerSacrifice` is produced only by `SACRIFICE_ALL` in `catalog.rs:37` (Crack the Earth), and a grep across `crates/` and `decks/` shows no further each-player ordering path.

## Test gates (re-run, no behaviour changed)

```
cargo test -p aperture-core --test apnap
  test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo test -p aperture-core --test issue17_slice5
  test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Identical pass counts to round 2 — consistent with a documentation-only commit.

## Worktree integrity

- `git show --stat a8f10a2` -> `HANDOFF.md | 12 +++++++-----`, `1 file changed, 7 insertions(+), 5 deletions(-)`; `git diff --name-only a8f10a2^ a8f10a2` -> `HANDOFF.md` only. The repair touched documentation only.
- `git status --porcelain` (tracked entries) -> only ` M crates/core/src/catalog.rs`. `git diff -- crates/core/src/catalog.rs` is empty (0 bytes) and the worktree sha256 `73b5864…15a4c8` equals `git cat-file blob HEAD:crates/core/src/catalog.rs`: a CRLF/stat-dirty index entry with no content change, present before this review. No tracked modifications were introduced or left behind by the repair.
- This review performed no commit, push or merge; `HEAD` is still `a8f10a2`.
- Files written by this review: this report, and a `round3` key appended to `slice5-review-2.json` with the earlier content preserved. Both are untracked evidence files.

## Conclusion

The finding is closed. The sentence that misdescribed Crack the Earth is removed and replaced with a statement that matches `engine.rs` exactly, backed by the APNAP suite. No new inaccuracy was introduced by the edit and no gameplay behaviour changed. **`approve_increment`.**
