//! Trace-backed coaching. Every statement is derived from the deterministic
//! state, the legal-action list, or the pinned rule text; nothing here is
//! generated prose or a fabricated evaluation.

use crate::action::LegalAction;
use crate::card::TargetSpec;
use crate::engine::Game;
use crate::ids::PlayerId;
use crate::state::{OrderMode, Pending};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Explanation {
    pub topic: String,
    pub text: String,
    /// Comprehensive Rules citations.
    pub rules: Vec<&'static str>,
}

/// Short factual notes for the rule numbers this engine cites. Paraphrases are
/// marked as such; the pinned revision is 2026-09-25.
pub static RULE_NOTES: &[(&str, &str)] = &[
    (
        "103.5",
        "Opening hands: a player may mulligan; the London mulligan draws a fresh seven and then puts a number of cards on the bottom equal to the mulligans taken.",
    ),
    (
        "104.3c",
        "A player who is required to draw more cards than their library holds draws what remains, then loses the game at the next priority check.",
    ),
    (
        "117.1a",
        "An instant may be cast any time its controller has priority; a noninstant spell only during its controller's main phase with an empty stack.",
    ),
    (
        "117.3c",
        "A player who casts a spell, activates an ability or takes a special action receives priority afterward.",
    ),
    (
        "117.3d",
        "A player who chooses not to act passes; then the next player in turn order receives priority.",
    ),
    (
        "117.4",
        "If all players pass in succession, the topmost spell or ability resolves, or the step/phase ends if the stack is empty.",
    ),
    (
        "117.5",
        "Each time a player would receive priority, state-based actions are performed first, then waiting triggered abilities are put on the stack.",
    ),
    (
        "302.6",
        "A creature cannot attack unless it has been under its controller's control continuously since that player's most recent turn began (summoning sickness).",
    ),
    (
        "401.2",
        "A library is a face-down pile: players may not look at or reorder it.",
    ),
    (
        "500.5",
        "As a step or phase ends, unspent mana empties from mana pools.",
    ),
    (
        "502.3",
        "The active player untaps all permanents they control simultaneously at the start of the untap step.",
    ),
    (
        "502.4",
        "No player receives priority during the untap step.",
    ),
    (
        "504.1",
        "The active player draws a card as a turn-based action during the draw step.",
    ),
    (
        "508.1a",
        "The active player chooses which untapped creatures attack; each must have been controlled continuously since the turn began.",
    ),
    ("508.1f", "The active player taps the chosen attackers."),
    (
        "509.1a",
        "The defending player chooses which untapped creatures block and which attacker each one blocks.",
    ),
    (
        "510.1a",
        "Each attacking and blocking creature assigns combat damage equal to its power.",
    ),
    (
        "510.1c",
        "A blocked creature assigns its damage to the creatures blocking it, divided as its controller chooses, lethal first.",
    ),
    ("510.2", "All combat damage is dealt simultaneously."),
    (
        "514.1",
        "In the cleanup step, the active player discards down to their maximum hand size.",
    ),
    (
        "514.2",
        "In the cleanup step, all damage marked on permanents is removed and all until-end-of-turn effects end.",
    ),
    (
        "514.3",
        "Normally no player receives priority during the cleanup step.",
    ),
    (
        "601.2a",
        "Casting a spell moves the card from its zone to the stack; it becomes the topmost object there.",
    ),
    (
        "605.1a",
        "A mana ability is an activated ability that needs no target, could add mana, is not a loyalty ability, and does not move cards to or from a library. It does not use the stack.",
    ),
    (
        "608.2b",
        "A spell with targets checks its targets on resolution; if every target is illegal it does not resolve.",
    ),
    (
        "608.2c",
        "The controller of a resolving spell follows its instructions in the order written.",
    ),
    (
        "701.6a",
        "To counter a spell is to remove it from the stack; none of its effects occur, and it goes to its owner's graveyard.",
    ),
    (
        "701.18a",
        "A land may be played during a player's main phase while they have priority, the stack is empty, and they have not played a land this turn.",
    ),
    (
        "701.22a",
        "Scry N: look at the top N cards of your library, then put any number of them on the bottom and the rest on top.",
    ),
    (
        "701.25a",
        "Surveil N: look at the top N cards of your library, then put any number of them into your graveyard and the rest on top.",
    ),
    ("704.5a", "A player at 0 or less life loses the game."),
    (
        "704.5b",
        "A player who attempted to draw from an empty library since the last state-based-action check loses the game.",
    ),
    (
        "704.5f",
        "A creature with toughness 0 or less is put into its owner's graveyard.",
    ),
    (
        "704.5g",
        "A creature with marked damage greater than or equal to its toughness has lethal damage and is destroyed.",
    ),
];

pub fn rule_note(number: &str) -> Option<&'static str> {
    RULE_NOTES
        .iter()
        .find(|(n, _)| *n == number)
        .map(|(_, text)| *text)
}

/// Why is this card not castable right now? Returns `None` when it is castable.
pub fn cast_blocker(game: &Game, player: PlayerId, card: crate::ids::ObjectId) -> Option<String> {
    let targets = game.target_combinations(player, card);
    if targets.is_empty() {
        return Some(format!(
            "{} needs a legal target and none exists for {}",
            game.state.object(card).card.name(),
            target_spec_label(
                game.state
                    .object(card)
                    .card
                    .def()
                    .spell_spec()
                    .map(|s| s.target)
            )
        ));
    }
    for combo in targets {
        let action = crate::action::Action::CastSpell {
            card,
            targets: combo,
        };
        match game.validate(player, &action) {
            Ok(()) => return None,
            Err(err) => {
                // Keep looking: another target combination may be legal.
                if let crate::error::EngineError::CannotPay { .. } = err {
                    return Some(err.reason());
                }
            }
        }
    }
    // No combination worked: report the first failure for the first target.
    let name = game.state.object(card).card.name();
    let cost = game.state.object(card).card.def().cost.unwrap_or("");
    Some(format!(
        "{name} is not castable now (cost {cost}); check timing and mana"
    ))
}

