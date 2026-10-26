//! End-to-end local-model boundary journey against a real engine.
//!
//! A loopback server answers with **protocol fixtures** (fixed typed-readout
//! documents); it is not a model and nothing here is weight inference. The
//! example proves, in order:
//! 1. a valid fixture answer is accepted through a bound envelope and the real
//!    engine applies the returned legal action;
//! 2. a truncated fixture answer is refused and the deterministic heuristic
//!    fallback is accepted by the same engine;
//! 3. a stale generation or revision envelope is refused without contacting the
//!    service and without mutating the engine.
//!
//! Run with: `cargo run -p aperture-ai --example model_decide`

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use aperture_ai::{
    DeciderConfig, DeciderPolicy, DecisionBinding, DecisionEnvelope, DecisionSource,
    HeuristicPolicy, Policy,
};
use aperture_core::*;

const SEED: u64 = 0xA11CE;
const GENERATION: u64 = 4;

fn deck_a() -> DeckList {
    DeckList::parse_practice(
        "Temur Beats",
        include_str!("../../../decks/temur-beats.txt"),
    )
    .expect("deck A parses")
}

fn deck_b() -> DeckList {
    DeckList::parse_practice(
        "Izzet Control",
        include_str!("../../../decks/izzet-control.txt"),
    )
    .expect("deck B parses")
}

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
        .find_map(|line| line.strip_prefix("content-length:"))
        .and_then(|value| value.trim().parse().ok())
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

/// One loopback protocol fixture serving a fixed queue of responses. Drop
/// wakes the accept loop and joins the thread, so the example exits cleanly.
struct FixtureServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl FixtureServer {
    fn start(bodies: Vec<String>) -> FixtureServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local address");
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let queue: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(bodies.into()));
        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(mut stream) = stream else { continue };
                if !read_full_request(&mut stream) {
                    continue;
                }
                let body = queue
                    .lock()
                    .expect("fixture queue")
                    .pop_front()
                    .unwrap_or_else(|| r#"{"error":"no fixture response queued"}"#.to_string());
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        FixtureServer {
            addr,
            stop,
            handle: Some(handle),
        }
    }

    fn client_config(&self) -> DeciderConfig {
        DeciderConfig {
            base_url: format!("http://{}", self.addr),
            timeout_ms: 1_500,
            ..DeciderConfig::default()
        }
    }
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocking accept; the loop then observes the stop flag.
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn valid_body() -> String {
    r#"{"answers":{"decision":{"type":"choice","choice":"a","answer_confidence":0.62,"confidence":0.18}},"usage":{"truncated":false,"state_tokens_dropped":0,"truncated_questions":[],"input_tokens":120,"output_tokens":0},"routing":{"model":"protocol-fixture"}}"#.to_string()
}

fn truncated_body() -> String {
    r#"{"answers":{"decision":{"type":"choice","choice":"a","answer_confidence":0.62,"confidence":0.18}},"usage":{"truncated":true,"state_tokens_dropped":300,"truncated_questions":["decision"],"input_tokens":120,"output_tokens":0},"routing":{"model":"protocol-fixture"}}"#.to_string()
}

fn fresh_game() -> Game {
    Game::new(SEED, &[deck_a(), deck_b()]).expect("practice game builds")
}

fn binding(game: &Game) -> DecisionBinding {
    DecisionBinding {
        viewer: P0,
        revision: game.revision(),
        decision: game.decision(),
        generation: GENERATION,
    }
}

fn submit(game: &mut Game, action: Action) {
    let submission = Submission {
        player: P0,
        revision: game.revision(),
        decision: game.decision(),
        action,
    };
    game.submit(&submission)
        .expect("the real engine accepts the decision");
}

fn main() {
    let server = FixtureServer::start(vec![valid_body(), truncated_body()]);
    let mut policy =
        DeciderPolicy::new(server.client_config()).expect("loopback fixture is usable");

    // 1. Valid protocol fixture -> bound envelope -> real engine.
    let mut game = fresh_game();
    let view = game.view_for(P0, &[]);
    let bound = binding(&game);
    let envelope = DecisionEnvelope::prepare(&view, bound).expect("envelope");
    let decision = policy.decide_bound(&view, &envelope, bound);
    assert_eq!(decision.source, DecisionSource::Decider);
    let chosen = decision.action.clone();
    submit(&mut game, chosen.clone());
    println!(
        "valid fixture: {} accepted by the engine -> {:?}; {} (revision {})",
        decision.source.label(),
        chosen,
        decision.note.as_deref().unwrap_or(""),
        game.revision()
    );

    // 2. Truncated fixture -> visible deterministic fallback -> real engine.
    let mut game = fresh_game();
    let view = game.view_for(P0, &[]);
    let bound = binding(&game);
    let envelope = DecisionEnvelope::prepare(&view, bound).expect("envelope");
    let decision = policy.decide_bound(&view, &envelope, bound);
    assert_eq!(decision.source, DecisionSource::DeciderFallback);
    assert!(
        decision.note.as_deref().unwrap_or("").contains("truncated"),
        "the refusal reason must be visible: {:?}",
        decision.note
    );
    let fallback = decision.action.clone();
    assert_eq!(
        fallback,
        HeuristicPolicy::new().decide(&view).action,
        "the fallback is the deterministic heuristic on the current masked view"
    );
    submit(&mut game, fallback.clone());
    println!(
        "truncated fixture: {} accepted by the engine -> {:?}; {} (revision {})",
        decision.source.label(),
        fallback,
        decision.note.as_deref().unwrap_or(""),
        game.revision()
    );

    // 3. Stale generation and revision -> refused before any request and
    //    without mutating the engine. The fixture queue is now empty, so a
    //    request would also visibly fail; the test asserts it never happens.
    let game = fresh_game();
    let view = game.view_for(P0, &[]);
    let bound = binding(&game);
    let envelope = DecisionEnvelope::prepare(&view, bound).expect("envelope");
    let digest = game.digest();
    let revision = game.revision();
    let cases = [
        (
            DecisionBinding {
                generation: bound.generation + 1,
                ..bound
            },
            "stale_generation",
        ),
        (
            DecisionBinding {
                revision: bound.revision + 1,
                ..bound
            },
            "stale_revision",
        ),
    ];
    for (current, code) in cases {
        let decision = policy.decide_bound(&view, &envelope, current);
        assert_eq!(decision.source, DecisionSource::DeciderFallback);
        assert!(
            decision.note.as_deref().unwrap_or("").contains(code),
            "expected {code}: {:?}",
            decision.note
        );
        assert_eq!(decision.action, HeuristicPolicy::new().decide(&view).action);
        assert_eq!(game.digest(), digest, "the engine must be unchanged");
        assert_eq!(game.revision(), revision, "the engine must be unchanged");
        println!(
            "stale envelope [{code}]: {} -> {:?}; engine unchanged at revision {}",
            decision.source.label(),
            decision.action,
            game.revision()
        );
    }

    println!(
        "\nAll answers came from labelled loopback protocol fixtures, not model weights. \
No strength or real-inference claim is made."
    );
}
