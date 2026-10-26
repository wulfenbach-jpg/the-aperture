# Issue 17 slice 4 — bounded delta review

**review_outcome: request_changes**  
Reviewed head: `86aebf9b1acf298a8dd9e0f0c908d2a62bfe63bc`  
Base: `9cecd62ddb56d44e6853f9f9098fd3a473635b66`  
`full_issue17_accepted: false`; #16 integration not approved.

## Finding

**High — Soul Warden does not trigger for creature tokens.**  
`crates/core/src/engine.rs:1187` queues `AnotherCreatureEnters` watchers from
`enter_battlefield`, but `create_tokens` at `engine.rs:4482` inserts tokens
directly and queues only the token's own ETB trigger. A creature token is still
“another creature,” so Soul Warden must gain life for it.

Reproduction:
`aperture-cli selfplay --seed 1 --deck evidence/issue17/slice4/slice4-probe-warden.txt`
×2, `--max-steps 3000 --verbose`
(10 Soul Warden, 10 Goblin Instigator, 10 Plains, 10 Mountain).
Observed in `slice4-probe-warden-token.log`: Goblin Instigator entering queues
Soul Warden, then `creates Goblin Token token (#81)` occurs with **no** Soul
Warden trigger for the token.

Expected: the token queues Soul Warden's gain-1 trigger under each Warden's
controller, exactly as a nontoken creature does. Required repair: share the
`AnotherCreatureEnters` watcher step with token creation (excluding the entering
token itself) and add a token-entry regression.

## Checks

- `cargo test -p aperture-core --test issue17_slice4 --locked` — **10/10 passed**.
- `cargo test --workspace --locked` — **exit 0**.
- `cargo fmt --all -- --check`; `git diff --check` — exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — exit 0.
- Required journey: four exact `package-ayula-slice4.txt` seats, seed 17,
  commander-combat practice, Jasmine each seat — terminal P1, 1567 steps /
  43 turns, digest `5e905fdc0399…`; separate `replay --file` matches.
- Live Scryfall `hob/133`, `tsp/190`, `afr/198`, `mm3/24` match the fixture and
  catalog for cost, type, P/T and Oracle text; Flash/Trample flags match printed
  keywords.
- Trample assignment reads correctly: lethal per blocker (including marked
  damage), surplus to player, non-trample residue on last blocker. Multi-blocker
  and pre-damaged cases are code-consistent but not separately tested.
- Corpus: catalog 97; union 459 = 85 supported + 374 missing; manifest matches;
  seven planned, one implemented, zero certified; desired-list files unchanged.

## Limitations

Bounded increment only: seven desired starters remain unsupported; no #16,
release, merge, GUI/import/search, full historical matrix or playing-strength
claim.