fn target_spec_label(spec: Option<TargetSpec>) -> &'static str {
    match spec {
        Some(TargetSpec::AnyTarget) => "any target",
        Some(TargetSpec::Creature) => "a creature",
        Some(TargetSpec::Spell) => "a spell",
        _ => "no target",
    }
}

/// Factual explanation records for the current decision point, generated from
/// `viewer`'s information boundary: another seat's hand is never described.
pub fn explain(game: &Game, viewer: PlayerId, legal: &[LegalAction]) -> Vec<Explanation> {
    let state = &game.state;
    let mut out = Vec::new();

    match state.decision_owner() {
        Some(owner) => out.push(Explanation {
            topic: "priority".to_string(),
            text: format!(
                "Turn {} — P{} is the active player. P{owner} {} in the {}.",
                state.turn.number,
                state.turn.active,
                if state.pending.is_some() {
                    "must answer a pending choice"
                } else {
                    "holds priority"
                },
                state.turn.step.name()
            ),
            rules: if state.pending.is_some() {
                vec!["608.2d"]
            } else {
                vec!["117.1a", "117.3d"]
            },
        }),
        None => out.push(Explanation {
            topic: "priority".to_string(),
            text: "No player holds priority: the game is resolving, between steps, or over."
                .to_string(),
            rules: vec!["117.4", "117.5"],
        }),
    }

    if let Some(item) = state.stack.last() {
        let card = state.object(item.object).card;
        let targets: Vec<String> = item
            .targets
            .iter()
            .map(|t| game.describe_target(*t))
            .collect();
        out.push(Explanation {
            topic: "stack".to_string(),
            text: format!(
                "Top of stack: {} (controlled by P{}{}). {} item(s) below it.",
                card.name(),
                item.controller,
                if targets.is_empty() {
                    String::new()
                } else {
                    format!(", targeting {}", targets.join(", "))
                },
                state.stack.len() - 1
            ),
            rules: vec!["608.2b", "608.2c"],
        });
    }

    if let Some(pending) = &state.pending {
        let rules = match pending {
            Pending::Mulligan { .. } | Pending::BottomCards { .. } => vec!["103.5"],
            Pending::DeclareAttackers { .. } => vec!["508.1a"],
            Pending::DeclareBlockers { .. } => vec!["509.1a"],
            Pending::OrderCard { mode, .. } => match mode {
                OrderMode::Scry => vec!["701.22a"],
                OrderMode::Surveil => vec!["701.25a"],
            },
            Pending::Discard { .. } => vec!["514.1"],
        };
        out.push(Explanation {
            topic: "pending choice".to_string(),
            text: pending.describe(),
            rules,
        });
    }

    // Why is each card in hand unavailable?
    // Only the seat that owns the decision is told about its own hand;
    // describing another seat's hand would leak it.
    if let Some(player) = state.decision_owner().filter(|owner| *owner == viewer)
        && state.pending.is_none()
    {
        for id in state.player(player).hand.clone() {
            let card = state.object(id).card;
            if card.def().is_land() {
                if let Err(err) =
                    game.validate(player, &crate::action::Action::PlayLand { card: id })
                {
                    out.push(Explanation {
                        topic: format!("{} unavailable", card.name()),
                        text: err.reason(),
                        rules: vec!["305.2", "701.18a"],
                    });
                }
                continue;
            }
            if let Some(reason) = cast_blocker(game, player, id) {
                out.push(Explanation {
                    topic: format!("{} unavailable", card.name()),
                    text: reason,
                    rules: vec!["117.1a", "601.2c"],
                });
            }
        }
    }

    let lethal: Vec<String> = state
        .objects
        .values()
        .filter(|o| {
            o.zone == crate::ids::Zone::Battlefield
                && o.is_creature()
                && (o.toughness() <= 0 || o.damage as i32 >= o.toughness())
        })
        .map(|o| {
            format!(
                "{} ({} damage, {} toughness)",
                o.name(),
                o.damage,
                o.toughness()
            )
        })
        .collect();
    if !lethal.is_empty() {
        out.push(Explanation {
            topic: "state-based actions".to_string(),
            text: format!(
                "Waiting for state-based actions: {} has lethal damage and will be destroyed.",
                lethal.join(", ")
            ),
            rules: vec!["117.5", "704.5g"],
        });
    }

    out.push(Explanation {
        topic: "board".to_string(),
        text: format!(
            "Life: P{} {} / P{} {}. Battlefield: P{} {} permanent(s), P{} {} permanent(s). Hand: P{} {}, P{} {}. Libraries: {} and {} card(s).",
            crate::ids::P0,
            state.player(crate::ids::P0).life,
            crate::ids::P1,
            state.player(crate::ids::P1).life,
            crate::ids::P0,
            state.battlefield(crate::ids::P0).len(),
            crate::ids::P1,
            state.battlefield(crate::ids::P1).len(),
            crate::ids::P0,
            state.player(crate::ids::P0).hand_size(),
            crate::ids::P1,
            state.player(crate::ids::P1).hand_size(),
            state.player(crate::ids::P0).library.len(),
            state.player(crate::ids::P1).library.len()
        ),
        rules: vec!["401.3"],
    });

    out.push(Explanation {
        topic: "legal actions".to_string(),
        text: format!(
            "{} legal action(s) generated for the current decision point.",
            legal.len()
        ),
        rules: vec!["117.1a"],
    });

    out
}
