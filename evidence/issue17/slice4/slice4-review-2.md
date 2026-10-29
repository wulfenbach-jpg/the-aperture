# Slice 4 review round 2 — R211-1 adjudication

**review_outcome: approve_increment**  
Reviewed head: `eea8f9dea34ae9b5f65c4bda3a1f50797449a118`  
Previous head: `86aebf9`  
`full_issue17_accepted: false`; #16 integration not approved.

R211-1 is genuinely fixed. `engine.rs` now has one shared
`queue_another_creature_enters(entering)` used by `enter_battlefield` for
creatures and `create_tokens` for creature tokens, excluding the entering
object. Re-running my probe with 10 Soul Warden / 10 Goblin Instigator / 10
Plains / 10 Mountain now logs a Soul Warden trigger immediately after
`creates Goblin Token token (#81)`, in addition to the Instigator entry.
Non-creature entries remain excluded by the `is_creature` guards in both
paths, and the Warden-never-triggers-for-itself test still passes.

## Regressions are genuine

- Token test: RED evidence `slice4-red-R211-1-token.log` fails when the hook is
  disabled; GREEN now 13/13.
- Two-blocker trample: asserts P1 life 18 and a 2-damage player event. Losing
  trample surplus changes both.
- Marked-damage trample: asserts P1 life 17 and a 3-damage player event.
  Ignoring marked damage would assign 2 lethal and leave life 18.

## No collateral change

- `cargo test --workspace --locked` — exit 0.
- `cargo fmt --all -- --check`; `git diff --check` — exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — exit 0.
- First journey (`package-ayula-slice4`, seed 17, four Jasmine seats) — terminal
  P1, digest `5e905fdc0399…`, replay matches; unchanged.
- Second journey (`package-soul-warden-tokens`, seed 23) — terminal P2, 1207
  steps / 30 turns, digest `11fd9618f1e9…`, replay matches; logs show Soul
  Warden triggers for Goblin tokens.
- Catalog 97; union 459 = 85 supported + 374 missing; manifest matches and
  desired-list files are unedited. Slice-4 source strings/flags unchanged.

Limitations: bounded increment only; seven desired lists remain unsupported;
no #16, release, GUI/import/search, historical matrix or playing-strength claim.
