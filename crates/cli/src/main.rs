//! THE APERTURE headless CLI: offline self-play, replay verification, coaching
//! output, and real (bounded) Archidekt import.

mod decks;
mod import_cmd;
mod info_cmd;
mod replay_cmd;
mod selfplay_cmd;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "aperture-cli",
    version,
    about = "THE APERTURE — practice slice CLI (supported card pool, 1v1, 20 life, NOT Commander)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Play one deterministic AI-vs-AI game and report the result.
    Selfplay {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value = "decks/temur-beats.txt")]
        deck_a: PathBuf,
        #[arg(long, default_value = "decks/izzet-control.txt")]
        deck_b: PathBuf,
        #[arg(long, default_value_t = 4000)]
        max_steps: usize,
        /// Use the local Decider-2B service for this seat (0 or 1).
        #[arg(long)]
        decider_seat: Option<u8>,
        #[arg(long, default_value_t = 4000)]
        decider_timeout_ms: u64,
        /// Write the machine-readable report here.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Also write the replay (with digest) here.
        #[arg(long)]
        replay_out: Option<PathBuf>,
        /// Print every event.
        #[arg(long)]
        verbose: bool,
    },
    /// Verify a save file or replay file: re-run it and compare digests.
    Replay {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        verbose: bool,
    },
    /// Import a public Archidekt deck by URL (allowlisted host only).
    Import {
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "cache")]
        cache: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        /// Attempt to plan the deck for play (refuses unsupported cards).
        #[arg(long)]
        plan: bool,
        /// Print the report JSON.
        #[arg(long)]
        json: bool,
    },
    /// Import a text deck list (the offline fallback).
    ImportTxt {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show the supported card pool.
    Pool,
    /// Show the bundled practice decks.
    Decks,
    /// Print the coaching explanation for the opening decision of a game.
    Coach {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value = "decks/temur-beats.txt")]
        deck_a: PathBuf,
        #[arg(long, default_value = "decks/izzet-control.txt")]
        deck_b: PathBuf,
    },
    /// Check whether the optional local Decider-2B service is reachable.
    DeciderHealth {
        #[arg(long, default_value = "http://127.0.0.1:8765")]
        base_url: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Selfplay {
            seed,
            deck_a,
            deck_b,
            max_steps,
            decider_seat,
            decider_timeout_ms,
            out,
            replay_out,
            verbose,
        } => selfplay_cmd::run(selfplay_cmd::Args {
            seed,
            deck_a,
            deck_b,
            max_steps,
            decider_seat,
            decider_timeout_ms,
            out,
            replay_out,
            verbose,
        }),
        Command::Replay { file, verbose } => replay_cmd::run(&file, verbose),
        Command::Import {
            url,
            cache,
            out,
            plan,
            json,
        } => import_cmd::import_url(&url, &cache, out.as_deref(), plan, json),
        Command::ImportTxt { file, name, json } => {
            import_cmd::import_text(&file, name.as_deref(), json)
        }
        Command::Pool => info_cmd::pool(),
        Command::Decks => info_cmd::decks(),
        Command::Coach {
            seed,
            deck_a,
            deck_b,
        } => info_cmd::coach(seed, &deck_a, &deck_b),
        Command::DeciderHealth { base_url } => import_cmd::decider_health(&base_url),
    }
}
