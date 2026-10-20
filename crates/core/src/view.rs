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
    pub oracle: &'static str,
    pub cost: Option<&'static str>,
}

/// What the engine is waiting for, typed (a policy must not parse prose).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PendingKind {
    Mulligan,
    BottomCards { count: u8 },
    DeclareAttackers,
    DeclareBlockers,
    OrderCard { index: usize, mode: OrderMode },
    Discard { count: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StackView {
    pub object: ObjectId,
    pub card: CardId,
    pub name: &'static str,
    pub controller: PlayerId,
    pub targets: Vec<String>,
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
    pub own_library_count: usize,
    pub own_life: i32,
    pub own_mana: String,
    pub own_mana_pool: crate::mana::ManaPool,
    pub own_lands_in_hand: usize,
    pub own_mulligans: u8,
    /// Public combat state (CR 508/509: declarations are public).
    pub attackers: Vec<ObjectId>,
    pub blockers: Vec<(ObjectId, ObjectId)>,
    pub stack: Vec<StackView>,
    pub turn_number: u32,
    pub active: PlayerId,
    pub step: Step,
    pub priority: Option<PlayerId>,
    pub pending: Option<String>,
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
        power: def.power.map(|p| p + obj.pump_power),
        toughness: def.toughness.map(|t| t + obj.pump_toughness),
        damage: obj.damage,
        summoning_sick: obj.summoning_sick,
        pump_power: obj.pump_power,
        pump_toughness: obj.pump_toughness,
        plus_one_counters: obj.plus_one_counters,
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
        Event::Destroyed { card, .. } => format!("{} is put into a graveyard", card.name()),
        Event::StateBasedAction { description } => format!("state-based action: {description}"),
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
            Some(crate::state::Pending::OrderCard { player, viewed, .. }) if *player == viewer => {
                viewed.iter().map(|id| card_view(self, *id)).collect()
            }
            _ => Vec::new(),
        };
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
            own_library_count: me.library.len(),
            own_life: me.life,
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
            stack: self
                .state
                .stack
                .iter()
                .map(|item| StackView {
                    object: item.object,
                    card: self.state.object(item.object).card,
                    name: self.state.object(item.object).card.name(),
                    controller: item.controller,
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
            pending: self.state.pending.as_ref().map(|p| p.describe()),
            pending_kind: self.state.pending.as_ref().map(|p| match p {
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
