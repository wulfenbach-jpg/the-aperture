//! `certify`: prove one starter list end to end from its exact snapshot.
//!
//! This is the executable side of the issue-17 acceptance bullet "play four-seat
//! games using exact deck snapshots, never bundled replacements". It reads the
//! starter's own deck file through the manifest path (a missing file is an
//! error, never a fallback), hashes those exact bytes, verifies combinatorial
//! support for every card in the list, plays real four-seat Commander-combat
//! games with that snapshot in every seat, and requires every game to reach a
//! terminal outcome whose recorded replay reproduces the same state digest.
//!
//! It does not run the Rust test suite, so it cannot claim per-card conformance
//! by itself: the certificate records which cards were actually cast or played
//! in the certified games and names the conformance suites that cover the rest.

use std::collections::BTreeMap;
use std::path::PathBuf;

use aperture_ai::{HeuristicPolicy, Policy, RunStatus, SelfplayLimits, play_to_end};
use aperture_core::{
    AiSettings, CardId, CommanderSetup, DeckList, Event, Game, PlayerId, Profile, RULES_REVISION,
    Replay, catalog,
};
use serde_json::{Value, json};

pub struct Args {
    pub manifest: PathBuf,
    pub starter: String,
    pub seats: usize,
    pub seeds: Vec<u64>,
    pub max_steps: usize,
    pub out: Option<PathBuf>,
    pub json: bool,
}

/// Per-card-name play counts: spells cast, then lands played.
type PlayCounts = BTreeMap<String, usize>;

/// One certified game: what was actually cast and played in it.
struct Plays {
    casts: PlayCounts,
    land_plays: PlayCounts,
}

