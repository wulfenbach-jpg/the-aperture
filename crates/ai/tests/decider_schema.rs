//! Regression: protocol responses are validated fail-closed. A missing or
//! malformed usage schema, a bad confidence, an oversized body, or a directly
//! constructed request with illegal cardinality/keys must never be treated as
//! a clean model answer. All servers here are local protocol fixtures; they do
//! not run or infer from model weights.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use aperture_ai::{
    DeciderClient, DeciderConfig, DeciderError, DecisionRequest, MAX_OPTIONS, OptionSpec,
    decision_request,
};
use aperture_core::view::PlayerView;
use aperture_core::*;
use serde_json::json;

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

fn dead_client() -> DeciderClient {
    DeciderClient::new(DeciderConfig {
        base_url: "http://127.0.0.1:9".to_string(),
        timeout_ms: 500,
        ..DeciderConfig::default()
    })
    .expect("loopback config")
}

fn p0_view() -> PlayerView {
    let game = fixture(43)
        .hand(P0, &[CardId::GrizzlyBears])
        .battlefield(P0, &[CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    common::view(&game, P0)
}

#[test]
fn missing_or_malformed_usage_is_never_treated_as_clean() {
    let bodies = [
        // No usage object at all.
        r#"{"answers":{"decision":{"choice":"a","answer_confidence":0.6}}}"#,
        // Wrong type for the truncation flag.
        r#"{"answers":{"decision":{"choice":"a","answer_confidence":0.6}},"usage":{"truncated":"no","state_tokens_dropped":0,"truncated_questions":[]}}"#,
        // Wrong type for the dropped-token count.
        r#"{"answers":{"decision":{"choice":"a","answer_confidence":0.6}},"usage":{"truncated":false,"state_tokens_dropped":"0","truncated_questions":[]}}"#,
        // Wrong type for the truncated-question list.
        r#"{"answers":{"decision":{"choice":"a","answer_confidence":0.6}},"usage":{"truncated":false,"state_tokens_dropped":0,"truncated_questions":"decision"}}"#,
        // Missing a required truncation field.
        r#"{"answers":{"decision":{"choice":"a","answer_confidence":0.6}},"usage":{"state_tokens_dropped":0,"truncated_questions":[]}}"#,
    ];
    for body in bodies {
        let (port, _served) = serve_once(response_for(body));
        let client = DeciderClient::new(config_for(port)).expect("loopback endpoint accepted");
        let request = decision_request(&p0_view()).expect("request");
        let err = client
            .choose(&request)
            .expect_err("a malformed usage schema must not be accepted");
        assert!(
            !matches!(err, DeciderError::Transport(_)),
            "the failure must be a protocol refusal, got {err:?} for {body}"
        );
    }
}

#[test]
fn invalid_confidence_is_refused_instead_of_defaulted_to_zero() {
    let bodies = [
        r#"{"answers":{"decision":{"choice":"a","answer_confidence":1.5,"confidence":0.2}},"usage":{"truncated":false,"state_tokens_dropped":0,"truncated_questions":[]}}"#,
        r#"{"answers":{"decision":{"choice":"a","answer_confidence":"high","confidence":0.2}},"usage":{"truncated":false,"state_tokens_dropped":0,"truncated_questions":[]}}"#,
        r#"{"answers":{"decision":{"choice":"a","answer_confidence":0.6,"confidence":-0.1}},"usage":{"truncated":false,"state_tokens_dropped":0,"truncated_questions":[]}}"#,
        r#"{"answers":{"decision":{"choice":"a","answer_confidence":0.6,"confidence":"bedrock"}},"usage":{"truncated":false,"state_tokens_dropped":0,"truncated_questions":[]}}"#,
    ];
    for body in bodies {
        let (port, _served) = serve_once(response_for(body));
        let client = DeciderClient::new(config_for(port)).expect("loopback endpoint accepted");
        let request = decision_request(&p0_view()).expect("request");
        let err = client
            .choose(&request)
            .expect_err("an invalid confidence must not be accepted");
        assert!(
            !matches!(err, DeciderError::Transport(_)),
            "the failure must be a protocol refusal, got {err:?} for {body}"
        );
    }
}

#[test]
fn an_oversized_protocol_response_body_is_refused() {
    // 300 KiB of valid JSON padding: the adapter must bound the read instead of
    // buffering whatever the local service sends.
    let padding = "x".repeat(300 * 1024);
    let body = format!(
        r#"{{"padding":"{padding}","answers":{{"decision":{{"choice":"a","answer_confidence":0.6}}}},"usage":{{"truncated":false,"state_tokens_dropped":0,"truncated_questions":[]}}}}"#
    );
    let (port, _served) = serve_once(response_for(&body));
    let client = DeciderClient::new(config_for(port)).expect("loopback endpoint accepted");
    let request = decision_request(&p0_view()).expect("request");
    client
        .choose(&request)
        .expect_err("an oversized response body must be refused");
}

#[test]
fn a_non_choice_decision_type_is_refused_as_malformed() {
    let body = r#"{"answers":{"decision":{"type":"score","choice":"a","answer_confidence":0.6}},"usage":{"truncated":false,"state_tokens_dropped":0,"truncated_questions":[]}}"#;
    let (port, _served) = serve_once(response_for(body));
    let client = DeciderClient::new(config_for(port)).expect("loopback endpoint accepted");
    let request = decision_request(&p0_view()).expect("request");
    let err = client
        .choose(&request)
        .expect_err("a score answer is not a supported choice");
    assert_eq!(err.reason_code(), "malformed_response");
    // Absent type stays accepted for installed-protocol compatibility.
    let (port, _served) = serve_once(response_for(
        r#"{"answers":{"decision":{"choice":"a","answer_confidence":0.6}},"usage":{"truncated":false,"state_tokens_dropped":0,"truncated_questions":[]}}"#,
    ));
    let client = DeciderClient::new(config_for(port)).expect("loopback endpoint accepted");
    client
        .choose(&request)
        .expect("an absent type remains accepted");
}

#[test]
fn choose_enforces_cardinality_and_unique_keys_for_directly_built_requests() {
    let client = dead_client();
    let state = json!({});
    let instructions = "fixture instructions".to_string();

    let one = DecisionRequest {
        state: state.clone(),
        instructions: instructions.clone(),
        options: vec![OptionSpec {
            key: "a".to_string(),
            label: "only".to_string(),
        }],
    };
    assert!(matches!(
        client.choose(&one),
        Err(DeciderError::TooFewOptions(1))
    ));

    let duplicate = DecisionRequest {
        state: state.clone(),
        instructions: instructions.clone(),
        options: vec![
            OptionSpec {
                key: "a".to_string(),
                label: "first".to_string(),
            },
            OptionSpec {
                key: "a".to_string(),
                label: "second".to_string(),
            },
        ],
    };
    let err = client
        .choose(&duplicate)
        .expect_err("duplicate option keys must be refused");
    assert!(
        !matches!(err, DeciderError::Transport(_)),
        "duplicate keys are refused before any request, got {err:?}"
    );

    let empty_key = DecisionRequest {
        state: state.clone(),
        instructions: instructions.clone(),
        options: vec![
            OptionSpec {
                key: String::new(),
                label: "first".to_string(),
            },
            OptionSpec {
                key: "b".to_string(),
                label: "second".to_string(),
            },
        ],
    };
    let err = client
        .choose(&empty_key)
        .expect_err("empty option keys must be refused");
    assert!(
        !matches!(err, DeciderError::Transport(_)),
        "empty keys are refused before any request, got {err:?}"
    );

    let too_many = DecisionRequest {
        state,
        instructions,
        options: (0..=MAX_OPTIONS)
            .map(|i| OptionSpec {
                key: format!("k{i}"),
                label: format!("option {i}"),
            })
            .collect(),
    };
    assert!(matches!(
        client.choose(&too_many),
        Err(DeciderError::TooManyOptions(_))
    ));
}
