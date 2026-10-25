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
    explicit: bool,
) -> anyhow::Result<(DeckList, String)> {
    if !path.exists() && explicit {
        anyhow::bail!(
            "explicitly requested deck {} does not exist or is not readable; refusing to substitute the bundled practice deck",
            path.display()
        );
    }
    if path.exists() {
        let text = std::fs::read_to_string(path)?;
        // An explicitly requested file's identity in snapshots comes from the
        // file, so an exact starter list is not recorded as "Temur Beats" just
        // because it was loaded through the same helper. Non-explicit default
        // loads keep the prior human names ("Temur Beats"/"Izzet Control").
        let list_name = if explicit {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(fallback_name)
        } else {
            fallback_name
        };
        let list = DeckList::parse_practice(list_name, &text)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        return Ok((list, format!("{}", path.display())));
    }
    let list = DeckList::parse_practice(fallback_name, fallback)
        .map_err(|e| anyhow::anyhow!("bundled {fallback_name}: {e}"))?;
    Ok((list, format!("bundled:{fallback_name}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_missing_deck_path_is_not_replaced_by_the_bundle() {
        let path = Path::new("evidence/issue12/definitely-no-such-deck.txt");
        let result = load(path, BUNDLED_TEMUR, "Temur Beats", true);
        assert!(
            result.is_err(),
            "an explicit missing path must be refused, not silently replaced: {result:?}"
        );
    }

    #[test]
    fn non_explicit_default_loads_keep_the_human_fallback_names() {
        let temur = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../decks/temur-beats.txt");
        let izzet = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../decks/izzet-control.txt");
        let (a, _) = load(&temur, BUNDLED_TEMUR, "Temur Beats", false)
            .expect("the default Temur file loads");
        let (b, _) = load(&izzet, BUNDLED_IZZET, "Izzet Control", false)
            .expect("the default Izzet file loads");
        assert_eq!(a.name, "Temur Beats");
        assert_eq!(b.name, "Izzet Control");
    }

    #[test]
    fn an_explicit_starter_file_keeps_its_own_identity() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../decks/starter-corpus/jasmine-vanilla-stompy.txt");
        let (list, source) = load(&path, BUNDLED_TEMUR, "Temur Beats", true)
            .expect("the exact starter library loads");
        assert_eq!(list.name, "jasmine-vanilla-stompy");
        assert_eq!(list.size(), 99);
        assert_eq!(source, path.display().to_string());
    }
}
