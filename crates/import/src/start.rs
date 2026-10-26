//! Complete, honest start flow: provider deck -> certification -> playable seat.
//!
//! Two certifications are produced and never conflated:
//!
//! * **format legality** — the Commander construction rules checked against the
//!   provider's own card data: one commander, 100 cards including it, singleton,
//!   color identity, commander eligibility. A check that the payload cannot
//!   answer is reported as `not_certified`, never assumed legal.
//! * **engine support** — whether every card in this build's supported pool can
//!   actually be seated, with the exact per-card reason. This is the axis that
//!   decides whether a match can start in the selected bounded profile.
//!
//! The selected profile adds a third, plainly separate question: can this seat
//! hold the list (deck size, command zone, commander eligibility, mana base)?
//!
//! Nothing here substitutes, trims or re-bundles a deck. A deck that fails an
//! axis is refused with the reason, and the caller shows it to the user.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use aperture_core::catalog;
use aperture_core::deck::DeckList;
use aperture_core::state::Profile;

use crate::commander::{LegalityStatus, SupportStatus};
use crate::{
    EntryReport, EntryStatus, ImportError, MAX_COPIES, NormalizedDeck, PRACTICE_DECK_SIZE,
    UnsupportedCard, classify, fetch_deck_cached, normalize, normalize_text, parse_deck_url,
};

pub const START_SCHEMA: &str = "aperture.import-start.v1";

/// Floor for the bounded command-zone profiles. The bounded profile plays the
/// imported library exactly as listed; the 100-card *construction* rule is a
/// format question reported on the format axis, not a seat requirement.
pub const BOUNDED_LIBRARY_MIN: usize = PRACTICE_DECK_SIZE;

/// One Commander construction check. Independent of engine support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatCheck {
    pub code: String,
    pub status: LegalityStatus,
    pub detail: String,
}

/// The format-legality axis: Commander construction, from provider data only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatCertification {
    pub format: String,
    /// Worst check wins: illegal > not_certified > legal.
    pub status: LegalityStatus,
    pub basis: String,
    pub checks: Vec<FormatCheck>,
}

/// The commander the deck designates, and whether this build can seat it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderChoice {
    pub name: String,
    pub quantity: u8,
    pub supported: bool,
    pub reason: String,
}

/// The engine-support axis: exact per-card pool support plus counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportCertification {
    pub status: SupportStatus,
    pub total_cards: u32,
    pub supported_cards: u32,
    pub unsupported_cards: u32,
    /// Every unsupported card with its exact reason. Never truncated.
    pub unsupported: Vec<UnsupportedCard>,
}

/// Whether the selected profile can seat this list at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatFit {
    pub profile: String,
    pub starting_life: i32,
    pub command_zone: bool,
    pub library_cards: usize,
    pub compatible: bool,
    pub reason: Option<String>,
}

/// One seat's complete, honest start certification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckCertification {
    pub schema: String,
    pub provider: String,
    pub source: String,
    pub deck_id: Option<u64>,
    pub deck_name: String,
    pub profile: String,
    pub format: FormatCertification,
    pub support: SupportCertification,
    pub seat: SeatFit,
    pub commander: Option<CommanderChoice>,
    /// Every main-deck entry with its exact status and reason.
    pub entries: Vec<EntryReport>,
    pub sideboard: Vec<UnsupportedCard>,
    pub excluded: Vec<UnsupportedCard>,
    pub excluded_categories: Vec<String>,
    pub startable: bool,
    /// Why start is refused. Empty exactly when `startable` is true.
    pub refusals: Vec<String>,
    pub notes: Vec<String>,
    pub raw_sha256: Option<String>,
    pub cached_path: Option<PathBuf>,
}

/// The exact list a seat receives, plus the commander when the profile has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartDeck {
    pub name: String,
    pub deck: DeckList,
    pub commander: Option<aperture_core::CardId>,
}

impl DeckCertification {
    pub fn can_start(&self) -> bool {
        self.startable
    }

