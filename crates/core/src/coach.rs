//! Trace-backed coaching. Every statement is derived from the deterministic
//! state, the legal-action list, or the pinned rule text; nothing here is
//! generated prose or a fabricated evaluation.

use crate::action::LegalAction;
use crate::card::TargetSpec;
use crate::engine::Game;
use crate::ids::PlayerId;
use crate::state::{OrderMode, Pending};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationKind {
    /// Derived from the deterministic state or the pinned rule text.
    Rule,
    /// An explicit engine simplification, never taught as the rule.
    Simplification,
    /// Clearly non-rule advice; this slice generates strategy text never from
    /// engine data, so the only strategy record states its absence.
    Strategy,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Explanation {
    pub topic: String,
    pub kind: ExplanationKind,
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
        "101.4",
        "Paraphrase: when several players make simultaneous choices, they make them in APNAP order — active player first, then players in turn order.",
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
        "103.5c",
        "In a multiplayer game (and Brawl), a player's first mulligan is free: it does not count toward the cards bottomed or the mulligan limit.",
    ),
    (
        "103.8a",
        "In a two-player game, the player who plays first skips the draw step of their first turn.",
    ),
    (
        "103.8c",
        "In all other multiplayer games, no player skips the draw step of their first turn.",
    ),
    (
        "104.4a",
        "If all the players remaining in a game lose simultaneously, the game is a draw (engine projection: Outcome::Draw).",
    ),
    (
        "800.4a",
        "When a player leaves the game, all objects they own leave the game and control effects end; if they had priority, the next player in turn order still in the game receives it.",
    ),
    (
        "800.4e",
        "If combat damage would be assigned to a player who has left the game, that damage isn't assigned.",
    ),
    (
        "800.4j",
        "If a player leaves during their turn, the turn continues to completion without an active player; priority goes to the next live player, the top object may resolve, or the step/phase may end.",
    ),
    (
        "800.4k",
        "If a player who has left the game would begin a turn, that turn doesn't begin.",
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
        "A blocked creature assigns its combat damage to the creatures blocking it. With one blocker it assigns all of it; with two or more it divides the damage among them as its controller chooses. There is no lethal-first requirement (the old wording was removed).",
    ),
    (
        "510.1d",
        "A blocking creature assigns its combat damage to the creatures it blocks, divided as its controller chooses if it blocks two or more.",
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
            kind: ExplanationKind::Rule,
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
            kind: ExplanationKind::Rule,
            text: "No player holds priority: the game is resolving, between steps, or over."
                .to_string(),
            rules: vec!["117.4", "117.5"],
        }),
    }

    // Consecutive passes and what resolves next (CR 117.3d, 117.4). Derived
    // from the typed pass-round bitmask; reset semantics live in the engine.
    if let Some(owner) = state.decision_owner()
        && state.pending.is_none()
        && state.turn.priority.is_some()
    {
        let live_mask = state.live_mask();
        let passed: Vec<String> = (0..state.seat_count() as PlayerId)
            .filter(|p| state.turn.passed & (1u8 << p) != 0)
            .map(|p| format!("P{p}"))
            .collect();
        let remaining: Vec<String> = (0..state.seat_count() as PlayerId)
            .filter(|p| live_mask & (1u8 << p) != 0 && state.turn.passed & (1u8 << p) == 0)
            .map(|p| format!("P{p}"))
            .collect();
        let resolves_next = match state.stack.last() {
            Some(item) => format!("{} resolves next", state.object(item.object).card.name()),
            None => "the current step/phase ends next".to_string(),
        };
        let text = if passed.is_empty() {
            format!(
                "No one has passed yet in the current priority round; P{owner} has the next response. If everyone passes without acting, {resolves_next}."
            )
        } else {
            format!(
                "Consecutive passes this round: {}. Still to respond: {}. When everyone remaining has passed, {resolves_next}.",
                passed.join(", "),
                if remaining.is_empty() {
                    "a new round has begun".to_string()
                } else {
                    remaining.join(", ")
                }
            )
        };
        out.push(Explanation {
            topic: "pass round".to_string(),
            kind: ExplanationKind::Rule,
            text,
            rules: vec!["117.3d", "117.4"],
        });
        if !state.stack.is_empty() {
            let top = state
                .stack
                .last()
                .map(|item| state.object(item.object).card.name())
                .unwrap_or_default();
            out.push(Explanation {
                    topic: "response window".to_string(),
                    kind: ExplanationKind::Rule,
                    text: format!(
                        "Open response window: any player may act while {top} is the top of the stack; it resolves only after all players pass in succession."
                    ),
                    rules: vec!["117.1a", "117.4"],
                });
        }
    }

    // Bounded-slice simplification, stated so the coaching never teaches it as
    // the rule: this engine assigns a blocked attacker's damage automatically,
    // in lethal-first order. CR 510.1c actually lets the attacking player divide
    // that damage among the blockers however they choose.
    if state.turn.step.is_combat() && !state.turn.attackers.is_empty() {
        out.push(Explanation {
            topic: "combat damage assignment".to_string(),
            kind: ExplanationKind::Simplification,
            text: format!(
                "This slice assigns combat damage automatically, in lethal-first order ({} attacker(s), {} declared blocker(s)). That is a bounded simplification of CR 510.1c, which lets the attacking player divide a blocked creature's damage among its blockers however they choose; typed damage assignment is not modeled yet.",
                state.turn.attackers.len(),
                state.turn.blockers.len()
            ),
            rules: vec!["510.1c", "510.2"],
        });
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
            kind: ExplanationKind::Rule,
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
            Pending::CombatDamageAssignment { .. } => vec!["510.1", "509.1h"],
            Pending::ApnapSacrifice { .. } => vec!["101.4", "701.21a"],
            Pending::ApnapReveal { .. } => vec!["101.4", "701.9a"],
            Pending::CommanderReturn { .. } => vec!["903.9a"],
            Pending::CommanderReplacement { .. } | Pending::CommanderDraw { .. } => {
                vec!["903.9b"]
            }
            Pending::TriggerTarget { .. } => vec!["603.3d"],
            Pending::TriggerChoice { .. } => vec!["603.5"],
            Pending::TriggerMode { .. } => vec!["603.3c", "700.2b"],
            Pending::TopCardChoice { .. } => vec!["701.20e"],
            Pending::SearchLibrary { .. } => vec!["701.23"],
        };
        let text = if pending.player() != viewer
            && matches!(
                pending,
                Pending::ApnapSacrifice { .. } | Pending::ApnapReveal { .. }
            ) {
            format!(
                "P{} has a pending choice; details are private to that seat.",
                pending.player()
            )
        } else {
            pending.describe()
        };
        let apnap_note = match pending {
            Pending::ApnapSacrifice { .. }
            | Pending::ApnapReveal { .. }
            | Pending::CommanderReturn { .. } => {
                " These choices are made in APNAP order: seats answer one at a time in turn order, starting with the active player (CR 101.4)."
            }
            _ => "",
        };
        out.push(Explanation {
            topic: "pending choice".to_string(),
            kind: ExplanationKind::Rule,
            text: format!("{text}{apnap_note}"),
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
                        kind: ExplanationKind::Rule,
                        text: err.reason(),
                        rules: vec!["305.2", "701.18a"],
                    });
                }
                continue;
            }
            if let Some(reason) = cast_blocker(game, player, id) {
                out.push(Explanation {
                    topic: format!("{} unavailable", card.name()),
                    kind: ExplanationKind::Rule,
                    text: reason,
                    rules: vec!["117.1a", "601.2c"],
                });
            }
        }
    }

    // CR 704.5f moves a creature with zero toughness to its owner's graveyard as
    // a state-based action — it never destroys it, and indestructible does not
    // stop that move — while CR 704.5g's lethal-damage destruction does not apply
    // to an indestructible permanent (CR 702.12b). Telling the player that such a
    // creature "will be destroyed" was a false statement either way.
    let mut lethal: Vec<String> = Vec::new();
    let mut zero_toughness: Vec<String> = Vec::new();
    let mut spared: Vec<String> = Vec::new();
    for o in state.objects.values() {
        if o.zone != crate::ids::Zone::Battlefield || !game.is_creature(o) {
            continue;
        }
        // CR 704.5f: zero or less toughness is a graveyard move by state-based
        // action — not a destroy, and not a consequence of damage.
        if o.toughness() <= 0 {
            zero_toughness.push(format!("{} ({} toughness)", o.name(), o.toughness()));
            continue;
        }
        if (o.damage as i32) < o.toughness() {
            continue;
        }
        let text = format!(
            "{} ({} damage, {} toughness)",
            o.name(),
            o.damage,
            o.toughness()
        );
        if game.has_indestructible(o.id) {
            spared.push(text);
        } else {
            lethal.push(text);
        }
    }
    if !lethal.is_empty() {
        out.push(Explanation {
            topic: "state-based actions".to_string(),
            kind: ExplanationKind::Rule,
            text: format!(
                "Waiting for state-based actions: {} has lethal damage and will be destroyed.",
                lethal.join(", ")
            ),
            rules: vec!["117.5", "704.5g"],
        });
    }
    if !zero_toughness.is_empty() {
        out.push(Explanation {
            topic: "state-based actions".to_string(),
            kind: ExplanationKind::Rule,
            text: format!(
                "Waiting for state-based actions: {} has zero or less toughness and is put into its owner's graveyard (CR 704.5f). That is not a destroy, so indestructible does not prevent it (CR 702.12b).",
                zero_toughness.join(", ")
            ),
            rules: vec!["117.5", "704.5f"],
        });
    }
    if !spared.is_empty() {
        out.push(Explanation {
            topic: "state-based actions".to_string(),
            kind: ExplanationKind::Rule,
            text: format!(
                "Waiting for state-based actions: {} has lethal damage but is indestructible, so the destroy does not apply (CR 702.12b).",
                spared.join(", ")
            ),
            rules: vec!["704.5g", "702.12b"],
        });
    }

    out.push(Explanation {
        topic: "board".to_string(),
        kind: ExplanationKind::Rule,
        text: format!(
            "Board (public counts): {}.",
            (0..state.seat_count() as PlayerId)
                .map(|p| {
                    let player = state.player(p);
                    let status = if player.lost { " [left the game]" } else { "" };
                    format!(
                        "P{p}{status}: {} life, {} permanent(s), {} card(s) in hand, {} in library",
                        player.life,
                        state.battlefield(p).len(),
                        player.hand_size(),
                        player.library.len()
                    )
                })
                .collect::<Vec<_>>()
                .join("; ")
        ),
        rules: vec!["401.3"],
    });

    out.push(Explanation {
        topic: "legal actions".to_string(),
        kind: ExplanationKind::Rule,
        text: format!(
            "{} legal action(s) generated for the current decision point.",
            legal.len()
        ),
        rules: vec!["117.1a"],
    });

    // Category separation (issue19): strategy text is never derived from the
    // engine, so the panel carries an explicit absence record instead of a
    // disguised rule claim.
    out.push(Explanation {
        topic: "strategy".to_string(),
        kind: ExplanationKind::Strategy,
        text: "No strategic suggestions are shown: this panel renders only engine-derived facts and pinned rule text. Any future strategy entry must carry the strategy label and is advice, not a rule or a win projection."
            .to_string(),
        rules: Vec::new(),
    });

    out
}
