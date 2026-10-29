# Slice 47 record corrections (review 327's R327-1..3) and one count

No product code changed in this commit. Three record statements were false at `1b1c542` and one
whole-field count was stale; this commit fixes them and adds the deliberate version decision the
review asked to be made explicitly rather than implied.

## R327-1 — the save half was false; both sites now say so, plus the version decision

**Old claim (checkpoint §2, `slice47/RESULT.json` disclosure 3):** pre-change saves "still load and
still verify their own stored digest".

**Executed at this head, which is why it was false:**

```
$ target/release/aperture-cli.exe replay --file evidence/issue17/slice45/demo-save.json
Error: replay ended early: digest 99bc84466546ebe4f873e76b751b3b01baa44825af6f0e8e930d4d90228431da
       != recorded 45d674d2e926bbab485106311929bd5c796d751676bac36b03efa8e8716e2143
EXIT=1
```

The save **parses and re-executes**; it does not verify. Second half, from the code rather than from
a claim: `capture_combat_alone` reads `turn.declared_attackers`, which a legacy mid-combat save
deserializes as **empty** (`serde(default)`), so for the remainder of that combat the alone question
answers "not alone" and a genuine alone-attack death stops firing Thijarian Witness. Nothing can
recover a declaration the writing build never recorded.

**Version decision, taken deliberately:** `SAVE_VERSION` stays **10**, `REPLAY_VERSION` stays **6**
(`crates/core/src/replay.rs:21-22`, unchanged by this commit). Rationale recorded in the checkpoint,
`RESULT.json`, README "Versioned persistence" and `docs/issue17-starter-corpus.md` §Slice 47: a bump
would reject every older artifact including the pinned pre-#16 v10 fixture whose purpose is to prove
an old save still loads, which is strictly worse than a digest under-report. The limitation is now
stated in user terms in all four places: older mid-combat saves replay to a different digest and may
under-report alone-attack triggers for the rest of that combat.

While editing that README paragraph its stale version numbers were corrected to the real constants
(it said replay v5 / save v8; the code says 6 / 10).

## R327-2 — the journey row cited a failed log

`slice47/journey.log` recorded a `--locked` lock-file error and `EXIT=101`; the success was
`slice47/journey-build.log` (`EXIT=0`). The checkpoint now cites `journey-build.log` as the gate, and
the failed file is kept rather than dropped but **renamed to `journey-locked-attempt-FAILED.log`**
with a three-line header saying it is not the gate, so a reader cannot mistake a red log for
evidence. `demo-journey.json` itself is untouched — the reviewer reproduced it byte-identically.

## R327-3 — code head and evidence tip were conflated

`RESULT.json.pr_state` published "headRefOid 4c55d94 == local == remote" while the PR head was
`1b1c542`. It now states both: code head `4c55d94` (local == remote), PR `headRefOid` `1b1c542` as
the evidence tip, differing by the receipts commit only.

## Whole-field count (the deferred item from review 327)

`decks/starter-corpus/manifest.json` `starters[elas-sacrifice].missing_interactions[0]` said
"remaining 53 desired-list nonland names" against **52** `missing_cards`. Corrected to 52. A
whole-field recheck of all eight starters after the edit: `missing_interactions[0]` counts now equal
`missing_cards.len()` everywhere they state a number, and no interaction entry names a supported card.

## Recorded, not fixed

The declaration record is outside `GameState::validate`. Written up as a known weakness in
`docs/issue17-starter-corpus.md` §Slice 47 and the checkpoint §2 for a later hardening pass, as
review 327 asked, rather than silently dropped.

## Gates run for this commit

| Gate | Result | Log |
| --- | --- | --- |
| `cargo test -p aperture-core --test starter_corpus` | 8 passed / 0 failed | `record-corrections-tests.log` |
| `--test catalog_pool` / `card_printings` / `command_zone_catalog` | 5 / 17 / 5 passed, 0 failed | same |
| `--test issue17_slice15_corpus` | 2 passed / 0 failed | same |
| `cargo fmt --all --check` | exit 0 (no Rust changed; still checked) | `record-corrections-fmt.log` |

No engine behaviour, digest, corpus count or certificate changes in this commit: catalog stays 160,
the eight-list gate stays **1 of 8**, Ayula stays at 6 missing names.