    /// Multi-line human report: format first, support second, then the exact
    /// unsupported cards. Folded by the UI; never truncated here.
    pub fn render(&self) -> String {
        let mut out = format!(
            "{}.{} '{}' — profile {}\nsource: {}\nformat legality: {} (Commander construction; not engine support)\nengine support: {} of {} main-deck cards supported ({} unsupported)\ncommander: {}\nseat fit: {}",
            self.provider,
            self.deck_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "-".to_string()),
            self.deck_name,
            self.profile,
            self.source,
            self.format.status.as_str(),
            self.support.supported_cards,
            self.support.total_cards,
            self.support.unsupported_cards,
            self.commander
                .as_ref()
                .map(|c| format!(
                    "{} ({})",
                    c.name,
                    if c.supported {
                        "supported"
                    } else {
                        "unsupported"
                    }
                ))
                .unwrap_or_else(|| "none listed".to_string()),
            if self.seat.compatible {
                "compatible".to_string()
            } else {
                format!(
                    "incompatible — {}",
                    self.seat.reason.clone().unwrap_or_default()
                )
            }
        );
        for check in &self.format.checks {
            out.push_str(&format!(
                "\n  format/{}/{}: {}",
                check.code,
                check.status.as_str(),
                check.detail
            ));
        }
        if let Some(commander) = &self.commander {
            out.push_str(&format!("\n  commander: {}", commander.reason));
        }
        for unsupported in &self.support.unsupported {
            out.push_str(&format!(
                "\n  unsupported: {} x{} — {}",
                unsupported.name, unsupported.quantity, unsupported.reason
            ));
        }
        for refusal in &self.refusals {
            out.push_str(&format!("\n  refused: {refusal}"));
        }
        for note in &self.notes {
            out.push_str(&format!("\n  note: {note}"));
        }
        if let Some(sha) = &self.raw_sha256 {
            out.push_str(&format!(
                "\n  response sha256: {}",
                &sha[..16.min(sha.len())]
            ));
        }
        if let Some(path) = &self.cached_path {
            out.push_str(&format!("\n  cached at {}", path.display()));
        }
        out
    }
}

/// Merge repeated names so an unsupported card is listed once with its total.
fn merge_unsupported(list: &mut Vec<UnsupportedCard>, name: &str, quantity: u8, reason: &str) {
    match list.iter_mut().find(|c| c.name.eq_ignore_ascii_case(name)) {
        Some(existing) => {
            existing.quantity = existing.quantity.saturating_add(quantity);
            if existing.reason.is_empty() && !reason.is_empty() {
                existing.reason = reason.to_string();
            }
        }
        None => list.push(UnsupportedCard {
            name: name.to_string(),
            quantity,
            reason: reason.to_string(),
        }),
    }
}

fn worst(a: LegalityStatus, b: LegalityStatus) -> LegalityStatus {
    use LegalityStatus::{Illegal, Legal, NotCertified};
    match (a, b) {
        (Illegal, _) | (_, Illegal) => Illegal,
        (NotCertified, _) | (_, NotCertified) => NotCertified,
        (Legal, Legal) => Legal,
    }
}

fn check(code: &str, status: LegalityStatus, detail: String) -> FormatCheck {
    FormatCheck {
        code: code.to_string(),
        status,
        detail,
    }
}

