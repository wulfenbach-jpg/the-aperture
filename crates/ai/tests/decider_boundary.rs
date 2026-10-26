//! Regression: a local model answer is applied only through a typed envelope
//! bound to the viewer, revision, decision serial and session generation, and
//! only when the returned key resolves to an action the engine offered. Every
//! loopback responder here is a labelled protocol fixture; no weights are run
//! and no inference is claimed.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use aperture_ai::{
    DeciderClient, DeciderConfig, DeciderError, DeciderPolicy, DecisionBinding, DecisionEnvelope,
    DecisionSource, HeuristicPolicy, Policy,
};
use aperture_core::view::PlayerView;
use aperture_core::*;

mod common;
use common::*;

fn read_full_request(stream: &mut TcpStream) -> bool {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    let header_end = loop {
        match stream.read(&mut chunk) {
            Ok(0) => return false,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
            }
            Err(_) => return false,
        }
    };
    let headers = String::from_utf8_lossy(&buf[..header_end]).to_ascii_lowercase();
    let len: usize = headers
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    while buf.len() < header_end + len {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(_) => break,
        }
    }
    true
}

/// Protocol fixture: this loopback server answers with a fixed typed-readout
/// document. It is not a model and its answer is not weight inference.
fn serve_once(body: String) -> (u16, Arc<AtomicBool>) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().unwrap().port();
    let got_request = Arc::new(AtomicBool::new(false));
    let flag = got_request.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            if !read_full_request(&mut stream) {
                continue;
            }
            flag.store(true, Ordering::SeqCst);
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (port, got_request)
}

fn config_for(port: u16) -> DeciderConfig {
    DeciderConfig {
        base_url: format!("http://127.0.0.1:{port}"),
        timeout_ms: 1_500,
        ..DeciderConfig::default()
    }
}

fn dead_policy() -> DeciderPolicy {
    DeciderPolicy::new(DeciderConfig {
        base_url: "http://127.0.0.1:9".to_string(),
        timeout_ms: 500,
        ..DeciderConfig::default()
    })
    .expect("loopback config")
}

fn healthy_body(choice: &str) -> String {
    format!(
        r#"{{"answers":{{"decision":{{"choice":"{choice}","answer_confidence":0.6,"confidence":0.2}}}},"usage":{{"truncated":false,"state_tokens_dropped":0,"truncated_questions":[],"input_tokens":100,"output_tokens":0}},"routing":{{"model":"protocol-fixture"}}}}"#
    )
}

fn truncated_body() -> String {
    r#"{"answers":{"decision":{"choice":"a","answer_confidence":0.6,"confidence":0.2}},"usage":{"input_tokens":100,"state_tokens_dropped":500,"truncated":true,"truncated_questions":["decision"]},"routing":{"model":"protocol-fixture"}}"#.to_string()
}

fn view_with(cards: &[CardId], lands: &[CardId], seed: u64) -> PlayerView {
    let game = fixture(seed)
        .hand(P0, cards)
        .battlefield(P0, lands)
        .main(P0, Step::PrecombatMain)
        .build();
    common::view(&game, P0)
}

fn one_option_view(seed: u64) -> PlayerView {
    let game = fixture(seed)
        .hand(P0, &[CardId::Divination])
        .main(P0, Step::PrecombatMain)
        .build();
    common::view(&game, P0)
}

fn binding() -> DecisionBinding {
    DecisionBinding {
        viewer: P0,
        revision: 7,
        decision: 3,
        generation: 2,
    }
}

#[test]
fn the_envelope_binds_the_viewer_revision_decision_generation_and_offers() {
    let view = view_with(
        &[CardId::GrizzlyBears],
        &[CardId::Forest, CardId::Forest],
        51,
    );
    let envelope = DecisionEnvelope::prepare(&view, binding()).expect("envelope");
    assert_eq!(envelope.binding(), binding());
    assert_eq!(
        envelope.offered(),
        view.legal
            .iter()
            .map(|legal| legal.action.clone())
            .collect::<Vec<_>>()
            .as_slice()
    );

    let wrong_seat = DecisionBinding {
        viewer: P1,
        ..binding()
    };
    let err = DecisionEnvelope::prepare(&view, wrong_seat)
        .expect_err("a binding for another seat is refused");
    assert_eq!(err.reason_code(), "wrong_seat");
}

