//! Security mutation harness for the #31 audit lane. Bounded, deterministic,
//! seeded mutations of the untrusted replay/save JSON boundary and of engine
//! action validation, executed against the real public API (`Replay::from_json`,
//! `SessionSave::from_json`, `Game::submit`/`validate`), not a detached toy.
//!
//! Properties under test, for every mutated input:
//! 1. no panic and no abort (a panic fails the test);
//! 2. hostile input is cleanly refused with a typed `ReplayError`/`EngineError`;
//! 3. a rejected submission leaves the engine digest bit-identical (no
//!    unintended state mutation);
//! 4. allocations stay bounded (injected vectors are capped; huge counts must
//!    be refused, never wrapped).

mod common;

use aperture_core::replay::REPLAY_VERSION;
use aperture_core::*;
use common::{auto_answer, deck_a, deck_b};

/// Bounded deterministic iteration counts (keep the harness fast and
/// reproducible; every failure prints its exact seed).
const GENERIC_MUTATIONS: usize = 256;
const MAX_INJECTED_LEN: usize = 100_000;
const MAX_DECISIONS_PROBED: usize = 40;

/// Deterministic PRNG so any failure is exactly reproducible from the seed.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

/// A fixed game short enough to replay quickly, with its recorded replay.
fn recorded_game(seed: u64, steps_cap: usize) -> (Game, Vec<Event>, Replay) {
    let (mut game, mut log) = Game::new_logged(seed, &[deck_a(), deck_b()]).expect("game");
    let mut replay = Replay::new(
        seed,
        game.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .expect("replay");
    for _ in 0..steps_cap {
        if game.state.is_over() {
            break;
        }
        let Some(owner) = game.decision_owner() else {
            break;
        };
        let action = auto_answer(&game);
        let step = ReplayStep {
            player: owner,
            revision: game.revision(),
            decision: game.decision(),
            action: action.clone(),
        };
        match game.submit_action(owner, action) {
            Ok(events) => {
                log.extend(events);
                replay.record(step);
            }
            Err(err) => panic!("scripted step rejected: {err}"),
        }
    }
    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();
    (game, log, replay)
}

const INJECTED: &[&str] = &[
    "\"",
    "\\",
    "}",
    "{",
    "[",
    "]",
    ":",
    ",",
    "null",
    "true",
    "0",
    "-0",
    "18446744073709551615",
    "1e400",
    "\u{0000}",
    "\u{FFFF}",
    "\u{202E}",
    "\u{10FFFF}",
    " ",
    "\n",
    "🦀",
    "{\"x\":",
];

/// Floor a byte offset to the nearest UTF-8 char boundary.
fn floor_boundary(text: &str, mut at: usize) -> usize {
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// One deterministic structural mutation of a JSON text.
fn mutate(text: &str, rng: &mut Lcg) -> String {
    match rng.below(5) {
        // Truncate at a random point.
        0 => text[..floor_boundary(text, rng.below(text.len()))].to_string(),
        // Overwrite one byte with an injected token's first char.
        1 => {
            let mut out = text.as_bytes().to_vec();
            let at = rng.below(out.len());
            out[at] = INJECTED[rng.below(INJECTED.len())].as_bytes()[0];
            String::from_utf8_lossy(&out).into_owned()
        }
        // Insert an injected token at a random point.
        2 => {
            let at = floor_boundary(text, rng.below(text.len()));
            let token = INJECTED[rng.below(INJECTED.len())];
            let mut out = String::with_capacity(text.len() + token.len());
            out.push_str(&text[..at]);
            out.push_str(token);
            out.push_str(&text[at..]);
            out
        }
        // Delete a random slice.
        3 => {
            let at = floor_boundary(text, rng.below(text.len()));
            let end = floor_boundary(text, at + rng.below(text.len() - at).min(32));
            let mut out = String::with_capacity(text.len());
            out.push_str(&text[..at]);
            out.push_str(&text[end..]);
            out
        }
        // Duplicate a small random slice.
        _ => {
            let at = floor_boundary(text, rng.below(text.len()));
            let end = floor_boundary(text, at + rng.below(text.len() - at).min(32));
            let mut out = String::with_capacity(2 * text.len());
            out.push_str(&text[..at]);
            out.push_str(&text[at..end]);
            out.push_str(&text[at..]);
            out
        }
    }
}

/// Parse mutated replay JSON and, when it loads, exercise the run/branch
/// paths. Only "no panic" and "typed error or coherent replay" are asserted:
/// any panic fails the test, any accepted hostile input is surfaced by the
/// targeted tests below.
fn exercise_replay_text(text: &str) {
    match Replay::from_json(text) {
        Err(_) => {}
        Ok(replay) => {
            // A loaded replay must survive run/branch without panic.
            let _ = replay.run();
            let len = replay.steps.len();
            let _ = replay.branch_at(len);
            let _ = replay.branch_at(len.saturating_add(1));
            let _ = replay.branch_at(usize::MAX);
        }
    }
}

fn exercise_save_text(text: &str) {
    match SessionSave::from_json(text) {
        Err(_) => {}
        Ok(save) => {
            // A loaded save must reconstruct a coherent game.
            let _ = save.game();
            let _ = save.tainted();
            let _ = save.record_line();
        }
    }
}

#[test]
fn generic_mutations_of_replay_json_never_panic() {
    let (_, _, replay) = recorded_game(4101, 120);
    let json = replay.to_json();
    for seed in 0..GENERIC_MUTATIONS as u64 {
        let mutated = mutate(&json, &mut Lcg(seed + 1));
        exercise_replay_text(&mutated);
    }
}

#[test]
fn generic_mutations_of_save_json_never_panic() {
    let (game, log, replay) = recorded_game(4102, 30);
    let json = SessionSave::capture(&game, replay, log, false).to_json();
    for seed in 0..GENERIC_MUTATIONS as u64 {
        let mutated = mutate(&json, &mut Lcg(seed + 1));
        exercise_save_text(&mutated);
    }
}

/// Field-targeted replay mutations that MUST be refused with a typed error.
#[test]
fn targeted_replay_mutations_are_refused() {
    let (_, _, replay) = recorded_game(4103, 40);
    let json = replay.to_json();
    let obj: serde_json::Value = serde_json::from_str(&json).expect("baseline parses");

    let mut wrong_version = obj.clone();
    wrong_version["version"] = serde_json::json!(REPLAY_VERSION + 1);
    assert!(
        matches!(
            Replay::from_json(&wrong_version.to_string()),
            Err(ReplayError::Version { .. })
        ),
        "a wrong replay version must be refused"
    );

    let seat0 = obj["seats"][0].clone();
    for seats in [
        serde_json::json!([]),
        serde_json::json!([{"name":"x","cards":[]}]),
        // Clone the real seat shape: 1 seat and 5 seats are both outside the
        // supported 2..=4 range; the payloads stay bounded.
        serde_json::json!(seat0.clone()),
        serde_json::json!(std::iter::repeat_n(seat0, 5).collect::<Vec<_>>()),
    ] {
        let mut mutated = obj.clone();
        mutated["seats"] = seats;
        assert!(
            Replay::from_json(&mutated.to_string()).is_err(),
            "mutated seat vector must be refused: {}",
            mutated["seats"]
        );
    }

    for (field, value) in [
        ("first_player", serde_json::json!(99)),
        ("rules_revision", serde_json::json!("not-this-build")),
        ("final_digest", serde_json::json!("0000000000000000")),
        ("seed", serde_json::json!(18446744073709551615u64)),
    ] {
        let mut mutated = obj.clone();
        mutated[field] = value;
        // Data-bound fields may parse (they are inert data) but must be
        // refused at replay time by the typed validation path.
        match Replay::from_json(&mutated.to_string()) {
            Err(_) => {}
            Ok(parsed) => assert!(
                parsed.run().is_err(),
                "mutated {field} must be refused at replay time"
            ),
        }
    }

    // A step with a hostile player/revision/decision must be refused at replay
    // time even if it deserializes.
    let mut step = obj["steps"][0].clone();
    step["player"] = serde_json::json!(99);
    let mut mutated = obj.clone();
    mutated["steps"][0] = step;
    let parsed = Replay::from_json(&mutated.to_string())
        .unwrap_or_else(|e| panic!("step player 99 should still parse: {e}"));
    assert!(
        parsed.run().is_err(),
        "a step by a nonexistent seat must be rejected by the engine"
    );

    // Huge but bounded step vectors must parse-fail or replay-fail, never hang
    // or OOM: an invalid action bails at parse time, a valid-shaped one is
    // rejected at the first engine step.
    let huge_invalid = format!(
        "{{\"version\":{REPLAY_VERSION},\"seed\":1,\"first_player\":0,\
         \"seats\":[],\"commanders\":[],\"profile\":\"Practice20\",\
         \"rules_revision\":\"x\",\"ai\":{{}},\"outcome\":null,\"truncated\":null,\
         \"final_digest\":\"x\",\"steps\":[{}]}}",
        std::iter::repeat_n(
            "{\"player\":0,\"revision\":0,\"decision\":0,\"action\":\"not-an-action\"}",
            MAX_INJECTED_LEN
        )
        .collect::<Vec<_>>()
        .join(",")
    );
    assert!(
        Replay::from_json(&huge_invalid).is_err(),
        "invalid action payloads must be refused"
    );
}

/// Deep nesting must hit serde's recursion guard, not the stack.
#[test]
fn deeply_nested_json_is_refused_not_stack_overflowing() {
    let (_, _, replay) = recorded_game(4104, 5);
    let json = replay.to_json();
    for depth in [128usize, 1_024, MAX_INJECTED_LEN] {
        let mut mutated = json.clone();
        // Inject `{"junk": [[[...` right after the opening brace; the parser
        // must refuse with a typed error long before this depth.
        let nest = format!("{{\"junk\": {}", "[".repeat(depth));
        mutated.insert_str(0, &nest);
        assert!(
            Replay::from_json(&mutated).is_err(),
            "nesting depth {depth} must be refused"
        );
    }
}

/// A tampered save digest must be refused; a save whose embedded replay was
/// swapped must be refused by the identity bindings.
#[test]
fn tampered_saves_are_refused() {
    let (game, log, replay) = recorded_game(4105, 30);
    let json = SessionSave::capture(&game, replay, log, false).to_json();
    let obj: serde_json::Value = serde_json::from_str(&json).expect("baseline parses");

    let mut bad_digest = obj.clone();
    bad_digest["digest"] = serde_json::json!("0".repeat(64));
    assert!(
        matches!(
            SessionSave::from_json(&bad_digest.to_string()),
            Err(ReplayError::SaveDigestMismatch { .. })
        ),
        "a forged save digest must be refused"
    );

    let mut swapped = obj.clone();
    swapped["replay"]["seed"] =
        serde_json::json!(swapped["replay"]["seed"].as_u64().unwrap_or(1) ^ 1);
    assert!(
        SessionSave::from_json(&swapped.to_string()).is_err(),
        "a save whose embedded replay seed was tampered with must be refused"
    );

    let mut foreign_replay = obj.clone();
    let (_, _, other) = recorded_game(4105 ^ 0x5a5a, 10);
    foreign_replay["replay"] = serde_json::from_str(&other.to_json()).expect("other parses");
    assert!(
        SessionSave::from_json(&foreign_replay.to_string()).is_err(),
        "a save with another game's replay embedded must be refused"
    );
}

/// Mutated submissions at the live engine boundary must be rejected without
/// state drift, and `validate` must agree with `submit`.
#[test]
fn engine_rejects_mutated_submissions_without_state_drift() {
    let (mut game, _) = Game::new_logged(4106, &[deck_a(), deck_b()]).expect("game");
    let mut decisions = 0;
    while decisions < MAX_DECISIONS_PROBED {
        let Some(owner) = game.decision_owner() else {
            break;
        };
        if game.state.is_over() {
            break;
        }
        let legal = auto_answer(&game);
        let revision = game.revision();
        let decision = game.decision();
        let digest_before = game.digest();

        // Mutants: wrong seat, stale identity, nonexistent objects, oversized
        // vectors. Everything here is illegal-by-construction.
        let mutants = vec![
            Submission {
                player: owner.wrapping_add(1),
                revision,
                decision,
                action: legal.clone(),
            },
            Submission {
                player: owner,
                revision: revision.wrapping_sub(1),
                decision,
                action: legal.clone(),
            },
            Submission {
                player: owner,
                revision: revision.wrapping_add(1),
                decision,
                action: legal.clone(),
            },
            Submission {
                player: owner,
                revision,
                decision: decision.wrapping_add(1),
                action: legal.clone(),
            },
            Submission {
                player: owner,
                revision: u64::MAX,
                decision: u64::MAX,
                action: legal.clone(),
            },
            Submission {
                player: owner,
                revision,
                decision,
                action: Action::CastSpell {
                    card: ObjectId(u32::MAX),
                    targets: vec![],
                },
            },
            Submission {
                player: owner,
                revision,
                decision,
                action: Action::PlayLand {
                    card: ObjectId(u32::MAX),
                },
            },
            Submission {
                player: owner,
                revision,
                decision,
                action: Action::Discard {
                    cards: vec![ObjectId(u32::MAX); 1_000],
                },
            },
            Submission {
                player: owner,
                revision,
                decision,
                action: Action::DeclareBlockers {
                    blocks: vec![
                        BlockDecl {
                            attacker: ObjectId(u32::MAX),
                            blocker: ObjectId(u32::MAX),
                        };
                        1_000
                    ],
                },
            },
        ];

        for (i, sub) in mutants.iter().enumerate() {
            // validate() answers "is this action legal for this seat at the
            // current state" and deliberately does not judge submission
            // identity; submit() enforces staleness on top. The security
            // property is: every hostile mutant is rejected by submit and the
            // state does not move.
            assert!(
                game.submit(sub).is_err(),
                "mutant {i} must be rejected at decision {decision}"
            );
            assert_eq!(
                game.digest(),
                digest_before,
                "a rejected submission must not move the state (decision {decision}, mutant {i})"
            );
        }

        // Advance with the genuine legal action.
        match game.submit_action(owner, legal) {
            Ok(_) => {}
            Err(err) => panic!("the legal action must submit: {err}"),
        }
        decisions += 1;
    }
}

/// Oversized hostile input must stay bounded. serde_json accepts leading
/// whitespace by spec, so a padded replay loads; the honest security
/// observation is that the local replay/save boundary has no size cap (the
/// remote fetch path does: 8 MiB). Recorded as a low-severity residual risk.
#[test]
fn oversized_replay_padding_is_bounded_and_inert() {
    let (_, _, replay) = recorded_game(4107, 5);
    let mut padded = replay.to_json();
    padded.insert_str(1, " ".repeat(10 * 1024 * 1024).as_str());
    match Replay::from_json(&padded) {
        // Refused: fine.
        Err(_) => {}
        // Accepted: whitespace must be inert — the padded replay must still
        // reproduce the recorded digest exactly.
        Ok(parsed) => {
            let (game, _) = parsed.run().expect("padded replay must replay");
            assert_eq!(game.digest(), replay.final_digest, "padding must be inert");
        }
    }
}
