//! Regression: the local model client must not follow redirects and must not
//! route through an environment proxy. An endpoint that answers with a redirect
//! must not be able to send the masked payload to another target (review finding:
//! SSRF/egress beyond loopback).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use aperture_ai::{DeciderClient, DeciderConfig, decision_request};
use aperture_core::view::PlayerView;
use aperture_core::{CardId, P0, Step};

mod common;
use common::*;

/// Minimal HTTP responder: answers the first request with a fixed response, then
/// stops. Returns the bound port and a flag that the test can poll (bounded).
fn serve_once(response: String) -> (u16, Arc<AtomicBool>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().unwrap().port();
    let got_request = Arc::new(AtomicBool::new(false));
    let flag = got_request.clone();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut buffer = [0u8; 2048];
            let _ = stream.read(&mut buffer);
            flag.store(true, Ordering::SeqCst);
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (port, got_request)
}

fn view_of_a_real_position() -> PlayerView {
    let game = fixture(90)
        .hand(P0, &[CardId::GrizzlyBears])
        .battlefield(P0, &[CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    common::view(&game, P0)
}

#[test]
fn the_decider_client_does_not_follow_a_redirect() {
    // The "capture" server stands in for any private-network target the model
    // endpoint might try to redirect the payload to.
    let (capture_port, captured) = serve_once(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}"
            .to_string(),
    );
    let redirect = format!(
        "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{capture_port}/x\r\nContent-Length: 0\r\n\r\n"
    );
    let (port, served) = serve_once(redirect);

    let client = DeciderClient::new(DeciderConfig {
        base_url: format!("http://127.0.0.1:{port}"),
        timeout_ms: 1_500,
        ..DeciderConfig::default()
    })
    .expect("loopback endpoint is accepted");
    let view = view_of_a_real_position();
    let request = decision_request(&view).expect("a request is built");
    let result = client.choose(&request);

    // The endpoint did decide to redirect, so the client must treat that as a
    // failure instead of replaying the body somewhere else.
    assert!(
        result.is_err(),
        "a redirect must not be treated as a model answer: {result:?}"
    );
    // Give a hypothetical followed request a moment to arrive.
    for _ in 0..50 {
        if captured.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !captured.load(Ordering::SeqCst),
        "the client followed the redirect and leaked the payload to another port"
    );
    assert!(served.load(Ordering::SeqCst), "the endpoint was contacted");
}