#[test]
fn a_valid_protocol_fixture_answer_is_accepted_from_the_offered_snapshot() {
    let view = view_with(
        &[CardId::GrizzlyBears],
        &[CardId::Forest, CardId::Forest],
        52,
    );
    let envelope = DecisionEnvelope::prepare(&view, binding()).expect("envelope");
    let (port, served) = serve_once(healthy_body("a"));
    let mut policy = DeciderPolicy::new(config_for(port)).expect("loopback endpoint");
    let decision = policy.decide_bound(&view, &envelope, binding());
    assert_eq!(
        decision.source,
        DecisionSource::Decider,
        "note: {:?}",
        decision.note
    );
    assert_eq!(decision.action, view.legal[0].action);
    let preference = decision
        .preference
        .expect("the fixture reported confidence");
    assert!(
        (preference - 0.6).abs() < 1e-6,
        "preference is the reported concentration: {preference}"
    );
    assert!(
        decision
            .note
            .as_deref()
            .unwrap_or("")
            .contains("not a win probability"),
        "preference must stay labelled as concentration: {:?}",
        decision.note
    );
    assert!(served.load(Ordering::SeqCst), "the fixture was contacted");
}

#[test]
fn a_stale_envelope_falls_back_to_the_current_masked_heuristic() {
    let view = view_with(
        &[CardId::GrizzlyBears],
        &[CardId::Forest, CardId::Forest],
        53,
    );
    let envelope = DecisionEnvelope::prepare(&view, binding()).expect("envelope");
    let expected = HeuristicPolicy::new().decide(&view);
    let mut policy = dead_policy();

    let cases = [
        (
            DecisionBinding {
                generation: binding().generation + 1,
                ..binding()
            },
            "stale_generation",
        ),
        (
            DecisionBinding {
                revision: binding().revision + 1,
                ..binding()
            },
            "stale_revision",
        ),
        (
            DecisionBinding {
                decision: binding().decision + 1,
                ..binding()
            },
            "stale_decision",
        ),
        (
            DecisionBinding {
                viewer: P1,
                ..binding()
            },
            "wrong_seat",
        ),
    ];
    for (current, code) in cases {
        let decision = policy.decide_bound(&view, &envelope, current);
        assert_eq!(
            decision.source,
            DecisionSource::DeciderFallback,
            "{code} must fall back: {:?}",
            decision.note
        );
        assert_eq!(
            decision.action, expected.action,
            "{code} must fall back deterministically to the current legal view"
        );
        let note = decision.note.clone().unwrap_or_default();
        assert!(note.contains(code), "the visible note needs {code}: {note}");
        assert!(
            !note.contains("transport"),
            "a stale envelope is refused before any request: {note}"
        );
    }
}

#[test]
fn a_stale_forced_view_keeps_the_stale_reason_instead_of_reporting_forced() {
    let offered_view = view_with(
        &[CardId::GrizzlyBears],
        &[CardId::Forest, CardId::Forest],
        57,
    );
    let forced_view = one_option_view(58);
    assert_eq!(
        forced_view.legal.len(),
        1,
        "the fixture must force a single legal action"
    );
    let envelope = DecisionEnvelope::prepare(&offered_view, binding()).expect("envelope");
    let mut policy = dead_policy();

    for (current, code) in [
        (
            DecisionBinding {
                generation: binding().generation + 1,
                ..binding()
            },
            "stale_generation",
        ),
        (
            DecisionBinding {
                viewer: P1,
                ..binding()
            },
            "wrong_seat",
        ),
    ] {
        let decision = policy.decide_bound(&forced_view, &envelope, current);
        assert_eq!(
            decision.source,
            DecisionSource::DeciderFallback,
            "{code} must win over the forced choice: {:?}",
            decision.note
        );
        let note = decision.note.clone().unwrap_or_default();
        assert!(note.contains(code), "expected {code}: {note}");
        assert!(
            !note.contains("forced"),
            "the stale reason was lost: {note}"
        );
        assert!(
            !note.contains("transport"),
            "no request may be made for a stale envelope: {note}"
        );
        assert_eq!(decision.preference, None);
    }
}