/// Certify one normalized deck for `profile`, on both axes, exactly as supplied.
pub fn certify(deck: &NormalizedDeck, profile: Profile) -> DeckCertification {
    let mut entries: Vec<EntryReport> = Vec::new();
    let mut unsupported: Vec<UnsupportedCard> = Vec::new();
    let mut supported_cards = 0u32;
    let mut unsupported_cards = 0u32;
    let mut total_cards = 0u32;
    let mut supported_lands = 0u32;
    let mut sideboard: Vec<UnsupportedCard> = Vec::new();
    let mut excluded: Vec<UnsupportedCard> = Vec::new();
    let mut commander_entries: Vec<(String, u8, Vec<String>)> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut unnamed = 0u32;
    let mut companions = 0u32;

    for entry in &deck.entries {
        let (status, detail) = classify(entry, &deck.excluded_categories);
        let label = if entry.name.is_empty() {
            unnamed += entry.quantity as u32;
            format!("(no card data #{})", entries.len() + 1)
        } else {
            entry.name.clone()
        };
        entries.push(EntryReport {
            name: label.clone(),
            quantity: entry.quantity,
            categories: entry.categories.clone(),
            status,
            detail: detail.clone(),
        });
        match status {
            EntryStatus::Supported => {
                total_cards += entry.quantity as u32;
                supported_cards += entry.quantity as u32;
                if catalog::by_name(&entry.name)
                    .map(|id| id.def().is_land())
                    .unwrap_or(false)
                {
                    supported_lands += entry.quantity as u32;
                }
            }
            EntryStatus::Unsupported => {
                total_cards += entry.quantity as u32;
                unsupported_cards += entry.quantity as u32;
                merge_unsupported(
                    &mut unsupported,
                    &label,
                    entry.quantity,
                    detail.as_deref().unwrap_or("not supported"),
                );
            }
            EntryStatus::Sideboard => merge_unsupported(
                &mut sideboard,
                &label,
                entry.quantity,
                "sideboard: reported and never added to the deck",
            ),
            EntryStatus::Excluded | EntryStatus::Deleted => merge_unsupported(
                &mut excluded,
                &label,
                entry.quantity,
                detail.as_deref().unwrap_or("excluded by the provider"),
            ),
            EntryStatus::Commander => commander_entries.push((
                label.clone(),
                entry.quantity,
                entry.color_identity.clone(),
            )),
            EntryStatus::Companion => companions += entry.quantity as u32,
        }
    }

    // ---------------------------------------------------------- format axis
    let commander_count = commander_entries.len();
    let commander_total: u32 = commander_entries
        .iter()
        .map(|(_, quantity, _)| *quantity as u32)
        .sum();
    let mut checks: Vec<FormatCheck> = Vec::new();

    checks.push(match commander_count {
        0 => check(
            "commander_present",
            LegalityStatus::Illegal,
            "no card is designated commander: a Commander deck has exactly one".to_string(),
        ),
        1 if commander_total == 1 => check(
            "commander_present",
            LegalityStatus::Legal,
            format!("exactly one commander ({})", commander_entries[0].0),
        ),
        1 => check(
            "commander_present",
            LegalityStatus::Illegal,
            format!(
                "{} is listed {} times: a commander is a single card",
                commander_entries[0].0, commander_total
            ),
        ),
        _ => check(
            "commander_present",
            LegalityStatus::Illegal,
            format!(
                "{commander_count} commanders are designated ({}): partners and multiple commanders are outside this slice",
                commander_entries
                    .iter()
                    .map(|(name, _, _)| name.clone())
                    .collect::<Vec<_>>()
                    .join(" + ")
            ),
        ),
    });

    let deck_total = total_cards + commander_total;
    checks.push(if deck_total == 100 {
        check(
            "deck_size",
            LegalityStatus::Legal,
            format!("100 cards including the commander ({total_cards} + {commander_total})"),
        )
    } else {
        check(
            "deck_size",
            LegalityStatus::Illegal,
            format!(
                "a Commander deck is exactly 100 cards including the commander; this deck has {deck_total} ({total_cards} library + {commander_total} command zone)"
            ),
        )
    });

    // Singleton (CR 903.5b) examines every card *in the library*, whether or not
    // this build can seat it. Engine support is a separate axis and must never
    // narrow a format check: skipping unsupported entries would let a repeated
    // unsupported nonbasic certify as `legal` without ever being looked at.
    //
    // The rule counts cards, not provider rows. Archidekt may split one card
    // into several entries (different printings) and a pasted text list may
    // repeat a line, so entries are aggregated by card name first; otherwise
    // two one-copy entries of the same nonbasic would certify as `legal`.
    struct SingletonGroup {
        key: String,
        name: String,
        quantity: u32,
        /// The provider said *Basic* on at least one entry for this card.
        provider_basic: bool,
        /// The provider sent a type line for at least one entry for this card,
        /// which is enough to know a nonbasic card is a nonbasic card.
        provider_type_line: bool,
        /// Entries carried no name at all: not a repeat we can certify.
        nameless: bool,
    }

    let mut library: Vec<SingletonGroup> = Vec::new();
    let mut singleton_illegal: Vec<String> = Vec::new();
    let mut singleton_unknown: Vec<String> = Vec::new();
    for entry in deck.entries.iter().filter(|e| {
        matches!(
            classify(e, &deck.excluded_categories).0,
            EntryStatus::Supported | EntryStatus::Unsupported
        )
    }) {
        let name = entry.name.trim();
        let key = name.to_lowercase();
        let provider_basic = entry
            .supertypes
            .iter()
            .any(|s| s.eq_ignore_ascii_case("basic"));
        let provider_type_line = !(entry.supertypes.is_empty() && entry.types.is_empty());
        match library
            .iter_mut()
            .find(|group| !group.nameless && !key.is_empty() && group.key == key)
        {
            Some(group) => {
                group.quantity += entry.quantity as u32;
                group.provider_basic |= provider_basic;
                group.provider_type_line |= provider_type_line;
            }
            None => library.push(SingletonGroup {
                key,
                name: name.to_string(),
                quantity: entry.quantity as u32,
                provider_basic,
                provider_type_line,
                nameless: name.is_empty(),
            }),
        }
    }

    for group in library.iter() {
        if group.quantity <= 1 {
            continue;
        }
        if group.nameless {
            singleton_unknown.push(format!("{} unnamed entries", group.quantity));
            continue;
        }
        // Basics are unlimited, but only on a fact: the provider's own `Basic`
        // supertype, or the catalog's basic-land data for a card this build
        // implements. Absence of a type line is not a fact, so a repeated card
        // with no type data stays not_certified instead of being called legal.
        let catalog_card = catalog::by_name(&group.name);
        let catalog_basic = catalog_card
            .map(|id| id.def().basic_mana.is_some())
            .unwrap_or(false);
        if group.provider_basic || catalog_basic {
            continue;
        }
        // A nonbasic card is a known nonbasic either because this build
        // implements it (the catalog says so) or because the provider sent a
        // type line. With neither, the source cannot answer the question.
        let known_nonbasic = catalog_card.is_some() || group.provider_type_line;
        let repeated = format!("{} x{}", group.name, group.quantity);
        if known_nonbasic {
            singleton_illegal.push(repeated);
        } else {
            singleton_unknown.push(repeated);
        }
    }
    checks.push(if !singleton_illegal.is_empty() {
        let mut detail = format!("singleton broken by {}", singleton_illegal.join(", "));
        if !singleton_unknown.is_empty() {
            detail.push_str(&format!(
                "; also not certified: {}",
                singleton_unknown.join(", ")
            ));
        }
        check("singleton", LegalityStatus::Illegal, detail)
    } else if !singleton_unknown.is_empty() {
        check(
            "singleton",
            LegalityStatus::NotCertified,
            format!(
                "singleton not certified: {} appear more than once and the source carries no type line, so basic-land status is unknown",
                singleton_unknown.join(", ")
            ),
        )
    } else {
        check(
            "singleton",
            LegalityStatus::Legal,
            "no nonbasic card appears more than once; basics are unlimited".to_string(),
        )
    });

    // Commander eligibility (CR 903.3): a legendary creature, or a card whose own
    // text says it can be your commander. The provider's type line answers the
    // first question; the second needs the card's permission text, which is not
    // re-certified here, so anything else is `not_certified`, never `illegal`.
    let eligibility = match deck
        .entries
        .iter()
        .find(|entry| classify(entry, &deck.excluded_categories).0 == EntryStatus::Commander)
    {
        None => check(
            "commander_eligibility",
            LegalityStatus::NotCertified,
            "no commander entry to check".to_string(),
        ),
        Some(entry) => {
            let legendary = entry
                .supertypes
                .iter()
                .any(|s| s.eq_ignore_ascii_case("legendary"));
            let creature = entry
                .types
                .iter()
                .any(|t| t.eq_ignore_ascii_case("creature"));
            if entry.supertypes.is_empty() && entry.types.is_empty() {
                check(
                    "commander_eligibility",
                    LegalityStatus::NotCertified,
                    format!(
                        "the provider sent no type line for '{}', so commander eligibility is not certified",
                        entry.name
                    ),
                )
            } else if legendary && creature {
                check(
                    "commander_eligibility",
                    LegalityStatus::Legal,
                    format!("'{}' is a legendary creature", entry.name),
                )
            } else {
                check(
                    "commander_eligibility",
                    LegalityStatus::NotCertified,
                    format!(
                        "'{}' is not a legendary creature on the provider's type line; other commander forms need the card's own permission text, which is not re-certified",
                        entry.name
                    ),
                )
            }
        }
    };
    checks.push(eligibility);

    // A colorless commander is a real, known identity (`[]`), not missing data:
    // the distinction decides between "illegal colored card" and "unknown".
    let commander_identity: Option<Vec<String>> = match commander_entries.first() {
        Some((_, _, identity)) => {
            if deck
                .entries
                .iter()
                .find(|entry| {
                    classify(entry, &deck.excluded_categories).0 == EntryStatus::Commander
                })
                .map(|entry| entry.color_identity_known)
                .unwrap_or(false)
            {
                Some(identity.clone())
            } else {
                None
            }
        }
        None => None,
    };
    let mut missing_identity = 0u32;
    let mut off_identity: Vec<String> = Vec::new();
    if let Some(identity) = &commander_identity {
        for entry in deck.entries.iter() {
            // Only the deck itself is checked: sideboard, excluded and deleted
            // entries are reported elsewhere and are not part of the library.
            if !matches!(
                classify(entry, &deck.excluded_categories).0,
                EntryStatus::Supported | EntryStatus::Unsupported
            ) {
                continue;
            }
            if !entry.color_identity_known {
                missing_identity += entry.quantity as u32;
                continue;
            }
            if let Some(off) = entry
                .color_identity
                .iter()
                .find(|color| !identity.iter().any(|allowed| allowed == *color))
            {
                off_identity.push(format!("{} ({off})", entry.name));
            }
        }
    }
    checks.push(match (commander_identity.is_some(), missing_identity, off_identity.is_empty()) {
        (false, _, _) => check(
            "color_identity",
            LegalityStatus::NotCertified,
            "the provider sent no color identity for the commander: identity is not inferred".to_string(),
        ),
        (true, 0, true) => check(
            "color_identity",
            LegalityStatus::Legal,
            format!(
                "every library card is inside the commander's identity [{}]",
                commander_identity.clone().unwrap_or_default().join("")
            ),
        ),
        (true, missing, true) => check(
            "color_identity",
            LegalityStatus::NotCertified,
            format!("{missing} card(s) arrived with no color identity, so the identity check is incomplete"),
        ),
        (true, _, false) => check(
            "color_identity",
            LegalityStatus::Illegal,
            format!(
                "outside the commander's identity: {}",
                off_identity.join(", ")
            ),
        ),
    });

    let format_status = checks
        .iter()
        .fold(LegalityStatus::Legal, |acc, c| worst(acc, c.status));
    let format = FormatCertification {
        format: "commander".to_string(),
        status: format_status,
        basis: "provider card data: commander designation, card count, quantities, color identity. \
                Per-card banned/errata status from the provider's legalities field is not re-certified."
            .to_string(),
        checks,
    };

    // --------------------------------------------------------- support axis
    let commander = commander_entries.first().map(|(name, quantity, _)| {
        let (supported, reason) = match catalog::by_name(name) {
            Some(card) if card.is_supported_commander() => (
                true,
                format!("'{name}' is a supported commander in this build"),
            ),
            Some(_) => (
                false,
                format!(
                    "'{name}' is in the pool but this build has no command-zone implementation for it; the supported commander is 'Isamaru, Hound of Konda'"
                ),
            ),
            None => (
                false,
                format!("'{name}' is not in the supported pool, so the command zone cannot hold it"),
            ),
        };
        CommanderChoice {
            name: name.clone(),
            quantity: *quantity,
            supported,
            reason,
        }
    });
    // Additional designated commanders are not library cards: they are reported
    // as their own refusal rather than inflating the main-deck counts.
    let extra_commanders: Vec<String> = commander_entries
        .iter()
        .skip(1)
        .map(|(name, _, _)| name.clone())
        .collect();
    // `total/supported/unsupported` stay strictly about the main deck: the
    // commander is reported on its own axis field and never inflated into the
    // library counts.
    let support_status = if unsupported_cards == 0
        && extra_commanders.is_empty()
        && commander
            .as_ref()
            .map(|choice| choice.supported)
            .unwrap_or(true)
    {
        SupportStatus::Supported
    } else {
        SupportStatus::Unsupported
    };
    let support = SupportCertification {
        status: support_status,
        total_cards,
        supported_cards,
        unsupported_cards,
        unsupported,
    };

    // ----------------------------------------------------------- seat fit
    let library_cards = total_cards as usize;
    let mut seat_reason: Option<String> = None;
    if profile.has_command_zone() {
        if commander.is_none() {
            seat_reason = Some(format!(
                "the {} profile needs a card in the Commander category; this deck lists none",
                profile.label()
            ));
        } else if library_cards < BOUNDED_LIBRARY_MIN {
            seat_reason = Some(format!(
                "the bounded {} profile needs at least {BOUNDED_LIBRARY_MIN} library cards; this deck has {library_cards}",
                profile.label()
            ));
        }
    } else if commander.is_some() {
        seat_reason = Some(format!(
            "the {} profile has no command zone, so a deck with a commander cannot be seated; select a named command profile to import it",
            profile.label()
        ));
    } else if library_cards != PRACTICE_DECK_SIZE {
        seat_reason = Some(format!(
            "the {} profile requires exactly {PRACTICE_DECK_SIZE} main-deck cards; this deck has {library_cards}",
            profile.label()
        ));
    }
    if seat_reason.is_none() && supported_lands == 0 {
        seat_reason =
            Some("the deck has no supported lands: the seat needs a working mana base".to_string());
    }
    if seat_reason.is_none()
        && let Some(over) = deck.entries.iter().find(|entry| {
            classify(entry, &deck.excluded_categories).0 == EntryStatus::Supported
                && entry.quantity > MAX_COPIES
                && catalog::by_name(&entry.name)
                    .map(|id| id.def().basic_mana.is_none())
                    .unwrap_or(false)
        })
    {
        seat_reason = Some(format!(
            "more than {MAX_COPIES} copies of {} (x{}) is outside the bounded profile",
            over.name, over.quantity
        ));
    }
    let seat = SeatFit {
        profile: profile.label().to_string(),
        starting_life: profile.starting_life(),
        command_zone: profile.has_command_zone(),
        library_cards,
        compatible: seat_reason.is_none(),
        reason: seat_reason,
    };

    // ----------------------------------------------------------- refusals
    let mut refusals = Vec::new();
    if support.status == SupportStatus::Unsupported {
        let listed: Vec<String> = support
            .unsupported
            .iter()
            .take(5)
            .map(|c| format!("{} x{} ({})", c.name, c.quantity, c.reason))
            .collect();
        refusals.push(format!(
            "engine support: {} card(s) cannot be seated — {}{}. Unsupported cards are refused, never substituted",
            support.unsupported_cards,
            listed.join("; "),
            if support.unsupported.len() > 5 {
                format!(" (+{} more listed above)", support.unsupported.len() - 5)
            } else {
                String::new()
            }
        ));
    }
    if !seat.compatible {
        refusals.push(format!(
            "seat fit: {}",
            seat.reason.clone().unwrap_or_default()
        ));
    }
    if let Some(choice) = &commander
        && !choice.supported
    {
        refusals.push(format!("commander: {}", choice.reason));
    }
    if !extra_commanders.is_empty() {
        refusals.push(format!(
            "commanders: {} additional designated commander(s) ({}) — partners and multiple commanders are not modeled, so only one commander can be seated",
            extra_commanders.len(),
            extra_commanders.join(", ")
        ));
    }

    if unnamed > 0 {
        notes.push(format!(
            "{unnamed} card(s) arrived without provider card data and are listed as unsupported rather than guessed"
        ));
    }
    if companions > 0 {
        notes.push(format!(
            "{companions} companion entry(ies) are reported separately and never added to the deck"
        ));
    }
    if format.status == LegalityStatus::Illegal {
        notes.push(
            "this deck is not Commander-legal (see the format checks); the bounded practice profiles are not the Commander format, so format legality is reported, not enforced"
                .to_string(),
        );
    } else if format.status == LegalityStatus::NotCertified {
        notes.push(
            "Commander format legality is not fully certified for this deck (see the format checks); it is reported separately from engine support"
                .to_string(),
        );
    }
    if !deck.excluded_categories.is_empty() {
        notes.push(format!(
            "excluded categories honored: {}",
            deck.excluded_categories.join(", ")
        ));
    }

    let startable =
        support.status == SupportStatus::Supported && seat.compatible && refusals.is_empty();

    DeckCertification {
        schema: START_SCHEMA.to_string(),
        provider: deck.provider.clone(),
        source: deck.source.clone(),
        deck_id: deck.deck_id,
        deck_name: deck.name.clone(),
        profile: profile.label().to_string(),
        format,
        support,
        seat,
        commander,
        entries,
        sideboard,
        excluded,
        excluded_categories: deck.excluded_categories.clone(),
        startable,
        refusals,
        notes,
        raw_sha256: None,
        cached_path: None,
    }
}

