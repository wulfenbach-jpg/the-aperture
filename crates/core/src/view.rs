//! Information-safe projections. `PlayerView` is the only thing a policy or a
//! human perspective is allowed to see: libraries are counts only (CR 401.2),
//! the opponent's hand is a count (CR 402.1), and hidden draws are masked in
//! the log.

use serde::Serialize;

use crate::action::LegalAction;
use crate::card::CardId;
use crate::digest;
use crate::engine::Game;
use crate::event::Event;
use crate::ids::{ObjectId, PlayerId, Zone};
use crate::state::{OrderMode, Outcome, Step};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CardView {
    pub id: ObjectId,
    pub card: CardId,
    pub name: &'static str,
    pub controller: PlayerId,
    pub tapped: bool,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub damage: u32,
    pub summoning_sick: bool,
    pub pump_power: i32,
    pub pump_toughness: i32,
    pub plus_one_counters: u32,
    /// Typed counters on this permanent (CR 122), public information.
    pub counters: Vec<(&'static str, u32)>,
    /// CR 111.1: true for a token permanent, which is not a card.
    pub is_token: bool,
    pub oracle: &'static str,
    pub cost: Option<&'static str>,
}

/// What the engine is waiting for, typed (a policy must not parse prose).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PendingKind {
    Mulligan,
    BottomCards {
        count: u8,
    },
    DeclareAttackers,
    DeclareBlockers,
    OrderCard {
        index: usize,
        mode: OrderMode,
    },
    Discard {
        count: u8,
    },
    /// CR 510.1: one blocked attacker is waiting for its controller's
    /// "assign as though it weren't blocked" answer. The attacker stays a public
    /// object, so the projection carries its id and card name.
    CombatDamageAssignment {
        attacker: ObjectId,
        card: CardId,
    },
    ApnapSacrifice,
    ApnapReveal,
    CommanderReturn,
    CommanderReplacement,
    CommanderDraw,
    TriggerTarget,
    TriggerChoice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StackView {
    pub object: ObjectId,
    pub card: CardId,
    pub name: &'static str,
    pub controller: PlayerId,
    pub targets: Vec<String>,
    /// Public choices announced when casting a modal spell, in printed order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modes: Vec<&'static str>,
}

/// Public command-zone information. The registry's current ObjectId is NOT
/// projected; only a commander actually in the public command zone appears.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommanderView {
    pub id: crate::ids::CommanderId,
    pub owner: PlayerId,
    pub card: CardId,
    pub name: &'static str,
    /// `Some` only for public zones. Hand and library are always redacted for
    /// every viewer; the stable CommanderId/owner/card/tax stay public.
    pub zone: Option<Zone>,
    pub casts_from_command_zone: u32,
    pub tax: u32,
}

