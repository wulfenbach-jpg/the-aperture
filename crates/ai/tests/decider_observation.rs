//! Regression: the masked model observation must carry every public zone the
//! viewer is allowed to see (exile, command zone, commander ledger, counters
//! and printed text) and must never claim a hardcoded 20-life profile.

mod common;

use aperture_ai::decision_request;
use aperture_core::*;
use common::*;

#[test]
fn the_request_exposes_public_exile_command_and_card_details() {
    let game = fixture(41)
        .hand(P0, &[CardId::GrizzlyBears])
        .battlefield(P0, &[CardId::Forest])
        .add(P0, Zone::Exile, CardId::Shock)
        .add(P1, Zone::Exile, CardId::Counterspell)
        .library(P1, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let view = common::view(&game, P0);
    let request = decision_request(&view).expect("a request is built");
    let state = serde_json::to_value(&request.state).expect("state serializes");

    assert_eq!(state["your_exile"][0]["name"], "Shock");
    assert_eq!(state["opponents"][0]["exile"][0]["name"], "Counterspell");
    assert!(
        state["your_battlefield"][0]["oracle"].is_string(),
        "printed text is the available keyword source: {state}"
    );
    assert_eq!(state["your_battlefield"][0]["plus_one_counters"], 0);
    assert_eq!(state["your_battlefield"][0]["damage"], 0);
    assert_eq!(state["command_zone_present"], false);
    assert_eq!(state["commanders"], serde_json::json!([]));
    assert_eq!(state["commander_damage"], serde_json::json!([]));
    assert_eq!(state["your_name"], "P0");
    assert!(state["opponents"][0]["deck_name"].is_string());
}

#[test]
fn the_observation_does_not_hardcode_a_twenty_life_game() {
    // 40 is a legal current life total; the projection reports the live total
    // and makes no claim about the starting-life profile.
    let game = fixture(42)
        .life(P0, 40)
        .life(P1, 40)
        .hand(P0, &[CardId::GrizzlyBears])
        .battlefield(P0, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let view = common::view(&game, P0);
    let request = decision_request(&view).expect("a request is built");
    let state = serde_json::to_value(&request.state).expect("state serializes");
    assert_eq!(state["life"]["P0"], 40);
    assert_eq!(state["life"]["P1"], 40);
    let text = serde_json::to_string(&request.state).expect("state serializes");
    assert!(
        !text.contains("20 life"),
        "the request must not assert a 20-life profile: {text}"
    );
}