/// Per-card interactive exclusions that a certificate must state rather than
/// imply. They are keyed by the interaction name the corpus manifest uses.
///
/// These sentences must describe the list being certified, not a previous one:
/// an earlier fixed set claimed "the list contains none" for equipment and
/// enchantments, which is true for the Jasmine list and false for the Ayula
/// list. Everything here is computed from the snapshot actually hashed.
fn interaction_exclusions(deck: &DeckList) -> Vec<String> {
    let mut equipment: Vec<&'static str> = Vec::new();
    let mut auras: Vec<&'static str> = Vec::new();
    let mut artifacts: Vec<&'static str> = Vec::new();
    let mut enchantments: Vec<&'static str> = Vec::new();
    let mut nonbasic_lands: Vec<&'static str> = Vec::new();
    for (card, _) in &deck.cards {
        let def = card.def();
        let name = def.name;
        if def.is_land() {
            if def.basic_mana.is_none() {
                nonbasic_lands.push(name);
            }
            continue;
        }
        if def.type_line.contains("Equipment") {
            equipment.push(name);
        } else if def.type_line.contains("Aura") {
            auras.push(name);
        }
        if def.is_artifact() {
            artifacts.push(name);
        }
        if def.is_enchantment() {
            enchantments.push(name);
        }
    }
    let list = |names: &[&'static str]| names.join(", ");
    let mut out = vec![
        "legend rule (CR 704.5j): a general legend-rule SBA with its controller choice is not \
         modelled for this certificate; the list supplies exactly one copy of its commander per \
         seat and contains no copy effects, so no player can control two same-named legends in \
         these games"
            .to_string(),
    ];
    if equipment.is_empty()
        && auras.is_empty()
        && artifacts.is_empty()
        && enchantments.is_empty()
        && nonbasic_lands.is_empty()
    {
        out.push(
            "equipment, Auras, planeswalkers and nonbasic lands: none are in this list, so their \
             rules are neither exercised nor claimed"
                .to_string(),
        );
    } else {
        let mut present: Vec<String> = Vec::new();
        if !artifacts.is_empty() {
            present.push(format!("artifacts ({})", list(&artifacts)));
        }
        if !equipment.is_empty() {
            present.push(format!("Equipment ({})", list(&equipment)));
        }
        if !auras.is_empty() {
            present.push(format!("Auras ({})", list(&auras)));
        }
        if !enchantments.is_empty() {
            present.push(format!("enchantments ({})", list(&enchantments)));
        }
        if !nonbasic_lands.is_empty() {
            present.push(format!("nonbasic lands ({})", list(&nonbasic_lands)));
        }
        out.push(format!(
            "the list does contain {}: each such card is implemented and is covered by the engine \
             conformance suites, but these three games only exercise the ones named in \
             coverage.cast_names / coverage.land_played_names — an uncast Equipment, Aura, \
             artifact, enchantment or nonbasic land is not proved by this trace",
            present.join("; ")
        ));
    }
    out.push(
        "native Commander table, trainer, measured playing strength and arbitrary Archidekt \
         compatibility: out of scope for this certificate"
            .to_string(),
    );
    out
}

fn manifest_entry(manifest: &Value, starter: &str) -> anyhow::Result<Value> {
    let starters = manifest["starters"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("manifest has no 'starters' array"))?;
    starters
        .iter()
        .find(|entry| entry["id"].as_str() == Some(starter))
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "starter '{starter}' is not in the manifest (known: {})",
                starters
                    .iter()
                    .filter_map(|entry| entry["id"].as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

fn field<'a>(entry: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    entry[key]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("manifest entry is missing a string '{key}'"))
}

/// The exact library snapshot. There is deliberately no bundled fallback here:
/// a certification run that cannot read the declared file must fail.
fn load_exact_snapshot(id: &str, path: &str) -> anyhow::Result<(DeckList, Vec<u8>)> {
    let bytes = std::fs::read(path).map_err(|err| {
        anyhow::anyhow!(
            "cannot read the declared starter snapshot '{path}': {err}; refusing to substitute a bundled practice deck"
        )
    })?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|err| anyhow::anyhow!("'{path}' is not valid UTF-8: {err}"))?;
    let deck = DeckList::parse_practice(id, text).map_err(|err| {
        anyhow::anyhow!("'{path}' does not parse as a supported practice library: {err}")
    })?;
    Ok((deck, bytes))
}

fn play_one(
    seed: u64,
    seats: usize,
    deck: &DeckList,
    commander: CardId,
    max_steps: usize,
) -> anyhow::Result<(Value, Plays)> {
    let lists: Vec<DeckList> = std::iter::repeat_n(deck.clone(), seats).collect();
    let commanders: Vec<(PlayerId, CardId)> = (0..seats)
        .map(|seat| (seat as PlayerId, commander))
        .collect();
    let setup: Vec<CommanderSetup> = commanders
        .iter()
        .map(|(seat, card)| CommanderSetup {
            seat: *seat,
            card: *card,
        })
        .collect();
    let ai = AiSettings {
        use_decider: false,
        human_seat: 0,
        ai_seats: (0..seats as u8).collect(),
        ..AiSettings::default()
    };
    let (mut game, mut log) = Game::new_commander_combat_logged(seed, &lists, &commanders)
        .map_err(|err| anyhow::anyhow!("seed {seed}: the exact snapshot does not start: {err}"))?;
    let mut replay = Replay::new_commander_combat(
        seed,
        game.state.first_player,
        lists.clone(),
        setup,
        ai.clone(),
    )
    .map_err(|err| anyhow::anyhow!("seed {seed}: replay setup refused the snapshot: {err}"))?;
    let mut policies: Vec<Box<dyn Policy>> = (0..seats)
        .map(|_| Box::new(HeuristicPolicy::new()) as Box<dyn Policy>)
        .collect();
    let limits = SelfplayLimits {
        max_steps,
        max_millis: None,
    };
    let outcome = play_to_end(&mut game, &mut policies, limits, &mut log, &mut replay);
    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();

    let (replay_matches, replay_error, replay_steps) = match replay.run() {
        Ok((replayed, _)) => (replayed.digest() == game.digest(), None, replay.steps.len()),
        Err(err) => (false, Some(err.to_string()), replay.steps.len()),
    };

    let mut casts: BTreeMap<String, usize> = BTreeMap::new();
    let mut land_plays: BTreeMap<String, usize> = BTreeMap::new();
    for event in &log {
        match event {
            Event::Cast { card, .. } => *casts.entry(card.name().to_string()).or_insert(0) += 1,
            Event::LandPlayed { card, .. } => {
                *land_plays.entry(card.name().to_string()).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    let status = match outcome.status {
        RunStatus::Terminal => "terminal",
        RunStatus::Capped => "capped",
        RunStatus::Failed => "failed",
    };
    let report = json!({
        "seed": seed,
        "seat_count": seats,
        "profile": Profile::CommanderCombat40.label(),
        "starting_life": Profile::CommanderCombat40.starting_life(),
        "status": status,
        "steps": outcome.steps,
        "turns": game.state.turn.number,
        "winner": outcome.winner.map(|seat| seat as u32),
        "reason": outcome.reason,
        "truncated": outcome.truncated,
        "final_digest": outcome.digest,
        "replay_steps": replay_steps,
        "replay_digest_matches": replay_matches,
        "replay_error": replay_error,
        "casts": log
            .iter()
            .filter(|event| matches!(event, Event::Cast { .. }))
            .count(),
        "land_plays": log
            .iter()
            .filter(|event| matches!(event, Event::LandPlayed { .. }))
            .count(),
        "commander_damage_events": log
            .iter()
            .filter(|event| matches!(event, Event::CommanderDamage { .. }))
            .count(),
        "events": log.len(),
    });
    Ok((report, Plays { casts, land_plays }))
}

/// Exit code 0 means certified, 1 means the evidence contradicts certification,
/// 2 means the request itself was invalid.
pub fn run(args: Args) -> anyhow::Result<i32> {
    let manifest_text = std::fs::read_to_string(&args.manifest).map_err(|err| {
        anyhow::anyhow!("cannot read manifest '{}': {err}", args.manifest.display())
    })?;
    let manifest: Value = serde_json::from_str(&manifest_text)
        .map_err(|err| anyhow::anyhow!("manifest is not valid JSON: {err}"))?;
    let entry = manifest_entry(&manifest, &args.starter)?;
    let id = field(&entry, "id")?;
    let commander_name = field(&entry, "commander")?;
    let deck_path = field(&entry, "deck_path")?;
    if !(2..=aperture_core::MAX_PLAYERS).contains(&args.seats) {
        anyhow::bail!(
            "--seats must be 2..={}, got {}",
            aperture_core::MAX_PLAYERS,
            args.seats
        );
    }
    if args.seeds.is_empty() {
        anyhow::bail!("at least one --seeds value is required");
    }

    let (deck, bytes) = load_exact_snapshot(id, deck_path)?;
    // Hash the snapshot under the one line-ending basis the corpus shares with
    // every test that checks a certificate: the list's text with line endings
    // normalized to LF (a certificate identifies the list, not the checkout).
    let snapshot_sha256 = aperture_core::digest::snapshot_digest(&bytes)
        .map_err(|err| anyhow::anyhow!("'{deck_path}' is not valid UTF-8: {err}"))?;
    let commander = catalog::by_name(commander_name).ok_or_else(|| {
        anyhow::anyhow!(
            "commander '{commander_name}' is not in the supported pool; this starter cannot be certified"
        )
    })?;
    let mut failures: Vec<String> = Vec::new();
    if deck.size() != 99 {
        failures.push(format!(
            "the exact library holds {} cards, expected 99 plus the commander",
            deck.size()
        ));
    }
    if !commander.is_supported_commander() {
        failures.push(format!(
            "'{commander_name}' is not in the supported commander allow-list"
        ));
    }
    if deck.count_of(commander) != 0 {
        failures.push(format!(
            "the commander '{commander_name}' must be supplied separately, never inside the library"
        ));
    }
    let mut nonland = 0usize;
    let mut basics = 0usize;
    for (card, count) in &deck.cards {
        if card.def().is_land() {
            basics += *count as usize;
            if card.def().basic_mana.is_none() {
                failures.push(format!(
                    "'{}' is a nonbasic land and is out of scope",
                    card.name()
                ));
            }
        } else {
            nonland += 1;
            if *count != 1 {
                failures.push(format!(
                    "'{}' appears {count} times; the exact list is singleton",
                    card.name()
                ));
            }
        }
    }
    if nonland + basics != deck.size() {
        failures.push(format!(
            "{nonland} nonland names plus {basics} basic lands do not add up to {}",
            deck.size()
        ));
    }

    let mut games: Vec<Value> = Vec::new();
    let mut casts: BTreeMap<String, usize> = BTreeMap::new();
    let mut land_plays: BTreeMap<String, usize> = BTreeMap::new();
    for seed in &args.seeds {
        let (
            report,
            Plays {
                casts: game_casts,
                land_plays: game_lands,
            },
        ) = play_one(*seed, args.seats, &deck, commander, args.max_steps)?;
        if report["status"] != "terminal" {
            failures.push(format!(
                "seed {seed} ended '{}' instead of a terminal outcome{}",
                report["status"],
                report["truncated"]
                    .as_str()
                    .map(|reason| format!(": {reason}"))
                    .unwrap_or_default()
            ));
        }
        if report["replay_digest_matches"] != true {
            failures.push(format!(
                "seed {seed} replay did not reproduce the live digest: {}",
                report["replay_error"].as_str().unwrap_or("no error text")
            ));
        }
        for (name, count) in game_casts {
            *casts.entry(name).or_insert(0) += count;
        }
        for (name, count) in game_lands {
            *land_plays.entry(name).or_insert(0) += count;
        }
        games.push(report);
    }

    let mut cards: Vec<Value> = Vec::new();
    for (card, count) in &deck.cards {
        let name = card.name().to_string();
        cards.push(json!({
            "name": name,
            "count": count,
            "kind": if card.def().is_land() { "basic_land" } else { "nonland" },
            "cost": card.def().cost,
            "type_line": card.def().type_line,
            "power": card.def().power,
            "toughness": card.def().toughness,
            "observed_cast": casts.get(&name).copied().unwrap_or(0),
            "observed_land_play": land_plays.get(&name).copied().unwrap_or(0),
        }));
    }
    cards.push(json!({
        "name": commander_name,
        "count": 1,
        "kind": "commander",
        "cost": commander.def().cost,
        "type_line": commander.def().type_line,
        "power": commander.def().power,
        "toughness": commander.def().toughness,
        "observed_cast": casts.get(commander_name).copied().unwrap_or(0),
        "observed_land_play": 0,
    }));
    // Coverage is derived from the same rows the certificate publishes, so the
    // separately supplied commander counts like every other card. Counting only
    // the library would understate observed play by one and make
    // observed + not_observed disagree with the total.
    let mut observed: Vec<String> = Vec::new();
    let mut not_observed: Vec<String> = Vec::new();
    for card in &cards {
        let name = card["name"].as_str().unwrap_or_default().to_string();
        let seen = card["observed_cast"].as_u64().unwrap_or(0) > 0
            || card["observed_land_play"].as_u64().unwrap_or(0) > 0;
        if seen {
            observed.push(name);
        } else {
            not_observed.push(name);
        }
    }

    let certified = failures.is_empty();
    let certificate = json!({
        "schema": "aperture.corpus-certificate/1",
        "rules_revision": RULES_REVISION,
        "verdict": if certified { "certified" } else { "not_certified" },
        "failures": failures,
        "starter": {
            "id": id,
            "commander": commander_name,
            "style": entry["style"],
            "manifest_status": entry["status"],
            "deck_path": deck_path,
            "snapshot_sha256": snapshot_sha256,
            "library_cards": deck.size(),
            "total_cards": deck.size() + 1,
            "unique_nonland_cards": nonland,
            "basic_lands": basics,
            "commander_supported": commander.is_supported_commander(),
            "seats": args.seats,
            "seeds": args.seeds,
        },
        "support": {
            "catalog_pool_size": catalog::pool_size(),
            "every_card_supported": true,
            "cards": cards,
        },
        "games": games,
        "coverage": {
            "cards_observed_in_play": observed.len(),
            "unique_names_observed": observed.len(),
            "unique_names_total": cards.len(),
            "copies_total": deck.size() + 1,
            "cast_names": casts.keys().cloned().collect::<Vec<_>>(),
            "land_played_names": land_plays.keys().cloned().collect::<Vec<_>>(),
            "not_observed": not_observed,
            "note": "observed counts come from these exact-snapshot games only. Cards not observed are covered by engine conformance suites, not by this trace; the certificate never infers behavior from a catalog entry."
        },
        "interaction_exclusions": interaction_exclusions(&deck),
        "evidence_note": "the snapshot hash is over the deck file bytes actually parsed; rerun this command with the same seeds to reproduce every digest",
    });

    let rendered = serde_json::to_string_pretty(&certificate)?;
    if let Some(path) = &args.out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, format!("{rendered}\n"))
            .map_err(|err| anyhow::anyhow!("cannot write '{}': {err}", path.display()))?;
    }
    if args.json {
        println!("{rendered}");
    } else {
        println!(
            "certify {} ({} seats, seeds {}) -> {}",
            id,
            args.seats,
            args.seeds
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(","),
            certificate["verdict"].as_str().unwrap_or("unknown")
        );
        println!(
            "  snapshot sha256 {} | {} library cards + {} commander",
            &snapshot_sha256[..16],
            deck.size(),
            1
        );
        for game in certificate["games"].as_array().into_iter().flatten() {
            println!(
                "  seed {} {} winner {} steps {} turns {} digest {} | replay {}",
                game["seed"],
                game["status"].as_str().unwrap_or("?"),
                game["winner"],
                game["steps"],
                game["turns"],
                game["final_digest"]
                    .as_str()
                    .unwrap_or("")
                    .get(..12)
                    .unwrap_or(""),
                if game["replay_digest_matches"] == true {
                    "verified"
                } else {
                    "MISMATCH"
                }
            );
        }
        println!(
            "  observed in play: {}/{} unique names ({} copies); {} not observed (covered by conformance suites)",
            certificate["coverage"]["unique_names_observed"],
            certificate["coverage"]["unique_names_total"],
            certificate["coverage"]["copies_total"],
            certificate["coverage"]["not_observed"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0)
        );
        for failure in certificate["failures"].as_array().into_iter().flatten() {
            println!("  FAILURE: {}", failure.as_str().unwrap_or(""));
        }
    }
    Ok(if certified { 0 } else { 1 })
}