/// Public cumulative commander-damage row (CR 903.10a); commander identities
/// and damage totals are public information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CommanderDamageView {
    pub commander: crate::ids::CommanderId,
    pub recipient: PlayerId,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpponentView {
    pub seat: PlayerId,
    pub name: String,
    pub deck_name: String,
    pub eliminated: bool,
    pub life: i32,
    pub hand_count: usize,
    pub library_count: usize,
    pub battlefield: Vec<CardView>,
    pub graveyard: Vec<CardView>,
    pub exile: Vec<CardView>,
    pub command_zone: Vec<CardView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerView {
    pub viewer: PlayerId,
    pub viewer_name: String,
    pub seat_count: usize,
    /// Every other seat, in seat order, including eliminated seats. Hidden
    /// zones are counts only; eliminated seats project no removed objects.
    pub opponents: Vec<OpponentView>,
    pub own_hand: Vec<CardView>,
    pub own_battlefield: Vec<CardView>,
    pub own_graveyard: Vec<CardView>,
    pub own_exile: Vec<CardView>,
    pub own_command_zone: Vec<CardView>,
    /// Cards revealed from a hidden zone (CR 701.16a). Public to every viewer
    /// while they stay in that hidden zone; no other hidden card appears here.
    pub revealed: Vec<CardView>,
    /// Public stable commander identities and command-zone cast tax.
    pub commanders: Vec<CommanderView>,
    /// Public cumulative commander combat damage (CR 903.10a).
    pub commander_damage: Vec<CommanderDamageView>,
    pub own_library_count: usize,
    pub own_life: i32,
    /// Whether the viewing seat has left the game.
    pub own_eliminated: bool,
    pub own_mana: String,
    pub own_mana_pool: crate::mana::ManaPool,
    pub own_lands_in_hand: usize,
    pub own_mulligans: u8,
    /// Public combat state (CR 508/509: declarations are public).
    pub attackers: Vec<ObjectId>,
    pub blockers: Vec<(ObjectId, ObjectId)>,
    /// CR 509.1g: a blocker whose attacker has left combat is still a blocking
    /// creature until it is removed from combat or the combat phase ends, so it
    /// has no pair to report in `blockers`. It is public combat state exactly
    /// like the pairs above and is listed separately rather than invented as a
    /// pair with a departed attacker.
    pub unpaired_blockers: Vec<ObjectId>,
    pub stack: Vec<StackView>,
    pub turn_number: u32,
    pub active: PlayerId,
    pub step: Step,
    pub priority: Option<PlayerId>,
    pub pending: Option<String>,
    /// Seat that must answer the pending decision, if any. Public information;
    /// the decision's contents remain masked per viewer.
    pub pending_owner: Option<PlayerId>,
    /// Typed form of `pending` for policies.
    pub pending_kind: Option<PendingKind>,
    /// Index (0-based) of the card currently being decided in a scry/surveil.
    pub pending_index: Option<usize>,
    pub pending_mode: Option<OrderMode>,
    /// Cards the viewer is currently looking at because of a scry/surveil they
    /// own (CR 701.22a). Empty for every other seat.
    pub pending_viewed: Vec<CardView>,
    pub legal: Vec<LegalAction>,
    pub log: Vec<String>,
    pub outcome: Option<Outcome>,
    /// Digest of this masked projection only. It is not the full engine-state
    /// digest and must never be used as an oracle over hidden zones.
    pub digest: String,
    pub rules_revision: String,
}

impl PlayerView {
    /// Serialize for a model or a log. Contains no library contents and no
    /// other seat's hand contents.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("view serializes")
    }

    /// The projected public state of one other seat.
    pub fn opponent_view(&self, seat: PlayerId) -> Option<&OpponentView> {
        self.opponents.iter().find(|o| o.seat == seat)
    }

    /// Convenience for the two-seat practice UI/tests. Returns `None` when the
    /// game has more than one other seat.
    pub fn sole_opponent(&self) -> Option<&OpponentView> {
        (self.opponents.len() == 1).then(|| &self.opponents[0])
    }

    pub fn terrains(&self, p: PlayerId) -> Option<&Vec<CardView>> {
        if p == self.viewer {
            Some(&self.own_battlefield)
        } else {
            self.opponent_view(p).map(|o| &o.battlefield)
        }
    }
}

fn card_view(game: &Game, id: ObjectId) -> CardView {
    let obj = game.state.object(id);
    let def = obj.card.def();
    CardView {
        id,
        card: obj.card,
        name: def.name,
        controller: obj.controller,
        tapped: obj.tapped,
        // Characteristics, not printed values: counters, pumps and an
        // animation that adds the creature type (Stuffed Bear) included.
        power: game.is_creature(obj).then(|| obj.power()),
        toughness: game.is_creature(obj).then(|| obj.toughness()),
        damage: obj.damage,
        summoning_sick: obj.summoning_sick,
        pump_power: obj.pump_power,
        pump_toughness: obj.pump_toughness,
        plus_one_counters: obj.plus_one_counters,
        counters: obj
            .counters
            .iter()
            .map(|(kind, count)| (kind.name(), count))
            .collect(),
        is_token: obj.is_token(),
        oracle: def.oracle,
        cost: def.cost,
    }
}

fn zone_cards(game: &Game, p: PlayerId, zone: Zone) -> Vec<CardView> {
    let ids: Vec<ObjectId> = match zone {
        Zone::Graveyard => game.state.player(p).graveyard.clone(),
        Zone::Exile => game.state.player(p).exile.clone(),
        Zone::Battlefield => game
            .state
            .battlefield(p)
            .into_iter()
            .map(|o| o.id)
            .collect(),
        Zone::Hand => game.state.player(p).hand.clone(),
        Zone::Command => game.state.player(p).command_zone.clone(),
        Zone::Library | Zone::Stack | Zone::Removed => Vec::new(),
    };
    ids.into_iter().map(|id| card_view(game, id)).collect()
}

