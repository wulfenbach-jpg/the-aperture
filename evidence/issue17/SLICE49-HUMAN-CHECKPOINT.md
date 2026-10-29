# Slice49 — three Owlbear Cub functional repairs

Same task/branch/draft PR46. Base `d86684508fc096ba9435c5277d9570820f0de395`;
product-and-journey commit **`d82eb0f5b227e97ec2cf360a6c3cf7346b8cf66a`**.
Any following commit is evidence only; it is not that code head and has no inherited
CI pass of its own. Native integration owner authored this increment and is **not**
its independent reviewer. Full issue17 acceptance is **false**.

## Changed behavior

- **R329-1:** the entitled controller now sees *all* examined cards, not only
  creature names in offers. Even an all-ineligible look suspends for private
  acknowledgement; a forged take is refused atomically. Whole opponent views and
  masked digests are identical across hidden eligible/all-ineligible libraries.
- **R329-2:** the eight-land restriction now qualifies the attack event. Cub's
  invented intervening-if classification and resolution recheck are removed.
  Eight lands at declaration, seven before resolution still performs the look;
  the existing seven-at-declaration refusal remains. The regression's land
  departure is a controlled valid fixture mutation, **not** a legal land-removal cast.
- **R329-3:** look mode binds to resolving Cub / triggered ability0 / controller /
  cursor1 / live defender; examined IDs are the complete exact min(8, size) prefix,
  eligible creatures exactly once, total selection capacity1. Titania's modal
  search guard runs before mode dispatch, so forged Cub mode cannot bypass it.
  Twelve malformed cases go through validate **and real SessionSave load**.
  Legitimate zero-eligible acknowledgement, empty/short looks and post-pick saved
  continuation remain valid and finish identically.

No UI, import, AI-search or model code changed. The existing saved writer history,
untracked reviewer directories and unrelated modified session JSONL are preserved.
The initial slice48 wrong claims are explicitly superseded, not erased from its logs.

## Actual execution

| Command / artifact | Result |
|---|---|
| `slice49/red-review329.log`, BEFORE product repair | compiling behavioral RED: 4 failed /1 positive control passed, exit101 |
| final focused test command in `slice49/gates-final.log` | 10 suites /76 passed /0 failed, exit0; slice49 adds8 tests |
| fmt / workspace-all-targets locked Clippy `-D warnings` | exit0 /exit0 |
| locked release CLI build | exit0 |
| new `slice49-journey-driver` | real4-seat /40-life terminal game, accepted actions only |
| separate release CLI `replay --file .../slice49/cub-replay.json` | digest matches,8658 events, exit0 |
| `slice49/corpus-audit.py` | catalog161; union459=148 implemented+311 missing; gaps55/0/51/48/5/52/52/53;1 certified, exit0 |

**Journey:** exact existing `decks/package-slice48.txt`40-card package, seed2323,
Win P2,3463 actions,95 turns,8658 events,
`acf204aa4711241f2c7c3124c3195d4f9ee93a6a471a25d5a8df56562e747ac9`.
12 Cub casts /5 looks /4 puts /5 bottom placements. Five completed private looks
project33 card samples; the driver's `done_only_acknowledgements=5` includes the
four post-pick Done decisions as well as the no-pick look, **not five all-ineligible
looks**. In-driver replay and fresh terminal save load agree, as does the separate
CLI replay. The extra acknowledgement changes the recording from slice48's3462
steps/8666 events/2b9f3b3d digest; old artifacts were not regenerated or overwritten.
This is an evidence-only demonstrator, not product AI, strength, or a100-card certificate.

The extra three control tests were added after the minimal repairs; they are
strengthened GREEN coverage, not retroactive test-first evidence. Retained
`expanded-controls.log` is my own initial test-API compile error (submit signature /
view digest field); `gates.log` is my mistaken nonexistent slice44 target. Neither
is a product defect or successful gate. Final successful receipt is `gates-final.log`.
A new local full-workspace test run is **not** claimed; CI supplies the required
cross-platform workflow gates. Final exact-code-head CI: **10/10 completed SUCCESS**,
every `head_sha` keyed to `d82eb0f`, 0 failed/pending; `gh pr checks 46` separately
read back all ten pass. Receipt: `slice49/exact-head-ci-d82eb0f.json`. No rerun was
requested by this run. A later evidence-only tip has no CI pass claim of its own.

## Rules correction and remaining gate

Fresh direct official pinned-rule retrieval matches the repository SHA (see
`slice49/rules-check.txt`). CR903.9a commander graveyard/exile return is an SBA
**after entry**: a commander sent to the graveyard dies. The inherited "never dies"
and replacement-effect instruction was incorrect; README/docs now withdraw it,
with no commander engine edit. CR506.5 distinguishes attacks-alone from
is-attacking-alone. Witness's shipped declaration-only reading is now visibly
an unresolved conformance gap before Ayula certification; this repair does not
change it or claim its correctness.

Original#17 remains **1/8 certified**. Ayula remains five cards short:
Dúnedain Rangers; Evercoat Ursine; Rampaging Yao Guai; Ruxa, Patient Professor;
Ursine Monstrosity. Exact100-card certification and separate four-seat replays
remain owed. Arbitrary Archidekt support is not claimed. Children t_26a7a4ff
(evaluation wait) and t_3c2fe51a (unassigned soak backlog) were inspected and left
gated; neither is an increment review child. No merge/master push/closure/unlock.

**Next:** independent concise practical review of `d866845..d82eb0f` and the
following evidence-only tip, focused on R329-1..3; then resume this same card /
branch / writer history / PR46 for the remaining Ayula work and genuine exact-list
certification. Do not complete the original issue or unlock dependents on repair approval.