/// Build the exact seat list, or refuse with every recorded reason.
pub fn plan_start(cert: &DeckCertification) -> Result<StartDeck, ImportError> {
    if !cert.startable {
        return Err(ImportError::NotLaunchable(cert.refusals.join(" | ")));
    }
    let mut cards: Vec<(aperture_core::CardId, u8)> = Vec::new();
    for entry in cert.entries.iter() {
        if entry.status != EntryStatus::Supported {
            continue;
        }
        let id = catalog::by_name(&entry.name).ok_or_else(|| {
            ImportError::NotLaunchable(format!(
                "'{}' was certified as supported but is not in the pool",
                entry.name
            ))
        })?;
        match cards.iter_mut().find(|(card, _)| *card == id) {
            Some((_, quantity)) => *quantity = quantity.saturating_add(entry.quantity),
            None => cards.push((id, entry.quantity)),
        }
    }
    if cards.is_empty() {
        return Err(ImportError::NotLaunchable(
            "the deck has no supported cards".to_string(),
        ));
    }
    let commander = match &cert.commander {
        Some(choice) => Some(choice.card().ok_or_else(|| {
            ImportError::NotLaunchable(format!("commander {} cannot be seated", choice.name))
        })?),
        None => None,
    };
    Ok(StartDeck {
        name: cert.deck_name.clone(),
        deck: DeckList::new(cert.deck_name.clone(), cards),
        commander,
    })
}