/// Render one event from `viewer`'s perspective, masking hidden information.
pub fn render_event(game: &Game, viewer: PlayerId, event: &Event) -> String {
    match event {
        Event::GameStart { first, .. } => {
            // The replay seed is deliberately NOT rendered: with public deck
            // lists it would reconstruct both hidden libraries.
            format!("Game start — P{first} plays first")
        }
        Event::Mulligan { player, count } => {
            format!("P{player} mulligans (mulligan count {count})")
        }
        Event::KeepHand { player, size } => format!("P{player} keeps a hand of {size}"),
        Event::Bottom { player, count } => {
            format!("P{player} puts {count} card(s) on the bottom of their library")
        }
        Event::TurnBegin { turn, active } => format!("— Turn {turn}: P{active} —"),
        Event::CombatDamageAssignment {
            player,
            attacker,
            card,
            as_though_unblocked,
        } => format!(
            "P{player} has {} ({attacker}) assign its combat damage {}{}",
            card.name(),
            if *as_though_unblocked {
                "as though it weren't blocked"
            } else {
                "normally"
            },
            " (CR 510.1)"
        ),
        Event::StepBegin { step } => format!("[{}]", step.name()),
        Event::Priority { player } => format!("P{player} receives priority"),
        Event::Pass { player } => format!("P{player} passes"),
        Event::LandPlayed { player, card, .. } => {
            format!("P{player} plays {}", card.name())
        }
        Event::ManaTapped {
            player,
            land,
            color,
        } => {
            let name = game
                .state
                .try_object(*land)
                .map(|o| o.card.name())
                .unwrap_or("land");
            format!("P{player} taps {name} for {}", color.symbol())
        }
        Event::ManaPoolEmptied { player, amount } => {
            format!("P{player}'s pool of {amount} mana empties (CR 500.5)")
        }
        Event::Cast {
            player,
            card,
            targets,
            ..
        } => {
            if targets.is_empty() {
                format!("P{player} casts {}", card.name())
            } else {
                format!(
                    "P{player} casts {} targeting {}",
                    card.name(),
                    targets.join(", ")
                )
            }
        }
        Event::Resolve {
            controller, card, ..
        } => {
            format!("{} resolves (controlled by P{controller})", card.name())
        }
        Event::PermanentEntered {
            controller, card, ..
        } => format!("{} enters the battlefield under P{controller}", card.name()),
        Event::NoLegalTargets { card, reason } => {
            format!("{} does not resolve: {reason}", card.name())
        }
        Event::Countered { card, by, .. } => {
            format!("{} is countered by {}", card.name(), by.name())
        }
        Event::Damage { target, amount, .. } => format!("{amount} damage to {target}"),
        Event::LifeChange {
            player,
            delta,
            life,
        } => {
            let verb = if *delta > 0 { "gains" } else { "loses" };
            format!("P{player} {verb} {} life (now {life})", delta.abs())
        }
        Event::Milled { player, card, .. } => {
            // A graveyard is a public zone (CR 400.2), so the card name is
            // rendered for every viewer, unlike a draw.
            format!("P{player} mills {}", card.name())
        }
        Event::ManaAdded {
            player,
            color,
            amount,
            pool_total,
        } => {
            if *player == viewer {
                format!(
                    "P{player} adds {amount} {} to their mana pool (now {pool_total})",
                    color.symbol()
                )
            } else {
                format!(
                    "P{player} adds {amount} {} to their mana pool",
                    color.symbol()
                )
            }
        }
        Event::Draw { player, card, .. } => {
            if *player == viewer {
                format!("P{player} draws {}", card.name())
            } else {
                format!("P{player} draws a card")
            }
        }
        Event::DrewFromEmpty { player } => {
            format!("P{player} attempted to draw from an empty library (CR 704.5b)")
        }
        Event::Scried { player, count } => format!("P{player} scries {count}"),
        Event::Surveilled { player, count } => format!("P{player} surveils {count}"),
        Event::OrderedCard {
            player,
            card,
            to_top,
            mode,
        } => {
            let public_in_graveyard = !*to_top && *mode == OrderMode::Surveil;
            if *player == viewer || public_in_graveyard {
                format!(
                    "{} is put {}",
                    card.name(),
                    match (*mode, *to_top) {
                        (_, true) => "on top of the library",
                        (OrderMode::Scry, false) => "on the bottom of the library",
                        (OrderMode::Surveil, false) => "into the graveyard",
                    }
                )
            } else {
                format!("P{player} keeps a card it looked at (contents hidden)")
            }
        }
        Event::Discarded { player, card } => {
            if *player == viewer {
                format!("P{player} discards {}", card.name())
            } else {
                format!("P{player} discards a card")
            }
        }
        Event::CardsRevealed {
            player,
            cards,
            count,
        } => {
            if cards.is_empty() {
                format!("P{player} reveals no creature cards")
            } else {
                let names: Vec<&'static str> = cards
                    .iter()
                    .map(|id| {
                        game.state
                            .try_object(*id)
                            .map(|object| object.card.name())
                            .unwrap_or("a card")
                    })
                    .collect();
                format!(
                    "P{player} reveals {count} creature card(s) from hand: {}",
                    names.join(", ")
                )
            }
        }
        Event::SacrificeChosen { player, permanent } => match permanent {
            Some(id) => format!(
                "P{player} chooses {} to sacrifice",
                game.state
                    .try_object(*id)
                    .map(|object| object.card.name())
                    .unwrap_or("a permanent")
            ),
            None => format!("P{player} has nothing to sacrifice"),
        },
        Event::AttackersDeclared { player, attackers } => {
            if attackers.is_empty() {
                format!("P{player} declares no attackers")
            } else {
                let names: Vec<&str> = attackers
                    .iter()
                    .filter_map(|id| game.state.try_object(*id))
                    .map(|o| o.card.name())
                    .collect();
                format!("P{player} attacks with {}", names.join(", "))
            }
        }
        Event::BlockersDeclared { player, blocks } => {
            if blocks.is_empty() {
                format!("P{player} declares no blockers")
            } else {
                format!("P{player} declares {} blocker(s)", blocks.len())
            }
        }
        Event::CombatDamage {
            amount, blocker, ..
        } => match blocker {
            Some((_, card)) => format!("{amount} combat damage to {}", card.name()),
            None => format!("{amount} combat damage to a player"),
        },
        Event::CommanderDamage {
            commander,
            recipient,
            amount,
            total,
            ..
        } => format!(
            "{amount} commander combat damage from {commander} to P{recipient} (cumulative {total}, CR 903.10a)"
        ),
        Event::PreventAllCombatDamage { turn, card } => {
            format!(
                "{} prevents all combat damage this turn (turn {turn})",
                card.name()
            )
        }
        Event::CombatDamagePrevented {
            amount, recipient, ..
        } => match recipient {
            Some(player) => format!("{amount} combat damage to P{player} is prevented"),
            None => format!("{amount} combat damage to a permanent is prevented"),
        },
        Event::DamageShieldCreated {
            player,
            card,
            target,
            amount,
            combat_only,
        } => {
            let scope = if *combat_only {
                "combat damage only"
            } else {
                "any damage"
            };
            match amount {
                Some(n) => format!(
                    "P{player} shields {target} with {} (prevent the next {n} damage, {scope}, CR 615)",
                    card.name()
                ),
                None => format!(
                    "P{player} shields {target} with {} (prevent all damage this turn, {scope}, CR 615)",
                    card.name()
                ),
            }
        }
        Event::DamagePrevented {
            card,
            target,
            amount,
            remaining,
            combat,
            ..
        } => {
            let kind = if *combat { "combat damage" } else { "damage" };
            match remaining {
                Some(0) => format!(
                    "{} prevents {amount} {kind} to {target}; the shield is used up (CR 615)",
                    card.name()
                ),
                Some(left) => format!(
                    "{} prevents {amount} {kind} to {target}; {left} prevention left this turn (CR 615)",
                    card.name()
                ),
                None => format!(
                    "{} prevents {amount} {kind} to {target} (CR 615)",
                    card.name()
                ),
            }
        }
        Event::ReplacementApplied {
            card,
            controller,
            chooser,
            target,
            before,
            after,
        } => format!(
            "{} replaces {target}'s {before} damage with {after} (P{controller} controls it, P{chooser} chose the order, CR 614.1a/616.1)",
            card.name()
        ),
        Event::ContinuousEffectStarted {
            card, layer, text, ..
        } => format!(
            "{} starts a continuous effect in layer {layer}: {text}",
            card.name()
        ),
        Event::Destroyed { card, .. } => format!("{} is put into a graveyard", card.name()),
        Event::AbilityActivated {
            player,
            card,
            text,
            targets,
            ..
        } => {
            if targets.is_empty() {
                format!("P{player} activates {}: {text}", card.name())
            } else {
                format!(
                    "P{player} activates {}: {text} (targeting {})",
                    card.name(),
                    targets.join(", ")
                )
            }
        }
        Event::TriggerStaged {
            controller,
            card,
            text,
            ..
        } => format!(
            "P{controller}'s triggered ability from {} goes on the stack: {text}",
            card.name()
        ),
        Event::TriggerRemoved {
            controller,
            card,
            reason,
        } => format!(
            "P{controller}'s {} trigger is removed: {reason}",
            card.name()
        ),
        Event::TriggerDeclined { controller, card } => {
            format!("P{controller} declines {}'s optional trigger", card.name())
        }
        Event::AbilityResolved {
            controller,
            card,
            text,
            ..
        } => format!(
            "P{controller}'s ability from {} resolves: {text}",
            card.name()
        ),
        Event::AbilityFizzled {
            controller,
            card,
            reason,
        } => format!("P{controller}'s {} ability fizzles: {reason}", card.name()),
        Event::StateBasedAction { description } => format!("state-based action: {description}"),
        Event::SandboxOverride {
            player,
            kind,
            detail,
            private_cards,
            ..
        } => {
            let mut text = format!("SANDBOX P{player} {}: {detail}", kind.label());
            // Only the seat that looked at the cards sees them. Every other
            // seat gets the public shape of the override and nothing else.
            if *player == viewer && !private_cards.is_empty() {
                let names: Vec<&str> = private_cards.iter().map(|card| card.name()).collect();
                text.push_str(&format!(" — you looked at: {}", names.join(", ")));
            }
            text
        }
        Event::GameOver { winner, reason } => format!("Game over — P{winner} wins: {reason}"),
        Event::GameDrawn { reason } => format!("Game drawn — {reason}"),
        Event::Truncated { reason } => format!("TRUNCATED: {reason}"),
    }
}