#[test]
fn a_key_outside_the_current_engine_offer_is_refused_even_when_the_binding_matches() {
    // The envelope was built from a Forest position; the live view is an Island
    // position with the same binding values. The returned key resolves against
    // the original snapshot, but its action is not in the live offer, so it must
    // not be applied.
    let offered_view = view_with(
        &[CardId::GrizzlyBears],
        &[CardId::Forest, CardId::Forest],
        54,
    );
    let live_view = view_with(&[CardId::Divination], &[CardId::Island], 55);
    let envelope = DecisionEnvelope::prepare(&offered_view, binding()).expect("envelope");
    let index = envelope
        .offered()
        .iter()
        .position(|action| !live_view.legal.iter().any(|legal| legal.action == *action))
        .expect("the two positions must differ in at least one offered action");
    let key = envelope.request().options[index].key.clone();
    let (port, _served) = serve_once(healthy_body(&key));
    let mut policy = DeciderPolicy::new(config_for(port)).expect("loopback endpoint");

    let decision = policy.decide_bound(&live_view, &envelope, binding());
    assert_eq!(
        decision.source,
        DecisionSource::DeciderFallback,
        "an action outside the live offer must be refused: {:?}",
        decision.note
    );
    assert!(
        decision
            .note
            .as_deref()
            .unwrap_or("")
            .contains("illegal_choice"),
        "the refusal reason must be stable: {:?}",
        decision.note
    );
    assert_eq!(
        decision.action,
        HeuristicPolicy::new().decide(&live_view).action
    );
    assert_eq!(
        decision.preference, None,
        "the rejected proposal's concentration must not be attached to the heuristic action"
    );
}

#[test]
fn a_truncated_protocol_fixture_answer_falls_back_with_the_stable_reason_code() {
    let view = view_with(
        &[CardId::GrizzlyBears],
        &[CardId::Forest, CardId::Forest],
        56,
    );
    let envelope = DecisionEnvelope::prepare(&view, binding()).expect("envelope");
    let (port, _served) = serve_once(truncated_body());
    let mut policy = DeciderPolicy::new(config_for(port)).expect("loopback endpoint");
    let decision = policy.decide_bound(&view, &envelope, binding());
    assert_eq!(decision.source, DecisionSource::DeciderFallback);
    assert!(
        decision.note.as_deref().unwrap_or("").contains("truncated"),
        "note: {:?}",
        decision.note
    );
    assert_eq!(decision.action, HeuristicPolicy::new().decide(&view).action);
}

#[test]
fn reason_codes_are_stable_for_offline_and_protocol_failures() {
    assert_eq!(
        DeciderError::BadEndpoint("https://example.com".into()).reason_code(),
        "bad_endpoint"
    );
    assert_eq!(DeciderError::NoOptions.reason_code(), "no_options");
    assert_eq!(
        DeciderError::Truncated { detail: "x".into() }.reason_code(),
        "truncated"
    );
    assert_eq!(
        DeciderError::UnknownKey("z".into()).reason_code(),
        "unknown_key"
    );
    assert_eq!(
        DeciderError::IllegalChoice {
            detail: "not offered".into()
        }
        .reason_code(),
        "illegal_choice"
    );
    // Offline clients remain constructible and fail with transport, not panic.
    let client = DeciderClient::new(DeciderConfig {
        base_url: "http://127.0.0.1:9".to_string(),
        timeout_ms: 300,
        ..DeciderConfig::default()
    })
    .expect("loopback offline is usable");
    assert_eq!(client.config().timeout_ms, 300);
}
