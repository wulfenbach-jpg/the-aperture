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

/// The one snapshot-hash basis a certificate may be checked against: the file's
/// UTF-8 text with CRLF folded to LF, through the same `aperture_core` function
/// `certify` itself uses. Hashing the raw checkout bytes here instead would fail
/// on a Windows checkout of the very same list, because git hands out CRLF there
/// and LF elsewhere — the list is identical, the bytes are not.
fn snapshot_sha256(path: &Path) -> String {
    let bytes =
        std::fs::read(path).unwrap_or_else(|err| panic!("{} is readable: {err}", path.display()));
    aperture_core::digest::snapshot_digest(&bytes).expect("the snapshot is valid UTF-8")
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

    let deck_path = repo_root().join("decks/starter-corpus/jasmine-vanilla-stompy.txt");
    assert_eq!(
        certificate["starter"]["snapshot_sha256"],
        snapshot_sha256(&deck_path),
        "the certificate hashes the snapshot's list text (LF-normalized), so the same list is identified on every platform's checkout"
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

/// Control for the cross-platform portability of the certificate hash: the same
/// list checked out with CRLF line endings must certify to the SAME snapshot
/// hash, because `certify` identifies the list's text, not the checkout's bytes.
/// A raw-byte basis reports two different hashes for one list here, which is
/// exactly what made the Windows CI runner disagree with ubuntu and macOS.
#[test]
fn a_crlf_checkout_of_the_same_list_certifies_to_the_same_snapshot_hash() {
    const CR: u8 = 13;
    const LF: u8 = 10;

    let root = repo_root();
    let canonical = root.join("decks/starter-corpus/jasmine-vanilla-stompy.txt");
    let canonical_hash = snapshot_sha256(&canonical);
    let canonical_bytes = std::fs::read(&canonical).expect("the exact snapshot is readable");
    // Start from the LF-normalized text so this control behaves the same whether
    // the checkout under test is LF (ubuntu/macOS) or CRLF (Windows).
    let lf_text = aperture_core::digest::normalize_snapshot_text(&canonical_bytes)
        .expect("the snapshot is valid UTF-8");

    // Re-encode the identical list with CRLF endings, the way a Windows
    // checkout with core.autocrlf=true hands it to the build.
    let mut crlf_bytes: Vec<u8> = Vec::with_capacity(lf_text.len() * 2);
    for line in lf_text.lines() {
        crlf_bytes.extend_from_slice(line.as_bytes());
        crlf_bytes.extend_from_slice(&[CR, LF]);
    }
    assert_eq!(
        crlf_bytes.len(),
        lf_text.len() + lf_text.lines().count(),
        "the CRLF copy differs only by one CR per line"
    );
    assert_ne!(
        sha256_hex(&crlf_bytes),
        canonical_hash,
        "the control is only meaningful if the raw bytes really hash differently: a raw-byte check calls this a different list"
    );

    let mut deck_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&deck_path).expect("target dir");
    deck_path.push("certify-test-crlf-deck.txt");
    std::fs::write(&deck_path, &crlf_bytes).expect("the CRLF fixture writes");

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
    let manifest_path = scratch("crlf-manifest.json", &manifest.to_string());
    let output = Command::new(BIN)
        .current_dir(&root)
        .args([
            "certify",
            "--manifest",
            manifest_path.to_str().expect("utf8 path"),
            "--starter",
            "jasmine-vanilla-stompy",
            "--seats",
            "4",
            "--seeds",
            "1",
            "--json",
        ])
        .output()
        .expect("the CLI binary runs");
    assert!(
        output.status.success(),
        "the CRLF copy of a certified list must certify: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let certificate: Value =
        serde_json::from_slice(&output.stdout).expect("the certificate is JSON");
    assert_eq!(certificate["verdict"], "certified");
    assert_eq!(
        certificate["starter"]["deck_path"],
        deck_path.to_str().expect("utf8 path"),
        "the certificate names the CRLF file it actually read"
    );
    assert_eq!(
        certificate["starter"]["snapshot_sha256"], canonical_hash,
        "one list has one snapshot hash, whatever the checkout's line endings"
    );
    assert_eq!(certificate["starter"]["library_cards"], 99);
    assert_eq!(certificate["starter"]["total_cards"], 100);
}