/// How many recent events a view carries. Bounded so a model request cannot
/// grow with game length.
pub const LOG_LINES: usize = 40;

impl Game {
    /// Projection for one seat. `log` is the session's event trace; it is
    /// rendered with viewer-aware masking.
    pub fn view_for(&self, viewer: PlayerId, log: &[Event]) -> PlayerView {
        let me = self.state.player(viewer);
        let pending_viewed = match &self.state.pending {
            Some(crate::state::Pending::SearchLibrary {
                player,
                looked,
                mode,
                ..
            }) if *player == viewer
                && *mode
                    == crate::state::LibrarySearchMode::CreatureFromTopToBattlefieldAttacking =>
            {
                looked.iter().map(|id| card_view(self, *id)).collect()
            }
            Some(crate::state::Pending::OrderCard { player, viewed, .. }) if *player == viewer => {
                viewed.iter().map(|id| card_view(self, *id)).collect()
            }
            Some(crate::state::Pending::TopCardChoice { player, card, .. })
                if *player == viewer =>
            {
                vec![card_view(self, *card)]
            }
            _ => Vec::new(),
        };
        let pending_for_viewer = self.state.pending.as_ref().filter(|pending| match pending {
            crate::state::Pending::ApnapSacrifice { player, .. }
            | crate::state::Pending::ApnapReveal { player, .. }
            | crate::state::Pending::TopCardChoice { player, .. }
            | crate::state::Pending::SearchLibrary { player, .. } => *player == viewer,
            _ => true,
        });
        let rendered: Vec<String> = if log.len() > LOG_LINES {
            log[log.len() - LOG_LINES..]
                .iter()
                .map(|e| render_event(self, viewer, e))
                .collect()
        } else {
            log.iter().map(|e| render_event(self, viewer, e)).collect()
        };
        let opponents = (0..self.state.seat_count() as PlayerId)
            .filter(|seat| *seat != viewer)
            .map(|seat| {
                let player = self.state.player(seat);
                if player.lost {
                    OpponentView {
                        seat,
                        name: player.name.clone(),
                        deck_name: player.deck_name.clone(),
                        eliminated: true,
                        life: player.life,
                        hand_count: 0,
                        library_count: 0,
                        battlefield: Vec::new(),
                        graveyard: Vec::new(),
                        exile: Vec::new(),
                        command_zone: Vec::new(),
                    }
                } else {
                    OpponentView {
                        seat,
                        name: player.name.clone(),
                        deck_name: player.deck_name.clone(),
                        eliminated: false,
                        life: player.life,
                        hand_count: player.hand_size(),
                        library_count: player.library.len(),
                        battlefield: zone_cards(self, seat, Zone::Battlefield),
                        graveyard: zone_cards(self, seat, Zone::Graveyard),
                        exile: zone_cards(self, seat, Zone::Exile),
                        command_zone: zone_cards(self, seat, Zone::Command),
                    }
                }
            })
            .collect();
        let mut view = PlayerView {
            viewer,
            viewer_name: me.name.clone(),
            seat_count: self.state.seat_count(),
            opponents,
            own_hand: zone_cards(self, viewer, Zone::Hand),
            own_battlefield: zone_cards(self, viewer, Zone::Battlefield),
            own_graveyard: zone_cards(self, viewer, Zone::Graveyard),
            own_exile: zone_cards(self, viewer, Zone::Exile),
            own_command_zone: zone_cards(self, viewer, Zone::Command),
            revealed: self
                .state
                .revealed_hidden()
                .into_iter()
                .map(|id| card_view(self, id))
                .collect(),
            commanders: self
                .state
                .commanders
                .iter()
                .map(|commander| CommanderView {
                    id: commander.id,
                    owner: commander.owner,
                    card: commander.card,
                    name: commander.card.name(),
                    zone: match self.state.object(commander.object).zone {
                        Zone::Command
                        | Zone::Battlefield
                        | Zone::Stack
                        | Zone::Graveyard
                        | Zone::Exile => Some(self.state.object(commander.object).zone),
                        Zone::Hand | Zone::Library | Zone::Removed => None,
                    },
                    casts_from_command_zone: commander.casts_from_command_zone,
                    tax: commander.casts_from_command_zone.saturating_mul(2),
                })
                .collect(),
            commander_damage: self
                .state
                .commander_damage
                .iter()
                .map(|entry| CommanderDamageView {
                    commander: entry.commander,
                    recipient: entry.recipient,
                    amount: entry.amount,
                })
                .collect(),
            own_library_count: me.library.len(),
            own_life: me.life,
            own_eliminated: me.lost,
            own_mana: me.mana.render(),
            own_mana_pool: me.mana,
            own_lands_in_hand: me
                .hand
                .iter()
                .filter(|id| self.state.object(**id).card.def().is_land())
                .count(),
            own_mulligans: me.mulligans,
            attackers: self.state.turn.attackers.clone(),
            blockers: self.state.turn.blockers.clone(),
            unpaired_blockers: self.state.turn.unpaired_blockers.clone(),
            stack: self
                .state
                .stack
                .iter()
                .map(|item| StackView {
                    object: item.object,
                    // The recorded card, not the source's current object: an
                    // ability can outlive its source's zone change.
                    card: item.card,
                    name: item.card.name(),
                    controller: item.controller,
                    modes: {
                        let mut modes = item.modes.clone();
                        modes.sort_unstable();
                        modes
                            .into_iter()
                            .filter_map(|mode| crate::card::modal_mode_name(item.card, mode))
                            .collect()
                    },
                    targets: item
                        .targets
                        .iter()
                        .map(|t| self.describe_target(*t))
                        .collect(),
                })
                .collect(),
            turn_number: self.state.turn.number,
            active: self.state.turn.active,
            step: self.state.turn.step,
            priority: self.state.turn.priority,
            pending: pending_for_viewer.map(|p| p.describe()),
            pending_owner: self.state.decision_owner(),
            pending_kind: pending_for_viewer.map(|p| match p {
                crate::state::Pending::Mulligan { .. } => PendingKind::Mulligan,
                crate::state::Pending::BottomCards { count, .. } => {
                    PendingKind::BottomCards { count: *count }
                }
                crate::state::Pending::DeclareAttackers { .. } => PendingKind::DeclareAttackers,
                crate::state::Pending::DeclareBlockers { .. } => PendingKind::DeclareBlockers,
                crate::state::Pending::OrderCard { index, mode, .. } => PendingKind::OrderCard {
                    index: *index,
                    mode: *mode,
                },
                crate::state::Pending::Discard { count, .. } => {
                    PendingKind::Discard { count: *count }
                }
                crate::state::Pending::ApnapSacrifice { .. } => PendingKind::ApnapSacrifice,
                crate::state::Pending::ApnapReveal { .. } => PendingKind::ApnapReveal,
                crate::state::Pending::CombatDamageAssignment { order, cursor, .. } => {
                    PendingKind::CombatDamageAssignment {
                        attacker: order[*cursor].attacker,
                        card: self.state.object(order[*cursor].attacker).card,
                    }
                }
                crate::state::Pending::CommanderReturn { .. } => PendingKind::CommanderReturn,
                crate::state::Pending::CommanderReplacement { .. } => {
                    PendingKind::CommanderReplacement
                }
                crate::state::Pending::CommanderDraw { .. } => PendingKind::CommanderDraw,
                crate::state::Pending::TriggerTarget { .. } => PendingKind::TriggerTarget,
                crate::state::Pending::TriggerChoice { .. }
                | crate::state::Pending::TriggerMode { .. }
                | crate::state::Pending::TopCardChoice { .. }
                | crate::state::Pending::SearchLibrary { .. } => PendingKind::TriggerChoice,
            }),
            pending_index: match &self.state.pending {
                Some(crate::state::Pending::OrderCard { player, index, .. })
                    if *player == viewer =>
                {
                    Some(*index)
                }
                _ => None,
            },
            pending_mode: match &self.state.pending {
                Some(crate::state::Pending::OrderCard { player, mode, .. })
                    if *player == viewer =>
                {
                    Some(*mode)
                }
                _ => None,
            },
            pending_viewed,
            // Legal actions are only projected to the seat that must act: their
            // labels describe that seat's hidden cards (hand contents, scryed
            // cards), so handing them to another seat would leak.
            legal: if self.state.decision_owner() == Some(viewer) {
                self.legal_actions()
            } else {
                Vec::new()
            },
            log: rendered,
            outcome: self.state.outcome.clone(),
            digest: String::new(),
            rules_revision: self.state.rules_revision.clone(),
        };
        view.digest = masked_digest(&view);
        view
    }
}

