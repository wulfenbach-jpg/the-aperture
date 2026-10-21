//! `commander-check`: offline Commander format-legality analysis only.
//!
//! Exit codes: 0 legal, 1 illegal, 2 not_certified or input/provenance error.

use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use aperture_import::commander::{CommanderReport, LegalityStatus, analyze_files};

fn summary(report: &CommanderReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Commander analysis: {} ({} commander + {} library = {} cards)\n",
        report.status.as_str(),
        report.deck.commanders,
        report.deck.library,
        report.deck.total
    ));
    for commander in &report.commanders {
        out.push_str(&format!(
            "  commander: {} [{}] {} ({})\n",
            commander.name,
            commander.color_identity.join(""),
            commander.eligibility.as_str(),
            commander.reason
        ));
    }
    out.push_str(&format!(
        "  format legality: {} as of {} (age {} days, stale: {})\n",
        report.format_legality.status.as_str(),
        report.format_legality.as_of,
        report.format_legality.age_days,
        report.format_legality.stale
    ));
    out.push_str(&format!(
        "  engine support: {}\n",
        match report.engine_support.status {
            aperture_import::commander::SupportStatus::Supported => "supported",
            aperture_import::commander::SupportStatus::Unsupported => "unsupported",
        }
    ));
    for card in &report.engine_support.unsupported_cards {
        out.push_str(&format!(
            "    unsupported: {} ({})\n",
            card.name, card.reason
        ));
    }
    out.push_str(&format!(
        "  playable Commander game: {} — {}\n",
        if report.gameplay.available {
            "yes"
        } else {
            "NO"
        },
        report.gameplay.reasons.join("; ")
    ));
    for finding in &report.findings {
        out.push_str(&format!(
            "  [{}] {}: {}\n",
            finding.status.as_str(),
            finding.code,
            finding.message
        ));
    }
    out
}

pub fn run(deck: &Path, cards: &Path, json: bool, out_path: Option<&Path>) -> i32 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let report = match analyze_files(deck, cards, now) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("commander-check: {error}");
            return 2;
        }
    };
    let mut json_bytes = match serde_json::to_vec_pretty(&report) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("commander-check: cannot serialize report: {error}");
            return 2;
        }
    };
    json_bytes.push(b'\n');
    if let Some(path) = out_path
        && let Err(error) = std::fs::write(path, &json_bytes)
    {
        eprintln!("commander-check: cannot write {}: {error}", path.display());
        return 2;
    }
    if json {
        if std::io::stdout().write_all(&json_bytes).is_err() {
            return 2;
        }
    } else {
        print!("{}", summary(&report));
    }
    match report.status {
        LegalityStatus::Legal => 0,
        LegalityStatus::Illegal => 1,
        LegalityStatus::NotCertified => 2,
    }
}
