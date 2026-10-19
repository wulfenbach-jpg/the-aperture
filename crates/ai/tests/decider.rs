//! The optional local Decider adapter: request shape, masking, and fallback.

mod common;

use aperture_ai::*;
use aperture_core::*;
use common::*;

/// A port nothing is listening on, so the adapter's failure path is exercised
/// without touching any service.
fn dead_port_config() -> DeciderConfig {
    DeciderConfig {
        base_url: "http://127.0.0.1:9".to_string(),
        model: "decider-2b".to_string(),
        timeout_ms: 700,
        max_len: 512,
    }
}

#[test]
fn the_request_uses_the_documented_endpoint_shape_and_masks_hidden_cards() {
    let game = fixture(20)
        .hand(P0, &[CardId::Shock, CardId::Mountain])
        .battlefield(P0, &[CardId::Mountain])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .hand(P1, &[CardId::Counterspell])
        .library(P0, &[CardId::Forest])
        .library(P1, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let view = common::view(&game, P0);
    let request = decision_request(&view).expect("a request is built");
    assert!(!request.options.is_empty());
    for option in &request.options {
        assert!(!option.key.is_empty());
        assert!(!option.label.is_empty());
    }
    let state = serde_json::to_string(&request.state).unwrap();
    let hidden: Vec<ObjectId> = game
        .state
        .player(P0)
        .library
        .iter()
        .chain(game.state.player(P1).library.iter())
        .chain(game.state.player(P1).hand.iter())
        .copied()
        .collect();
    for id in hidden {
        assert!(
            !state.contains(&format!("\"id\":{}", id.0)),
            "hidden object {} leaked into the request",
            id.0
        );
    }
    assert!(!state.contains("Counterspell"), "opponent hand leaked");
}

#[test]
fn an_unreachable_service_falls_back_to_the_heuristic_with_a_visible_reason() {
    let game = fixture(21)
        .hand(P0, &[CardId::GrizzlyBears])
        .battlefield(P0, &[CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let view = common::view(&game, P0);
    let mut heuristic = HeuristicPolicy::new();
    let expected = heuristic.decide(&view);

    let mut policy = DeciderPolicy::new(dead_port_config()).expect("config is loopback only");
    let decision = policy.decide(&view);
    assert_eq!(decision.source, DecisionSource::DeciderFallback);
    assert_eq!(
        decision.action, expected.action,
        "the baseline still decides"
    );
    let note = decision.note.clone().unwrap_or_default();
    assert!(!note.is_empty(), "the fallback reason must be visible");
}

#[test]
fn a_single_legal_option_is_forced_and_never_asked_of_the_model() {
    // The live typed readout rejects a 1-option choice with HTTP 422, so the
    // adapter must not send one.
    // No lands means no mana ability and an unaffordable spell: passing is the
    // only legal action.
    let game = fixture(22)
        .hand(P0, &[CardId::Divination])
        .main(P0, Step::PrecombatMain)
        .build();
    let view = common::view(&game, P0);
    assert_eq!(view.legal.len(), 1, "only passing is legal here");
    assert!(matches!(
        decision_request(&view),
        Err(DeciderError::TooFewOptions(1))
    ));

    let mut policy = DeciderPolicy::new(dead_port_config()).expect("config");
    let decision = policy.decide(&view);
    assert_eq!(decision.source, DecisionSource::Forced);
    assert_eq!(decision.action, Action::Pass);
    assert!(
        decision.note.unwrap_or_default().contains("forced"),
        "the reason must be visible"
    );
}

#[test]
fn only_loopback_http_endpoints_are_accepted() {
    for bad in [
        "https://127.0.0.1:8765",
        "http://example.com:8765",
        "http://169.254.169.254",
        "http://user:pw@127.0.0.1:8765",
        "http://0.0.0.0:8765",
    ] {
        let config = DeciderConfig {
            base_url: bad.to_string(),
            ..DeciderConfig::default()
        };
        assert!(
            DeciderClient::new(config).is_err(),
            "{bad} must be rejected"
        );
    }
    assert!(DeciderClient::new(dead_port_config()).is_ok());
    assert!(DeciderClient::new(DeciderConfig::default()).is_ok());
}