/// Digest over the serialized masked projection only. The `digest` field is
/// empty while hashing, so the result is deterministic and cannot encode hidden
/// identifiers, contents or library order.
fn masked_digest(view: &PlayerView) -> String {
    let json = serde_json::to_string(view).expect("view serializes");
    digest::digest_bytes(json.as_bytes())
        .chars()
        .take(12)
        .collect()
}

/// Omniscient postgame review text. This reads the FULL engine state — every
/// hand, library and hidden zone — and is therefore only ever rendered inside
/// the explicit postgame review mode, never as a live or policy input. The
/// caller gates it on a finished game; the function re-checks and refuses to
/// render anything while the game is still live, so a live seat cannot use it
/// to see hidden cards.
pub fn render_review(game: &Game) -> Result<String, String> {
    if game.state.outcome.is_none() {
        return Err(
            "postgame review is only available once the game has actually ended".to_string(),
        );
    }
    let state = &game.state;
    let mut text = String::new();
    text.push_str(
        "POSTGAME OMNISCIENT REVIEW — shown because the game is over; this is not a policy view\n",
    );
    text.push_str(&format!("rules revision {}\n", state.rules_revision));
    for player in &state.players {
        let zone_names = |ids: &[ObjectId]| -> Vec<String> {
            ids.iter()
                .map(|id| {
                    state
                        .try_object(*id)
                        .map(|object| object.name().to_string())
                        .unwrap_or_else(|| format!("object #{id}"))
                })
                .collect()
        };
        text.push_str(&format!(
            "\n--- P{} '{}' — life {}{} ---\n",
            player.id,
            player.name,
            player.life,
            if player.lost { " (left the game)" } else { "" }
        ));
        text.push_str(&format!(
            "hand ({}): {}\n",
            player.hand.len(),
            renderer_join(&zone_names(&player.hand))
        ));
        text.push_str(&format!(
            "library ({}): {}\n",
            player.library.len(),
            renderer_join(&zone_names(&player.library))
        ));
        text.push_str(&format!(
            "graveyard ({}): {}\n",
            player.graveyard.len(),
            renderer_join(&zone_names(&player.graveyard))
        ));
        text.push_str(&format!(
            "exile ({}): {}\n",
            player.exile.len(),
            renderer_join(&zone_names(&player.exile))
        ));
        if state.profile.has_command_zone() {
            text.push_str(&format!(
                "command zone ({}): {}\n",
                player.command_zone.len(),
                renderer_join(&zone_names(&player.command_zone))
            ));
        }
    }
    if state.stack.is_empty() {
        text.push_str("\nstack: empty at the recorded end\n");
    } else {
        text.push_str(&format!(
            "\nstack at the recorded end: {} item(s)\n",
            state.stack.len()
        ));
    }
    Ok(text)
}

