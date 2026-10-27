# REVIEW22-DELTA — re-adjudication of the mana-value repair

- Repository: `wulfenbach-jpg/the-aperture`, worktree `C:/AI/aperture-worktrees/issue-17`, branch `aperture/issue-17`
- Reviewed range: `c0e0a69..89247d66dd0de8a4923aa6bf3537326ac22d2509` (exactly one commit)
- Reviewed head: `89247d66dd0de8a4923aa6bf3537326ac22d2509`
- Head commit subject: `Issue 17 slice 22: review verdict, hybrid mana-value repair, slice evidence`
- Scope of this re-adjudication: **only** the repair delta on top of the already-approved `c0e0a69` slice-22 review. The rest of the slice was adjudicated in `evidence/issue17/REVIEW22.md` and is not re-reviewed here.
- Adjudicated: 2026-10-27T07:07Z

## 1. Delta contents — product vs evidence

`git diff --stat c0e0a69..HEAD`:

```
 crates/core/src/card.rs                   |  48 +++--
 crates/core/tests/issue17_slice22.rs      |  21 ++
 evidence/issue17/REVIEW22.md              | 315 ++++++++++++++++++++++++++++++
 evidence/issue17/slice22/REVIEW22.json    | 175 +++++++++++++++++
 evidence/issue17/slice22/corpus-audit.log |  21 ++
 evidence/issue17/slice22/corpus-audit.py  |  62 ++++++
 evidence/issue17/slice22/pr-before.md     | 211 ++++++++++++++++++++
 evidence/issue17/slice22/pr-body.md       | 222 +++++++++++++++++++++
 evidence/issue17/slice22/pr-readback.md   | 223 ++++++++++++++++++++
 9 files changed, 1281 insertions(+), 17 deletions(-)
```

| File | Class |
| --- | --- |
| `crates/core/src/card.rs` | **product** — `mana_value` body extracted; new `pub fn cost_mana_value` |
| `crates/core/tests/issue17_slice22.rs` | **product** — one added test |
| `evidence/issue17/REVIEW22.md` | evidence (review report) |
| `evidence/issue17/slice22/REVIEW22.json` | evidence |
| `evidence/issue17/slice22/corpus-audit.log` | evidence |
| `evidence/issue17/slice22/corpus-audit.py` | evidence (audit script; not built by cargo) |
| `evidence/issue17/slice22/pr-before.md` | evidence (PR body snapshot) |
| `evidence/issue17/slice22/pr-body.md` | evidence |
| `evidence/issue17/slice22/pr-readback.md` | evidence (PR readback) |

Confirmed: **no product file other than the two Rust files changed.** No fixture, no catalog data, no Cargo manifest/lockfile, no CI config, no docs outside `evidence/`.

### Source hunk 1 — `crates/core/src/card.rs` (`mana_value` delegates)

```rust
     pub fn mana_value(&self) -> u8 {
-        self.cost.map_or(0, |cost| {
-            cost.split('{')
-                .skip(1)
-                .filter_map(|rest| rest.split_once('}').map(|(symbol, _)| symbol))
-                .map(|symbol| {
-                    if let Ok(generic) = symbol.parse::<u8>() {
-                        generic
-                    } else if symbol.eq_ignore_ascii_case("X") {
-                        0
-                    } else if let Some((half, _)) = symbol.split_once('/') {
-                        half.parse::<u8>().unwrap_or(0)
-                    } else {
-                        1
-                    }
-                })
-                .sum()
-        })
+        self.cost.map_or(0, cost_mana_value)
     }
```

### Source hunk 2 — `crates/core/src/card.rs` (new free function, appended after `impl Display for CardId`)

```rust
+/// CR 202.3: mana value of a printed mana cost string, without needing a
+/// `CardDef`. Exposed separately from `CardDef::mana_value` so the rule can be
+/// tested on symbol shapes the current pool does not print (hybrid, Phyrexian,
+/// snow), instead of adding fixture cards only a test would ever use.
+///
+/// Each symbol counts as:
+/// * a pure number — that number (`{3}` = 3);
+/// * `{X}` — 0, because an X outside the stack is 0 (and no X-cost spell in the
+///   pool is a creature, so no supported card reads this while X is chosen);
+/// * a hybrid/Phyrexian symbol with a numeric half — that number (`{2/W}` = 2);
+/// * any other hybrid or Phyrexian symbol — 1 (`{G/W}`, `{W/P}`);
+/// * any other single symbol — 1 (`{G}`, `{S}`, `{C}`).
+pub fn cost_mana_value(cost: &str) -> u8 {
+    cost.split('{')
+        .skip(1)
+        .filter_map(|rest| rest.split_once('}').map(|(symbol, _)| symbol))
+        .map(|symbol| {
+            if let Ok(generic) = symbol.parse::<u8>() {
+                generic
+            } else if symbol.eq_ignore_ascii_case("X") {
+                0
+            } else if let Some((half, _)) = symbol.split_once('/') {
+                half.parse::<u8>().unwrap_or(1)
+            } else {
+                1
+            }
+        })
+        .sum()
+}
```

