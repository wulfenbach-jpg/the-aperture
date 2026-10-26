//! Real CLI acceptance for `certify`: the issue-17 acceptance bullet "play
//! four-seat games using exact deck snapshots, never bundled replacements" is
//! proven by running the shipped binary, not by reading the handoff.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const BIN: &str = env!("CARGO_BIN_EXE_aperture-cli");

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .current_dir(repo_root())
        .args(args)
        .output()
        .expect("the CLI binary runs")
}

fn scratch(name: &str, contents: &str) -> PathBuf {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&path).expect("target dir");
    path.push(format!("certify-test-{name}"));
    std::fs::write(&path, contents).expect("fixture file writes");
    path
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn certify_proves_one_starter_from_its_exact_snapshot() {
    let output = run(&[
        "certify",
        "--starter",
        "jasmine-vanilla-stompy",
        "--seats",
        "4",
        "--seeds",
        "1",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "certification must pass: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let certificate: Value =
        serde_json::from_slice(&output.stdout).expect("the certificate is JSON");
    assert_eq!(certificate["schema"], "aperture.corpus-certificate/1");
    assert_eq!(certificate["verdict"], "certified");
    assert_eq!(
        certificate["failures"].as_array().expect("failures").len(),
        0
    );

    let deck = std::fs::read(repo_root().join("decks/starter-corpus/jasmine-vanilla-stompy.txt"))
        .expect("the exact snapshot is readable");
    assert_eq!(
        certificate["starter"]["snapshot_sha256"],
        sha256_hex(&deck),
        "the certificate hashes the bytes it actually parsed"
    );
    assert_eq!(certificate["starter"]["library_cards"], 99);
    assert_eq!(certificate["starter"]["total_cards"], 100);
    assert_eq!(certificate["starter"]["commander"], "Jasmine Boreal");
    assert_eq!(certificate["starter"]["commander_supported"], true);
    assert_eq!(certificate["starter"]["seats"], 4);
    assert_eq!(
        certificate["starter"]["deck_path"],
        "decks/starter-corpus/jasmine-vanilla-stompy.txt"
    );

    let games = certificate["games"].as_array().expect("games");
    assert_eq!(games.len(), 1);
    assert_eq!(games[0]["status"], "terminal");
    assert_eq!(games[0]["seat_count"], 4);
    assert_eq!(games[0]["starting_life"], 40);
    assert_eq!(games[0]["replay_digest_matches"], true);
    assert!(
        games[0]["replay_steps"].as_u64().expect("steps") > 0,
        "a verified replay has recorded steps"
    );
    assert!(games[0]["turns"].as_u64().expect("turns") > 0);

    // Support is enumerated per name, including the separately supplied
    // commander, so a certified list can never hide an unsupported card.
    let cards = certificate["support"]["cards"].as_array().expect("cards");
    assert_eq!(cards.len(), 61, "58 nonland names + 2 basics + commander");
    for card in cards {
        assert!(card["kind"].as_str().is_some(), "{card}");
        assert!(card["cost"].is_null() || card["cost"].is_string(), "{card}");
    }
    let coverage = &certificate["coverage"];
    assert_eq!(coverage["unique_names_total"], 61);
    assert!(
        coverage["unique_names_observed"]
            .as_u64()
            .expect("observed")
            > 0,
        "the certified games must observe real play"
    );
    // Coverage arithmetic is a published claim, not prose: the separately
    // supplied commander counts like every other card, so observed plus
    // unobserved must add up to the total (an earlier build undercounted by one
    // because it iterated the library only).
    let observed = coverage["unique_names_observed"]
        .as_u64()
        .expect("observed");
    let not_observed = coverage["not_observed"]
        .as_array()
        .expect("not_observed")
        .len() as u64;
    assert_eq!(
        observed + not_observed,
        coverage["unique_names_total"].as_u64().expect("total"),
        "observed + not observed must equal the published name total"
    );
    assert!(
        coverage["cast_names"]
            .as_array()
            .expect("cast names")
            .iter()
            .any(|name| name == "Jasmine Boreal")
            || coverage["not_observed"]
                .as_array()
                .expect("not observed")
                .iter()
                .any(|name| name == "Jasmine Boreal"),
        "the commander must appear in the coverage ledger"
    );
    let exclusions = certificate["interaction_exclusions"]
        .as_array()
        .expect("exclusions");
    assert!(
        exclusions
            .iter()
            .filter_map(Value::as_str)
            .any(|text| text.contains("legend rule"))
    );
}

#[test]
fn a_missing_snapshot_is_refused_rather_than_replaced_by_a_bundled_deck() {
    let root = repo_root();
    let manifest = json!({
        "version": 1,
        "starters": [{
            "id": "jasmine-vanilla-stompy",
            "commander": "Jasmine Boreal",
            "style": "green/white vanilla-creature stompy",
            "status": "implemented",
            "deck_path": "decks/starter-corpus/does-not-exist.txt",
            "missing_cards": [],
            "missing_interactions": [],
        }]
    });
    let manifest_path = scratch("missing-manifest.json", &manifest.to_string());
    let output = Command::new(BIN)
        .current_dir(&root)
        .args([
            "certify",
            "--manifest",
            manifest_path.to_str().expect("utf8 path"),
            "--starter",
            "jasmine-vanilla-stompy",
            "--seeds",
            "1",
        ])
        .output()
        .expect("the CLI binary runs");
    assert!(
        !output.status.success(),
        "a missing exact snapshot must fail certification"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("refusing to substitute"),
        "the failure must name the no-substitution rule: {stderr}"
    );
}

#[test]
fn an_edited_snapshot_stops_certifying() {
    let root = repo_root();
    let text =
        std::fs::read_to_string(root.join("decks/starter-corpus/jasmine-vanilla-stompy.txt"))
            .expect("the exact snapshot reads");
    // Same card count, one unsupported name: the list is no longer the
    // certified snapshot even though nothing else changed.
    let edited = text.replace("1 Savannah Lions", "1 Not A Supported Card");
    assert_ne!(edited, text, "the fixture edit must apply");
    let deck_path = scratch("edited-deck.txt", &edited);
    let manifest = json!({
        "version": 1,
        "starters": [{
            "id": "jasmine-vanilla-stompy",
            "commander": "Jasmine Boreal",
            "style": "green/white vanilla-creature stompy",
            "status": "implemented",
            "deck_path": deck_path.to_str().expect("utf8 path"),
            "missing_cards": [],
            "missing_interactions": [],
        }]
    });
    let manifest_path = scratch("edited-manifest.json", &manifest.to_string());
    let output = Command::new(BIN)
        .current_dir(&root)
        .args([
            "certify",
            "--manifest",
            manifest_path.to_str().expect("utf8 path"),
            "--starter",
            "jasmine-vanilla-stompy",
            "--seeds",
            "1",
        ])
        .output()
        .expect("the CLI binary runs");
    assert!(
        !output.status.success(),
        "an edited snapshot must not certify: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Not A Supported Card"),
        "the error names the unsupported card: {stderr}"
    );
}