fn renderer_join(names: &[String]) -> String {
    if names.is_empty() {
        "(none)".to_string()
    } else {
        names.join(", ")
    }
}

/// One search hit from the viewer's own masked projection. `zone` is where the
/// viewer may legally see the card right now; hidden opponent hand/library cards
/// never appear because they are not in the projection at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ZoneHit {
    pub seat: PlayerId,
    pub zone: Zone,
    pub card: CardView,
}

/// Zones in the order a table inspector lists them.
pub const ZONE_ORDER: [Zone; 5] = [
    Zone::Hand,
    Zone::Battlefield,
    Zone::Graveyard,
    Zone::Exile,
    Zone::Command,
];

impl PlayerView {
    /// Where this viewer may legally see `id`, or `None` when the object is not
    /// projected at all (another seat's hand or library, a face-down card, the
    /// stack, or an object that left the game). This reads only the masked
    /// projection, so a shared inspector can never invent or query a hidden
    /// location, and a command-zone card is never mislabeled as a battlefield.
    pub fn zone_of(&self, id: ObjectId) -> Option<Zone> {
        if self.own_hand.iter().any(|card| card.id == id) {
            return Some(Zone::Hand);
        }
        if self.own_battlefield.iter().any(|card| card.id == id) {
            return Some(Zone::Battlefield);
        }
        if self.own_graveyard.iter().any(|card| card.id == id) {
            return Some(Zone::Graveyard);
        }
        if self.own_exile.iter().any(|card| card.id == id) {
            return Some(Zone::Exile);
        }
        if self.own_command_zone.iter().any(|card| card.id == id) {
            return Some(Zone::Command);
        }
        for opponent in &self.opponents {
            for (zone, cards) in [
                (Zone::Battlefield, &opponent.battlefield),
                (Zone::Graveyard, &opponent.graveyard),
                (Zone::Exile, &opponent.exile),
                (Zone::Command, &opponent.command_zone),
            ] {
                if cards.iter().any(|card| card.id == id) {
                    return Some(zone);
                }
            }
        }
        None
    }