The only behavioural change is `unwrap_or(0)` → `unwrap_or(1)` on the numeric-less hybrid/Phyrexian branch. The function is reachable as `aperture_core::card::cost_mana_value` (`pub mod card;` in `crates/core/src/lib.rs:14`, `pub fn`), which the new test's `use` import confirms by compiling.

### Source hunk 3 — `crates/core/tests/issue17_slice22.rs`

```rust
+#[test]
+fn mana_value_counts_the_symbol_shapes_the_pool_does_not_print_yet() {
+    use aperture_core::card::cost_mana_value;
+    assert_eq!(cost_mana_value("{G/W}"), 1, "hybrid without a numeric half");
+    assert_eq!(cost_mana_value("{W/P}"), 1, "Phyrexian without a number");
+    assert_eq!(cost_mana_value("{2/W}"), 2, "hybrid with a numeric half");
+    assert_eq!(cost_mana_value("{X}{G}"), 1, "X counts 0 outside the stack");
+    assert_eq!(cost_mana_value("{S}{S}{2}"), 4, "snow symbols count 1 each");
+    assert_eq!(cost_mana_value(""), 0, "an empty cost is 0");
+    for def in catalog::all() {
+        let expected: u8 = cost_mana_value(def.cost.unwrap_or(""));
+        assert_eq!(def.mana_value(), expected, "{} disagrees", def.name);
+    }
+}
```

## 2. Is `cost_mana_value` correct under CR 202.3?

Probe: both implementations were extracted mechanically (`awk` over `crates/core/src/card.rs` at HEAD and over `git show c0e0a69:crates/core/src/card.rs`) into a standalone `rustc` program in the Hermes scratch dir (outside the repo), whitespace-normalised `diff` against the bodies inside the probe returned **empty** for both — the probe runs the exact reviewed code, not a paraphrase.

`rustc -O --edition 2021 probe.rs && ./probe.exe`:

```
cost                        CR202.3     new     old
"{3}"                             3       3       3
"{G}"                             1       1       1
"{X}"                             0       0       0
"{X}{G}"                          1       1       1
"{G/W}"                           1       1       0  <== OLD WRONG
"{2/W}"                           2       2       2
"{W/P}"                           1       1       0  <== OLD WRONG
"{S}"                             1       1       1
"{C}"                             1       1       1
"{C/P}"                           1       1       0  <== OLD WRONG
"{S}{S}{2}"                       4       4       4
"{2}{G}{G/W}{W/P}{X}{S}"          6       6       4  <== OLD WRONG
"{U}{U}{1}"                       3       3       3
"{B/P}{B/P}"                      2       2       0  <== OLD WRONG
"{G/W}{G/W}{2/W}"                 4       4       2  <== OLD WRONG
""                                0       0       0
"{0}"                             0       0       0

new mismatches = 0, old mismatches = 6
```

Every shape named in the task is right on the repaired code: `{3}`=3, `{G}`=1, `{X}`=0, `{G/W}`=1, `{2/W}`=2, `{W/P}`=1, `{S}`=1, `{C}`=1, `{C/P}`=1, mixed strings correct, empty string 0. The previously-reported defect (`{G/W}`/`{W/P}` → 0) is fixed; `{C/P}` and multi-symbol mixes, which the review did not originally enumerate, are correct too.

Two anti-tautology checks on the algorithm itself: `skip(1)` correctly drops the empty prefix of `split('{')` so a cost that does not start with `{` cannot be silently counted, and `parse::<u8>` is tried before the `X` test so `{X}` cannot be mistaken for a generic.

## 3. Commands run in the worktree (actual result lines)

```
$ cargo test -p aperture-core --test issue17_slice22 --locked
running 9 tests
test mana_value_counts_the_symbol_shapes_the_pool_does_not_print_yet ... ok
...
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

$ cargo fmt --all --check
(no output)  fmt exit=0
```

## 4. Would the new test fail against the pre-repair code?

Yes. Same probe, replaying the new test's assertions against the `c0e0a69` predicate:

```
old_mana_value("{G/W}") = 0 vs assert_eq!(1) -> FAIL
old_mana_value("{W/P}") = 0 vs assert_eq!(1) -> FAIL
old_mana_value("{2/W}") = 2 vs assert_eq!(2) -> PASS
old_mana_value("{X}{G}") = 1 vs assert_eq!(1) -> PASS
old_mana_value("{S}{S}{2}") = 4 vs assert_eq!(4) -> PASS
old_mana_value("") = 0 vs assert_eq!(0) -> PASS
```