impl CommanderChoice {
    /// The pool card for a supported commander.
    pub fn card(&self) -> Option<aperture_core::CardId> {
        if !self.supported {
            return None;
        }
        catalog::by_name(&self.name).filter(|id| id.is_supported_commander())
    }
}

// ------------------------------------------------------------- source entry

/// URL -> certification. The existing URL guard, redirect policy, timeout and
/// size cap are reused unchanged, and the raw response is cached with the
/// certification beside it.
pub fn certify_url(
    url: &str,
    cache_dir: &Path,
    profile: Profile,
) -> Result<DeckCertification, ImportError> {
    let deck_ref = parse_deck_url(url)?;
    let mut fetch = fetch_deck_cached(&deck_ref, cache_dir)?;
    let mut normalized = normalize(&fetch.raw_json)?;
    if let Some(id) = normalized.deck_id
        && id != deck_ref.deck_id
    {
        return Err(ImportError::Parse(format!(
            "provider returned deck id {id} for a request for {}",
            deck_ref.deck_id
        )));
    }
    normalized.source = deck_ref.url.clone();
    let mut cert = certify(&normalized, profile);
    cert.raw_sha256 = Some(fetch.sha256.clone());
    cert.cached_path = fetch.cached_path.clone();
    if let Some(note) = fetch.note.take() {
        cert.notes.push(note);
    }
    let report_path = cache_dir.join(format!(
        "archidekt-{}.start-{}.json",
        deck_ref.deck_id,
        profile.label()
    ));
    std::fs::write(
        &report_path,
        serde_json::to_vec_pretty(&cert).unwrap_or_default(),
    )
    .map_err(|e| ImportError::Cache(e.to_string()))?;
    cert.notes
        .push(format!("certification cached at {}", report_path.display()));
    Ok(cert)
}

/// Pasted text -> certification. Same certification code path as a provider
/// response; the text form simply carries no provider card data.
pub fn certify_text(
    name: &str,
    text: &str,
    profile: Profile,
) -> Result<DeckCertification, ImportError> {
    let deck = normalize_text(name, text)?;
    Ok(certify(&deck, profile))
}

/// Deck file -> certification. A read failure is reported, never replaced by a
/// bundled list.
pub fn certify_file(path: &Path, profile: Profile) -> Result<DeckCertification, ImportError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| ImportError::Parse(format!("could not read {}: {e}", path.display())))?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported list");
    certify_text(name, &text, profile)
}
