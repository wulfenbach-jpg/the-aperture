//! `import` / `import-txt` / `decider-health`.

use std::path::Path;

use aperture_ai::{DeciderClient, DeciderConfig};
use aperture_import as imports;
use serde_json::json;

fn summarize(report: &imports::DeckReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} '{}' — {} card(s): {} supported, {} unsupported\n",
        report.provider,
        report.deck_name,
        report.total_cards,
        report.supported_cards,
        report.unsupported_cards
    ));
    out.push_str(&format!("  source: {}\n", report.source));
    if let Some(sha) = &report.raw_sha256 {
        out.push_str(&format!("  provider response sha256: {}\n", &sha[..32]));
    }
    if let Some(path) = &report.cached_path {
        out.push_str(&format!("  cached response: {}\n", path.display()));
    }
    if !report.sideboard.is_empty() {
        out.push_str(&format!(
            "  sideboard (not part of the deck): {}\n",
            report
                .sideboard
                .iter()
                .map(|c| format!("{} x{}", c.name, c.quantity))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !report.excluded.is_empty() {
        out.push_str(&format!(
            "  excluded entries: {}\n",
            report
                .excluded
                .iter()
                .map(|c| format!("{} x{}", c.name, c.quantity))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !report.commander.is_empty() {
        out.push_str(&format!("  commander: {}\n", report.commander.join(", ")));
    }
    if !report.unsupported.is_empty() {
        out.push_str("  unsupported:\n");
        for card in report.unsupported.iter().take(12) {
            out.push_str(&format!(
                "    {} x{} — {}\n",
                card.name, card.quantity, card.reason
            ));
        }
        if report.unsupported.len() > 12 {
            out.push_str(&format!("    ... {} more\n", report.unsupported.len() - 12));
        }
    }
    for note in &report.notes {
        out.push_str(&format!("  note: {note}\n"));
    }
    match &report.refusal {
        Some(reason) => out.push_str(&format!("  LAUNCH REFUSED: {reason}\n")),
        None => out.push_str("  launchable: yes (fully supported main deck)\n"),
    }
    out
}

pub fn import_url(
    url: &str,
    cache: &Path,
    out: Option<&Path>,
    plan: bool,
    json_out: bool,
) -> anyhow::Result<()> {
    let report = imports::import_url(url, cache)?;
    if json_out {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", summarize(&report));
    }
    if plan {
        match imports::plan_deck(&report) {
            Ok(list) => println!(
                "  planned deck list: {} card(s) across {} distinct cards — playable in the strict mode",
                list.size(),
                list.cards.len()
            ),
            Err(err) => println!("  plan refused: {err}"),
        }
    }
    if let Some(path) = out {
        std::fs::write(path, serde_json::to_string_pretty(&report)? + "\n")?;
        println!("report written to {}", path.display());
    }
    Ok(())
}

pub fn import_text(file: &Path, name: Option<&str>, json_out: bool) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(file)?;
    let deck_name = name.map(|s| s.to_string()).unwrap_or_else(|| {
        file.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "text deck".to_string())
    });
    let report = imports::text_report(&deck_name, &text)?;
    if json_out {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", summarize(&report));
    }
    match imports::plan_deck(&report) {
        Ok(list) => {
            println!(
                "  text list is playable as-is: {} card(s), {} distinct",
                list.size(),
                list.cards.len()
            );
            Ok(())
        }
        Err(err) => {
            println!("  text list refused for play: {err}");
            Ok(())
        }
    }
}

pub fn decider_health(base_url: &str) -> anyhow::Result<()> {
    let config = DeciderConfig {
        base_url: base_url.to_string(),
        ..DeciderConfig::default()
    };
    let client = match DeciderClient::new(config) {
        Ok(client) => client,
        Err(err) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema": "aperture-decider-health/1",
                    "base_url": base_url,
                    "accepted": false,
                    "error": err.to_string(),
                    "note": "only http://127.0.0.1:<port> / http://localhost:<port> is accepted",
                }))?
            );
            return Ok(());
        }
    };
    match client.health() {
        Ok(true) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema": "aperture-decider-health/1",
                    "base_url": base_url,
                    "accepted": true,
                    "reachable": true,
                    "model": client.config().model,
                    "note": "the model is optional; the heuristic is the default policy",
                }))?
            );
        }
        Ok(false) => println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema": "aperture-decider-health/1",
                "base_url": base_url,
                "accepted": true,
                "reachable": false,
                "note": "the service answered with an error status",
            }))?
        ),
        Err(err) => println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema": "aperture-decider-health/1",
                "base_url": base_url,
                "accepted": true,
                "reachable": false,
                "error": err.to_string(),
                "note": "the model is optional; the heuristic produced every reported result",
            }))?
        ),
    }
    Ok(())
}