The test dies on its **first** assertion (`{G/W}`), i.e. it is a genuine regression test for the finding, not decoration. A related observation: in the pre-repair code the bug was strictly under-counting numeric-less hybrid/Phyrexian symbols, so the finding could only ever have surfaced as a silent lower mana value on the first such card added — the shape-only test closes that gap without inventing fixture cards.

## 5. The repair did not move the reviewed slice-22 behaviour

```
$ cargo test -p aperture-core --test card_printings --locked
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

$ cargo test -p aperture-core --test catalog_pool --locked
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`mana_value_reads_the_printed_cost` (the pre-existing pool-level test: Sol Ring 1, lands 0) still passes inside the slice-22 run, so the extraction did not change any currently-printed card's value.

## 6. CI on the reviewed head

```
$ gh pr view 46 --json headRefOid,headRefName,state
{"headRefName":"aperture/issue-17","headRefOid":"89247d66dd0de8a4923aa6bf3537326ac22d2509","state":"OPEN"}
```

The PR head equals the reviewed SHA, so CI is keyed to the reviewed code.

```
$ gh pr checks 46
rust (macos-latest)     pending   0   .../runs/40464995565/job/122058148591
rust (ubuntu-latest)    pending   0   .../runs/40464995565/job/122058148217
rust (windows-latest)   pending   0   .../runs/40464995565/job/122058148485
windows package         pending   0   .../runs/40464995565/job/122058148495
board and roadmap       pass      8s  .../runs/40464995565/job/122058148512
rust (macos-latest)     pending   0   .../runs/40464989485/job/122058130723
rust (ubuntu-latest)    pending   0   .../runs/40464989485/job/122058130607
rust (windows-latest)   pending   0   .../runs/40464989485/job/122058130716
windows package         pending   0   .../runs/40464989485/job/122058130670
board and roadmap       pass      7s  .../runs/40464989485/job/122058130349
```

Both runs are keyed to the reviewed head — run `40464995565` (`event=pull_request`) and run `40464989485` (`event=push`), both `head_sha=89247d66dd0de8a4923aa6bf3537326ac22d2509`, both `status=in_progress`, `conclusion=null` at 2026-10-27T07:07Z. `board and roadmap` is green on both; **`rust (ubuntu|macos|windows-latest)` and `windows package` are still running on both runs. The reviewed head's CI is therefore NOT green — it is incomplete.** This verdict approves the code on local evidence only and does not assert CI success.

`mergeStateStatus` was `BEHIND` at the time of review; that is PR housekeeping, not a property of this delta.

## 7. Verdict

The repair is minimal (one-character semantic change plus an extraction), correctly implements CR 202.3 for every symbol shape asked about, exposes the rule so it can be tested without fake fixtures, and adds a test that provably fails on the pre-repair code. The previously-approved slice-22 behaviour is unchanged, judged by the two gate suites plus the pre-existing pool-level mana-value test.

**VERDICT: APPROVED**

### Remaining findings (none blocking)

1. Non-blocking, test strength: the pool-wide loop in the new test compares `def.mana_value()` to `cost_mana_value(def.cost)` — since `mana_value` now *is* `cost_mana_value`, that loop only proves the delegation wiring, not independent agreement. The symbol-shape assertions carry the real content, and they are correct, so this is cosmetic. Reproduce the tautology: `grep -n "self.cost.map_or(0, cost_mana_value)" crates/core/src/card.rs`.
2. Non-blocking, doc accuracy: the `{X}` doc note says "no X-cost spell in the pool is a creature". The pool in fact has **no** cost string containing `{X}` at all (`grep -rn '{X}' crates/core/src/catalog.rs` → 0 hits), so the statement is true but weaker than reality. No action needed.
3. Informational, out of pool: generic symbols above `u8::MAX` (`{300}`) would fail `parse::<u8>` and fall through to 1, and a cost summing above 255 would overflow the `u8` `sum()` (panic in debug). No printed Magic symbol or supported cost reaches either, and this was already true before the repair.

### Unverified (explicit)

- **CI on the reviewed head** — `rust` matrix (3 OS) and `windows package` were `in_progress` on both runs when this verdict was written; not observed green.
- **`cargo test --workspace`** — not re-run by this reviewer (out of scope by instruction; the author reports it green). Only the three named suites were executed locally.
- **`cargo clippy`** — not run in this re-adjudication; the delta adds a `pub` item only.
- **Runtime behaviour on real hybrid/Phyrexian cards** — none exist in the pool, so `cost_mana_value` is exercised by unit assertions only, not by a game.
- **CI on `c0e0a69`** — unchanged by this delta; not re-queried.
- The unrelated dirty/untracked files already present in the worktree before this review (`git status --porcelain`: session logs, `.starter-run.yml`, `evidence/issue17/slice17..21` artifacts) were not touched and are not part of the commit; they are pre-existing workspace state, not authored by this delta.