    /// Every zone this viewer may inspect for `seat`, in table order. A seat
    /// other than the viewer projects public zones only; its hand and library
    /// are counts, so they are returned as empty lists rather than invented
    /// contents.
    pub fn zones_for(&self, seat: PlayerId) -> Vec<(Zone, Vec<CardView>)> {
        if seat == self.viewer {
            vec![
                (Zone::Hand, self.own_hand.clone()),
                (Zone::Battlefield, self.own_battlefield.clone()),
                (Zone::Graveyard, self.own_graveyard.clone()),
                (Zone::Exile, self.own_exile.clone()),
                (Zone::Command, self.own_command_zone.clone()),
            ]
        } else if let Some(opponent) = self.opponent_view(seat) {
            vec![
                (Zone::Hand, Vec::new()),
                (Zone::Battlefield, opponent.battlefield.clone()),
                (Zone::Graveyard, opponent.graveyard.clone()),
                (Zone::Exile, opponent.exile.clone()),
                (Zone::Command, opponent.command_zone.clone()),
            ]
        } else {
            Vec::new()
        }
    }

    /// Case-insensitive search over the projected cards: name, printed text and
    /// mana cost. `seats` limits the search to those seats (empty means every
    /// seat); `zones` limits it to those zones. Results are ordered by seat,
    /// then table zone order, then name, so the inspector is stable.
    pub fn search(&self, query: &str, seats: &[PlayerId], zones: &[Zone]) -> Vec<ZoneHit> {
        let needle = query.trim().to_lowercase();
        let mut hits = Vec::new();
        for seat in 0..self.seat_count as PlayerId {
            if !seats.is_empty() && !seats.contains(&seat) {
                continue;
            }
            for (zone, cards) in self.zones_for(seat) {
                if !zones.is_empty() && !zones.contains(&zone) {
                    continue;
                }
                for card in cards {
                    let matches = needle.is_empty()
                        || card.name.to_lowercase().contains(&needle)
                        || card.oracle.to_lowercase().contains(&needle)
                        || card
                            .cost
                            .is_some_and(|cost| cost.to_lowercase().contains(&needle));
                    if matches {
                        hits.push(ZoneHit { seat, zone, card });
                    }
                }
            }
        }
        hits.sort_by(|a, b| {
            a.seat
                .cmp(&b.seat)
                .then_with(|| zone_rank(a.zone).cmp(&zone_rank(b.zone)))
                .then_with(|| a.card.name.cmp(b.card.name))
        });
        hits
    }
}

fn zone_rank(zone: Zone) -> usize {
    ZONE_ORDER
        .iter()
        .position(|candidate| *candidate == zone)
        .unwrap_or(ZONE_ORDER.len())
}
