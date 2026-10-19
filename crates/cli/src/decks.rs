//! Bundled deck and card-pool access. The bundled practice decks are compiled
//! in, so the CLI works from any working directory; a file path given on the
//! command line wins when it exists.

use std::path::Path;

use aperture_core::DeckList;

pub const BUNDLED_TEMUR: &str = include_str!("../../../decks/temur-beats.txt");
pub const BUNDLED_IZZET: &str = include_str!("../../../decks/izzet-control.txt");

pub fn load(
    path: &Path,
    fallback: &str,
    fallback_name: &str,
) -> anyhow::Result<(DeckList, String)> {
    if path.exists() {
        let text = std::fs::read_to_string(path)?;
        let list = DeckList::parse_practice(fallback_name, &text)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        return Ok((list, format!("{}", path.display())));
    }
    let list = DeckList::parse_practice(fallback_name, fallback)
        .map_err(|e| anyhow::anyhow!("bundled {fallback_name}: {e}"))?;
    Ok((list, format!("bundled:{fallback_name}")))
}
