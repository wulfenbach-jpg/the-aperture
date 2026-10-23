//! THE APERTURE headless CLI: offline self-play, replay verification, coaching
//! output, and real (bounded) Archidekt import.

mod commander_cmd;
mod decks;
mod harness_cmd;
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
    /// Play one deterministic 2-4 seat AI game and report the result.
    Selfplay {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Seat count when no explicit --deck paths are supplied (2..=4).
        #[arg(long)]
        seats: Option<usize>,
        /// One deck path per seat; repeat 2..=4 times. Defaults to the bundled
        /// practice decks (cycling A/B for extra seats).
        #[arg(long = "deck")]
        deck: Vec<PathBuf>,
        #[arg(long, default_value_t = 4000)]
        max_steps: usize,
        /// Wall-clock bound for the run, milliseconds.
        #[arg(long)]
        max_millis: Option<u64>,
        /// Use the local Decider-2B service for this seat (0..=3).
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
        /// Explicit command-zone practice profile (20 life, one supported
        /// commander per seat, NOT full Commander).
        #[arg(long = "command-zone-practice")]
        command_zone_practice: bool,
        /// One `SEAT:CARD` commander spec per seat; requires
        /// --command-zone-practice. Example: `0:Isamaru, Hound of Konda`.
        #[arg(long = "commander")]
        commander: Vec<String>,
    },
    /// Run bounded 2/3/4-seat headless games with paired seeds and verify every
    /// replay digest. Exit 0 all terminal, 2 capped but no failure, 1 failure.
    Harness {
        /// Comma-separated seat counts (default 2,3,4).
        #[arg(long, default_value = "2,3,4")]
        seats: String,
        #[arg(long, default_value_t = 1)]
        seed_base: u64,
        #[arg(long, default_value_t = 4)]
        games_per_seat: usize,
        #[arg(long, default_value_t = 4000)]
        max_steps: usize,
        #[arg(long)]
        max_millis: Option<u64>,
        #[arg(long, default_value = "evidence/m1-harness.json")]
        out: PathBuf,
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
        #[arg(long)]
        deck_a: Option<PathBuf>,
        #[arg(long)]
        deck_b: Option<PathBuf>,
    },
    /// Analyze an offline Commander deck snapshot. Format legality only; never
    /// starts a game. Exit 0 legal, 1 illegal, 2 not_certified/input error.
    CommanderCheck {
        #[arg(long)]
        deck: PathBuf,
        #[arg(long)]
        cards: PathBuf,
        /// Print the versioned report JSON.
        #[arg(long)]
        json: bool,
        /// Write the report JSON here (same bytes as --json stdout).
        #[arg(long)]
        out: Option<PathBuf>,
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
            seats,
            deck,
            max_steps,
            max_millis,
            decider_seat,
            decider_timeout_ms,
            out,
            replay_out,
            verbose,
            command_zone_practice,
            commander,
        } => {
            if command_zone_practice != !commander.is_empty() {
                return Err(anyhow::anyhow!(
                    "--command-zone-practice needs exactly one --commander SEAT:CARD per seat, and --commander cannot be used without the profile flag"
                ));
            }
            let mut commanders: Vec<(aperture_core::PlayerId, aperture_core::CardId)> = Vec::new();
            for spec in &commander {
                let (seat, name) = spec.split_once(':').ok_or_else(|| {
                    anyhow::anyhow!("--commander must be SEAT:CARD, got '{spec}'")
                })?;
                let seat: u8 = seat.trim().parse().map_err(|_| {
                    anyhow::anyhow!("--commander seat must be a number, got '{seat}'")
                })?;
                let name = name.trim();
                let card = aperture_core::catalog::by_name(name)
                    .ok_or_else(|| anyhow::anyhow!("'{name}' is not a supported card name"))?;
                if !card.is_supported_commander() {
                    return Err(anyhow::anyhow!(
                        "{} is not in the supported command-zone practice commander profile; multi-commander variants are unsupported",
                        card.name()
                    ));
                }
                if commanders.iter().any(|(existing, _)| *existing == seat) {
                    return Err(anyhow::anyhow!(
                        "P{seat} has more than one --commander; multi-commander variants are unsupported"
                    ));
                }
                commanders.push((seat, card));
            }
            let deck_explicit = !deck.is_empty();
            let mut decks = deck;
            if decks.is_empty() {
                let seat_count = seats.unwrap_or(2);
                let paths = [
                    PathBuf::from("decks/temur-beats.txt"),
                    PathBuf::from("decks/izzet-control.txt"),
                ];
                for index in 0..seat_count {
                    decks.push(paths[index % 2].clone());
                }
            } else if let Some(seat_count) = seats
                && seat_count != decks.len()
            {
                return Err(anyhow::anyhow!(
                    "--seats {seat_count} needs exactly {seat_count} --deck path(s), got {}",
                    decks.len()
                ));
            }
            selfplay_cmd::run(selfplay_cmd::Args {
                seed,
                decks,
                max_steps,
                max_millis,
                decider_seat,
                decider_timeout_ms,
                out,
                replay_out,
                verbose,
                commanders,
                deck_explicit,
            })
        }
        Command::Harness {
            seats,
            seed_base,
            games_per_seat,
            max_steps,
            max_millis,
            out,
        } => {
            let seats: Vec<usize> = seats
                .split(',')
                .map(|part| part.trim().parse::<usize>())
                .collect::<Result<_, _>>()
                .map_err(|_| anyhow::anyhow!("--seats must be comma-separated integers"))?;
            harness_cmd::run(harness_cmd::Args {
                seats,
                seed_base,
                games_per_seat,
                max_steps,
                max_millis,
                out,
            })
        }
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
        } => info_cmd::coach(seed, deck_a.as_deref(), deck_b.as_deref()),
        Command::CommanderCheck {
            deck,
            cards,
            json,
            out,
        } => {
            std::process::exit(commander_cmd::run(&deck, &cards, json, out.as_deref()));
        }
        Command::DeciderHealth { base_url } => import_cmd::decider_health(&base_url),
    }
}
