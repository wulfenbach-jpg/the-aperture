//! THE APERTURE — native desktop card table (eframe/egui).
//!
//! Run: `cargo run -p aperture-ui --release` (or the double-click launcher).
//! `--smoke out.png` renders the real native window, captures a screenshot of
//! the actual framebuffer and exits; it is used for automated visual evidence.

use std::path::PathBuf;

use aperture_ui::app::{ApertureApp, Startup};
use eframe::egui;

fn parse_options() -> Startup {
    let mut options = Startup {
        smoke: None,
        seed: 1,
        deck_a: PathBuf::from("decks/temur-beats.txt"),
        deck_b: PathBuf::from("decks/izzet-control.txt"),
        deck_a_explicit: false,
        deck_b_explicit: false,
        ai_seat: 1,
        human_autoplay: false,
        autoplay: false,
        use_decider: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--smoke" => options.smoke = args.next().map(PathBuf::from),
            "--seed" => {
                if let Some(value) = args.next().and_then(|v| v.parse().ok()) {
                    options.seed = value;
                }
            }
            "--deck-a" => {
                if let Some(value) = args.next() {
                    options.deck_a = PathBuf::from(value);
                    options.deck_a_explicit = true;
                }
            }
            "--deck-b" => {
                if let Some(value) = args.next() {
                    options.deck_b = PathBuf::from(value);
                    options.deck_b_explicit = true;
                }
            }
            "--ai-seat" => {
                if let Some(value) = args.next().and_then(|v| v.parse().ok()) {
                    options.ai_seat = value;
                }
            }
            "--autoplay" => options.autoplay = true,
            "--ai-both" => options.human_autoplay = true,
            "--decider" => options.use_decider = true,
            "--help" | "-h" => {
                println!(
                    "THE APERTURE — practice slice UI\n\
                     --seed N            deterministic game seed\n\
                     --deck-a PATH       deck for seat P0 (default decks/temur-beats.txt)\n\
                     --deck-b PATH       deck for seat P1 (default decks/izzet-control.txt)\n\
                     --ai-seat 0|1       which seat the baseline AI plays (default 1)\n\
                     --autoplay          let the AI keep playing without clicking\n\
                     --ai-both           both seats are played by the AI (watch mode)\n\
                     --decider           opt in to the local Decider-2B service (default off)\n\
                     --smoke OUT.png     render, screenshot the real window, exit\n"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    options
}

fn main() -> eframe::Result {
    let startup = parse_options();
    let smoke = startup.smoke.clone();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 940.0])
            .with_min_inner_size([1100.0, 760.0])
            .with_title("THE APERTURE — practice slice"),
        ..Default::default()
    };
    let result = eframe::run_native(
        "THE APERTURE",
        native_options,
        Box::new(move |cc| Ok(Box::new(ApertureApp::new(cc, startup)))),
    );
    if let Some(path) = smoke {
        println!("smoke screenshot written to {}", path.display());
    }
    result
}
