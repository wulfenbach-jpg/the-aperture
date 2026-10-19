//! Regression: the local Decider request must label life by absolute seat, and
//! a response-indicated truncation must be refused (never silently accepted).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use aperture_ai::*;
use aperture_core::view::PlayerView;
use aperture_core::*;

mod common;
use common::*;

/// Read one complete HTTP request (headers + Content-Length body) so the server
/// never closes a connection with unread bytes (which would RST the response).
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

/// Minimal HTTP responder: answers every request with a fixed response. Each
/// connection serves one request and is then closed, so sequential client calls
/// get a fresh connection. Returns the bound port and a "was contacted" flag.
fn serve_once(response: String) -> (u16, Arc<AtomicBool>) {
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

fn response_for(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

fn config_for(port: u16) -> DeciderConfig {
    DeciderConfig {
        base_url: format!("http://127.0.0.1:{port}"),
        timeout_ms: 1_500,
        ..DeciderConfig::default()
    }
}

/// Viewer P1, active P1, P0 on 3 life, P1 on 17. P1 has a Forest on the
/// battlefield and Forest + Grizzly Bears in hand, so the main phase offers at
/// least the mana ability, PlayLand and Pass.
fn p1_view() -> PlayerView {
    let game = fixture(7)
        .life(P0, 3)
        .life(P1, 17)
        .battlefield(P1, &[CardId::Forest])
        .hand(P1, &[CardId::Forest, CardId::GrizzlyBears])
        .main(P1, Step::PrecombatMain)
        .build();
    common::view(&game, P1)
}

fn truncated_body() -> String {
    r#"{"answers":{"decision":{"type":"choice","choice":"a","probabilities":{"a":0.6,"b":0.4},"answer_confidence":0.6,"confidence":0.2}},"usage":{"input_tokens":100,"output_tokens":0,"state_tokens":9000,"state_tokens_dropped":500,"truncated":true,"truncated_questions":["decision"]},"routing":{"model":"decider-2b"}}"#
        .to_string()
}

fn healthy_body() -> String {
    r#"{"answers":{"decision":{"type":"choice","choice":"a","probabilities":{"a":0.6,"b":0.4},"answer_confidence":0.6,"confidence":0.2}},"usage":{"truncated":false,"state_tokens_dropped":0,"truncated_questions":[],"input_tokens":100,"output_tokens":0},"routing":{"model":"decider-2b"}}"#
        .to_string()
}

#[test]
fn the_life_object_is_keyed_by_absolute_seat_for_a_p1_viewer() {
    let view = p1_view();
    let request = decision_request(&view).expect("a request is built");
    assert!(
        request.options.len() >= 2,
        "the fixture must offer a real choice, got {} option(s)",
        request.options.len()
    );
    let state = serde_json::to_value(&request.state).expect("state serializes");
    assert_eq!(state["you_are"], "P1");
    assert_eq!(state["life"]["P1"], 17, "viewer P1's own life must be 17");
    assert_eq!(state["life"]["P0"], 3, "the opponent seat P0 must be 3");
}

#[test]
fn a_truncated_response_is_refused_with_a_truncated_error() {
    let (port, _served) = serve_once(response_for(&truncated_body()));
    let client = DeciderClient::new(config_for(port)).expect("loopback endpoint accepted");
    let view = p1_view();
    let request = decision_request(&view).expect("a request is built");
    let err = client
        .choose(&request)
        .expect_err("a truncated state must not produce a choice");
    assert!(
        matches!(err, DeciderError::Truncated { .. }),
        "expected a Truncated error, got: {err:?}"
    );
}

#[test]
fn a_truncated_response_makes_the_policy_fall_back_visibly() {
    let (port, _served) = serve_once(response_for(&truncated_body()));
    let mut policy = DeciderPolicy::new(config_for(port)).expect("loopback endpoint accepted");
    let view = p1_view();
    let decision = policy.decide(&view);
    assert_eq!(
        decision.source,
        DecisionSource::DeciderFallback,
        "a truncated answer must fall back, note: {:?}",
        decision.note
    );
    assert!(
        decision.note.as_deref().unwrap_or("").contains("truncat"),
        "the fallback note should mention truncation, got: {:?}",
        decision.note
    );
}

#[test]
fn a_healthy_response_is_used_by_the_client_and_the_policy() {
    let (port, _served) = serve_once(response_for(&healthy_body()));
    let view = p1_view();
    let request = decision_request(&view).expect("a request is built");
    let client = DeciderClient::new(config_for(port)).expect("loopback endpoint accepted");
    let answer = client
        .choose(&request)
        .expect("a healthy response is accepted");
    assert_eq!(answer.key, "a");

    // Second call to the same server, this time through the policy.
    let mut policy = DeciderPolicy::new(config_for(port)).expect("loopback endpoint accepted");
    let decision = policy.decide(&view);
    assert_eq!(
        decision.source,
        DecisionSource::Decider,
        "note: {:?}",
        decision.note
    );
    assert_eq!(decision.action, view.legal[0].action);
}

#[test]
fn the_default_state_budget_is_large_enough_for_the_masked_projection() {
    // The masked projection for a real position measured 566..696 state tokens
    // in a live probe. A 512-token budget made every request truncate, so the
    // adapter refused every answer and the model was never used. The default
    // must stay above the observed projection and inside the service bound.
    let budget = DeciderConfig::default().max_len;
    assert!(
        (1024..=2048).contains(&budget),
        "the default state budget must fit the masked projection: {budget}"
    );
}
