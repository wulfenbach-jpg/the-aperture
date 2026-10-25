//! THE APERTURE — native desktop card table (eframe/egui).
//!
//! Run: `cargo run -p aperture-ui --release` (or the double-click launcher).
//! `--smoke out.png` renders the real native window, captures a screenshot of
//! the actual framebuffer and exits; it is used for automated visual evidence.

use std::path::PathBuf;

use aperture_core::Profile;
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
        seats: 2,
        profile: Profile::Practice20,
        window: None,
        study_file: None,
        branch_at: 0,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--smoke" => options.smoke = args.next().map(PathBuf::from),
            "--study-file" => {
                options.study_file = args.next().map(|value| value.to_string());
            }
            "--branch-at" => {
                if let Some(value) = args.next().and_then(|v| v.parse().ok()) {
                    options.branch_at = value;
                }
            }
            "--seed" => {
                if let Some(value) = args.next().and_then(|v| v.parse().ok()) {
                    options.seed = value;
                }
            }
            "--seats" => {
                if let Some(value) = args.next().and_then(|v| v.parse().ok()) {
                    options.seats = value;
                }
            }
            "--profile" => {
                if let Some(value) = args.next() {
                    options.profile = match value.as_str() {
                        "ordinary" | "practice-20" | "20" => Profile::Practice20,
                        "bounded" | "commander-combat-practice-40" | "40" => {
                            Profile::CommanderCombat40
                        }
                        other => {
                            eprintln!(
                                "unknown profile '{other}'; use ordinary (20 life) or bounded (40 life)"
                            );
                            std::process::exit(2);
                        }
                    };
                }
            }
            "--size" => {
                if let Some(value) = args.next() {
                    let parsed = value
                        .split_once(['x', 'X'])
                        .and_then(|(w, h)| Some((w.parse::<f32>().ok()?, h.parse::<f32>().ok()?)));
                    match parsed {
                        Some((w, h)) if w >= 1024.0 && h >= 640.0 => options.window = Some((w, h)),
                        _ => {
                            eprintln!("--size expects e.g. 1280x720 (minimum 1024x640)");
                            std::process::exit(2);
                        }
                    }
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
                     --seats 2|3|4       native table size (default 2)\n\
                     --profile ordinary|bounded\n\
                                         ordinary = 20 life; bounded = 40 life with one\n\
                                         supported Isamaru per seat (practice, not Commander)\n\
                     --deck-a PATH       deck for seat P0 (default decks/temur-beats.txt)\n\
                     --deck-b PATH       deck for seat P1 (default decks/izzet-control.txt)\n\
                     --ai-seat 0|1       two-seat only: which seat the baseline AI plays (default 1)\n\
                     --autoplay          let the AI keep playing without clicking\n\
                     --ai-both           every seat is played by the AI (watch mode)\n\
                     --decider           opt in to the local Decider-2B service (default off)\n\
                     --size WxH          initial window size (default 1400x940; min 1024x640)\n\
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
    let (width, height) = startup.window.unwrap_or((1400.0, 940.0));
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width, height])
            .with_min_inner_size([1024.0, 640.0])
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
