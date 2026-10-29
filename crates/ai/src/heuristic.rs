//! Deterministic baseline heuristic. It is deliberately shallow, but it does
//! play: it makes lands, casts creatures and removal, counters opponent spells,
//! attacks when the attack is profitable, blocks when the trade is good, and
//! answers scry/surveil, mulligan and discard choices.
//!
//! It sees only `PlayerView` (masked), and every action it returns is still
//! validated by the engine before it mutates anything.

use aperture_core::catalog;
use aperture_core::mana::{ManaCost, ManaPool};
use aperture_core::state::Step;
use aperture_core::view::{CardView, PendingKind, PlayerView, StackView};
use aperture_core::{Action, BlockDecl, CardId, ManaColor, ObjectId, PlayerId, Target};

use crate::{Decision, Policy};

/// Mana value of a card in the supported pool.
pub fn card_mana_value(card: CardId) -> u8 {
    card.def()
        .mana_cost()
        .map(|cost| cost.mana_value())
        .unwrap_or(0)
}

/// Mana the seat could use now: its pool plus one per untapped basic land.
pub fn available_mana(view: &PlayerView) -> ManaPool {
    let mut pool = view.own_mana_pool;
    for card in &view.own_battlefield {
        if card.tapped {
            continue;
        }
        if let Some(color) = card.card.def().basic_mana {
            pool.add(color, 1);
        }
    }
    pool
}

fn affordable(view: &PlayerView, cost: &ManaCost) -> bool {
    let mut pool = available_mana(view);
    for pip in &cost.pips {
        if pool.take(*pip, 1) == 0 {
            return false;
        }
    }
    pool.total() >= u32::from(cost.generic)
}

fn cost_of(card: CardId) -> ManaCost {
    card.def().mana_cost().unwrap_or_else(ManaCost::empty)
}

fn is_creature(card: &CardView) -> bool {
    card.card.def().is_creature()
}

fn lands_in_play(view: &PlayerView) -> usize {
    view.own_battlefield
        .iter()
        .filter(|c| c.card.def().is_land())
        .count()
}

#[derive(Debug, Default)]
pub struct HeuristicPolicy;

impl HeuristicPolicy {
    pub fn new() -> Self {
        HeuristicPolicy
    }

    fn choose(&self, view: &PlayerView) -> Action {
        match view.pending_kind {
            Some(PendingKind::Mulligan) => self.mulligan(view),
            Some(PendingKind::BottomCards { count }) => Action::BottomCards {
                cards: worst_cards(view, count),
            },
            Some(PendingKind::Discard { count }) => Action::Discard {
                cards: worst_cards(view, count),
            },
            Some(PendingKind::OrderCard { index, .. }) => Action::OrderCard {
                to_top: self.keep_viewed(view, index),
            },
            Some(PendingKind::DeclareAttackers) => {
                let (attackers, defender) = self.attackers(view);
                // A required attack removes the engine's no-attack offer. In
                // that case use an actual offered declaration: the ordinary
                // tactical subset can omit the required creature or choose a
                // different opponent. Keep the old policy unchanged when no
                // requirement applies; this is a legality adapter, not ranking.
                let may_decline = view.legal.iter().any(|legal| {
                    matches!(&legal.action, Action::DeclareAttackers { attackers, .. } if attackers.is_empty())
                });
                if !may_decline {
                    let offered = view.legal.iter().find(|legal| {
                        matches!(&legal.action, Action::DeclareAttackers { defender: offered, .. } if *offered == defender)
                    }).or_else(|| view.legal.iter().find(|legal| {
                        matches!(legal.action, Action::DeclareAttackers { .. })
                    }));
                    if let Some(offered) = offered {
                        return offered.action.clone();
                    }
                }
                Action::DeclareAttackers {
                    attackers,
                    defender,
                }
            }
            Some(PendingKind::DeclareBlockers) => Action::DeclareBlockers {
                blocks: self.blocks(view),
            },
            // CR 510.1 + 509.1h: Ruxa's permission is always taken by this
            // heuristic. Assigning as though unblocked sends all of the
            // creature's combat damage to the defending player and none to the
            // blockers, which is the aggressive reading of a "may"; it is a
            // policy choice for an evidence harness, not a claim of strength.
            Some(PendingKind::CombatDamageAssignment { .. }) => {
                Action::AnswerCombatDamageAssignment {
                    as_though_unblocked: true,
                }
            }
            Some(
                PendingKind::ApnapSacrifice
                | PendingKind::CommanderReturn
                | PendingKind::CommanderReplacement
                | PendingKind::CommanderDraw
                // Triggered-ability choices: take the engine's first offered
                // option (declared targets, and "use the trigger" for a "may"
                // ability).
                | PendingKind::TriggerTarget
                | PendingKind::TriggerChoice,
            ) => view
                .legal
                .first()
                .map(|legal| legal.action.clone())
                .unwrap_or(Action::Pass),
            // CR 701.9a: revealing creature cards is free — the cards stay in
            // hand — so the offered "reveal every creature card" (the last, and
            // largest, offer) strictly dominates "reveal nothing".
            Some(PendingKind::ApnapReveal) => view
                .legal
                .last()
                .map(|legal| legal.action.clone())
                .unwrap_or(Action::Pass),
            None => self.priority_action(view),
        }
    }

    fn mulligan(&self, view: &PlayerView) -> Action {
        let lands = view.own_lands_in_hand;
        let keep = (2..=5).contains(&lands) || view.own_mulligans >= 2;
        Action::Mulligan { keep }
    }

    fn keep_viewed(&self, view: &PlayerView, index: usize) -> bool {
        let Some(card) = view.pending_viewed.get(index) else {
            return true;
        };
        if card.card.def().is_land() {
            // Keep developing lands, but not an eleventh one.
            lands_in_play(view) < 5
        } else {
            true
        }
    }

    fn attackers(&self, view: &PlayerView) -> (Vec<ObjectId>, Option<PlayerId>) {
        let mine: Vec<&CardView> = view
            .own_battlefield
            .iter()
            .filter(|c| {
                is_creature(c) && !c.tapped && !c.summoning_sick && !c.card.def().has_defender()
            })
            .collect();
        if mine.is_empty() {
            return (Vec::new(), None);
        }
        // Choose the least-defended live opponent, then the lowest life; the
        // engine validates the choice and the model sees it as a legal option.
        let target = view
            .opponents
            .iter()
            .filter(|o| !o.eliminated)
            .min_by_key(|o| {
                (
                    o.battlefield
                        .iter()
                        .filter(|c| is_creature(c) && !c.tapped)
                        .count(),
                    o.life,
                    o.seat,
                )
            });
        let Some(target) = target else {
            return (Vec::new(), None);
        };
        let defenders: Vec<&CardView> = target
            .battlefield
            .iter()
            .filter(|c| is_creature(c) && !c.tapped)
            .collect();
        let total_power: i32 = mine.iter().map(|c| c.power.unwrap_or(0)).sum();
        if defenders.is_empty() || total_power >= target.life {
            return (mine.iter().map(|c| c.id).collect(), Some(target.seat));
        }
        let best_def_toughness = defenders
            .iter()
            .map(|c| c.toughness.unwrap_or(0))
            .max()
            .unwrap_or(0);
        let best_def_power = defenders
            .iter()
            .map(|c| c.power.unwrap_or(0))
            .max()
            .unwrap_or(0);
        let chosen: Vec<ObjectId> = mine
            .iter()
            .filter(|c| {
                let power = c.power.unwrap_or(0);
                // Attack if the creature survives and kills (or trades up).
                power >= best_def_toughness || power > best_def_power
            })
            .map(|c| c.id)
            .collect();
        // An empty declaration must carry `defender: None`; policies never pick
        // a defender for zero attackers.
        if chosen.is_empty() {
            (Vec::new(), None)
        } else {
            (chosen, Some(target.seat))
        }
    }

    fn blocks(&self, view: &PlayerView) -> Vec<BlockDecl> {
        let mut blockers: Vec<&CardView> = view
            .own_battlefield
            .iter()
            .filter(|c| is_creature(c) && !c.tapped)
            .collect();
        blockers.sort_by_key(|c| (std::cmp::Reverse(c.power.unwrap_or(0)), c.id.0));
        let enemy_cards: Vec<&CardView> = view
            .opponents
            .iter()
            .flat_map(|o| o.battlefield.iter())
            .collect();
        let mut out = Vec::new();
        let mut used_attackers: Vec<ObjectId> = Vec::new();
        for blocker in blockers {
            let blocker_power = blocker.power.unwrap_or(0);
            let blocker_toughness = blocker.toughness.unwrap_or(0);
            let mut best: Option<(ObjectId, i32)> = None;
            for attacker_id in &view.attackers {
                let Some(attacker) = enemy_cards.iter().find(|c| c.id == *attacker_id) else {
                    continue;
                };
                let survives = blocker_toughness > attacker.power.unwrap_or(0);
                let kills = blocker_power >= attacker.toughness.unwrap_or(0);
                if (survives || kills) && !used_attackers.contains(attacker_id) {
                    let power = attacker.power.unwrap_or(0);
                    if best.is_none_or(|(_, best_power)| power > best_power) {
                        best = Some((*attacker_id, power));
                    }
                }
            }
            if let Some((attacker, _)) = best {
                used_attackers.push(attacker);
                out.push(BlockDecl {
                    blocker: blocker.id,
                    attacker,
                });
            }
        }
        out
    }

    fn priority_action(&self, view: &PlayerView) -> Action {
        // 0. Use a non-sacrifice activated ability when the engine offers one,
        // aimed at the weakest live opponent. Rules legality stays with the
        // engine; this only chooses among already-legal options.
        if let Some(action) = self.activation(view) {
            return action;
        }
        // 1. Counter an opponent's spell if we hold a counterspell (CR 701.6a).
        if let Some(spell) = top_opponent_spell(view)
            && self.threat(spell) >= 2
            && let Some(counter) = find_in_hand(view, CardId::Counterspell)
            && affordable(view, &cost_of(CardId::Counterspell))
        {
            return Action::CastSpell {
                card: counter,
                targets: vec![Target::Spell(spell.object)],
            };
        }

        let my_turn = view.active == view.viewer;
        let main_phase = matches!(view.step, Step::PrecombatMain | Step::PostcombatMain);
        if my_turn && main_phase && view.stack.is_empty() {
            if let Some(action) = self.main_phase_action(view) {
                return action;
            }
        } else if !my_turn {
            // Off-turn: remove a creature that is about to kill us.
            if let Some(action) = self.defensive_action(view) {
                return action;
            }
        }
        Action::Pass
    }

    fn activation(&self, view: &PlayerView) -> Option<Action> {
        let weakest = view
            .opponents
            .iter()
            .filter(|opponent| !opponent.eliminated)
            .min_by_key(|opponent| (opponent.life, opponent.seat));
        view.legal.iter().find_map(|legal| {
            let Action::ActivateAbility { targets, .. } = &legal.action else {
                return None;
            };
            // Sacrificing our own permanent for value is not part of this
            // baseline policy.
            if legal.label.contains("Sacrifice") {
                return None;
            }
            if targets.is_empty() {
                return Some(legal.action.clone());
            }
            let opponent = weakest?;
            (targets.as_slice() == [Target::Player(opponent.seat)]).then(|| legal.action.clone())
        })
    }

    fn main_phase_action(&self, view: &PlayerView) -> Option<Action> {
        // Land drop first (CR 701.18a), but only when the engine actually
        // offers it: one land per turn (CR 305.2) and no other timing blocker.
        if let Some(land) = view.own_hand.iter().find(|c| c.card.def().is_land()) {
            let action = Action::PlayLand { card: land.id };
            if view.legal.iter().any(|legal| legal.action == action) {
                return Some(action);
            }
        }

        // Command-zone practice: the engine's legal CastSpell action already
        // includes commander tax and timing. Use it as the authority instead of
        // re-deriving a base cost that would ignore tax.
        if let Some(action) = view.legal.iter().find_map(|legal| match &legal.action {
            Action::CastSpell { card, .. }
                if view
                    .own_command_zone
                    .iter()
                    .any(|view_card| view_card.id == *card) =>
            {
                Some(legal.action.clone())
            }
            _ => None,
        }) {
            return Some(action);
        }

        // Cast Crack the Earth when the engine offers it and an opponent has a
        // permanent; the policy only picks an existing legal option.
        if view
            .opponents
            .iter()
            .any(|opponent| !opponent.eliminated && !opponent.battlefield.is_empty())
            && let Some(legal) = view.legal.iter().find(|legal| {
                matches!(
                    &legal.action,
                    Action::CastSpell { card, .. }
                        if view
                            .own_hand
                            .iter()
                            .any(|view_card| view_card.id == *card
                                && view_card.card == CardId::CrackTheEarth)
                )
            })
        {
            return Some(legal.action.clone());
        }

        // Pump for lethal when some opponent's board is open.
        let power: i32 = view
            .own_battlefield
            .iter()
            .filter(|c| is_creature(c) && !c.tapped && !c.summoning_sick)
            .map(|c| c.power.unwrap_or(0))
            .sum();
        let open_target = view
            .opponents
            .iter()
            .filter(|o| !o.eliminated)
            .filter(|o| !o.battlefield.iter().any(|c| is_creature(c) && !c.tapped))
            .filter(|o| power < o.life && power + 3 >= o.life)
            .min_by_key(|o| (o.life, o.seat));
        if let Some(_target) = open_target
            && let (Some(growth), Some(biggest)) = (
                find_in_hand(view, CardId::GiantGrowth),
                view.own_battlefield
                    .iter()
                    .filter(|c| is_creature(c) && !c.summoning_sick)
                    .max_by_key(|c| c.power.unwrap_or(0)),
            )
            && affordable(view, &cost_of(CardId::GiantGrowth))
        {
            return Some(Action::CastSpell {
                card: growth,
                targets: vec![Target::Permanent(biggest.id)],
            });
        }

        // Biggest affordable creature.
        let mut creatures: Vec<&CardView> =
            view.own_hand.iter().filter(|c| is_creature(c)).collect();
        creatures.sort_by_key(|c| {
            (
                std::cmp::Reverse(c.power.unwrap_or(0)),
                card_mana_value(c.card),
                c.id.0,
            )
        });
        for creature in creatures {
            if affordable(view, &cost_of(creature.card)) {
                return Some(Action::CastSpell {
                    card: creature.id,
                    targets: Vec::new(),
                });
            }
        }

        // Removal that kills the biggest creature any opponent has.
        let enemy_cards: Vec<&CardView> = view
            .opponents
            .iter()
            .flat_map(|o| o.battlefield.iter())
            .collect();
        if let Some(victim) = enemy_cards
            .iter()
            .copied()
            .filter(|c| is_creature(c))
            .max_by_key(|c| c.power.unwrap_or(0))
        {
            for spell in [CardId::LightningStrike, CardId::Shock] {
                if let Some(card) = find_in_hand(view, spell)
                    && affordable(view, &cost_of(spell))
                    && damage_of(spell) >= victim.toughness.unwrap_or(0).max(0) as u32
                {
                    return Some(Action::CastSpell {
                        card,
                        targets: vec![Target::Permanent(victim.id)],
                    });
                }
            }
        }

        // Card advantage, then cheap filtering in the postcombat main.
        if view.own_hand.len() <= 4
            && let Some(card) = find_in_hand(view, CardId::Divination)
            && affordable(view, &cost_of(CardId::Divination))
        {
            return Some(Action::CastSpell {
                card,
                targets: Vec::new(),
            });
        }
        if view.step == Step::PostcombatMain && view.own_hand.len() <= 6 {
            for spell in [CardId::Consider, CardId::Opt] {
                if let Some(card) = find_in_hand(view, spell)
                    && affordable(view, &cost_of(spell))
                {
                    return Some(Action::CastSpell {
                        card,
                        targets: Vec::new(),
                    });
                }
            }
        }
        None
    }

    fn defensive_action(&self, view: &PlayerView) -> Option<Action> {
        let threat = view
            .opponents
            .iter()
            .flat_map(|o| o.battlefield.iter())
            .filter(|c| is_creature(c))
            .max_by_key(|c| c.power.unwrap_or(0))?;
        if threat.power.unwrap_or(0) < view.own_life {
            return None;
        }
        for spell in [CardId::LightningStrike, CardId::Shock] {
            if let Some(card) = find_in_hand(view, spell)
                && affordable(view, &cost_of(spell))
                && damage_of(spell) >= threat.toughness.unwrap_or(0).max(0) as u32
            {
                return Some(Action::CastSpell {
                    card,
                    targets: vec![Target::Permanent(threat.id)],
                });
            }
        }
        None
    }

    /// Shallow threat ranking for counterspell targets: creatures first, then
    /// direct damage, then pump, then card draw.
    fn threat(&self, spell: &StackView) -> u8 {
        let def = spell.card.def();
        if def.is_creature() {
            3
        } else if def.spell_spec().is_some_and(|s| {
            s.effects
                .iter()
                .any(|e| matches!(e, aperture_core::Effect::Damage(_)))
        }) {
            2
        } else if def.is_instant() || def.is_sorcery() {
            1
        } else {
            0
        }
    }
}

fn damage_of(card: CardId) -> u32 {
    let def = card.def();
    def.spell_spec()
        .and_then(|spec| {
            spec.effects.iter().find_map(|e| match e {
                aperture_core::Effect::Damage(n) => Some(*n as u32),
                _ => None,
            })
        })
        .unwrap_or(0)
}

fn find_in_hand(view: &PlayerView, card: CardId) -> Option<ObjectId> {
    view.own_hand.iter().find(|c| c.card == card).map(|c| c.id)
}

fn top_opponent_spell(view: &PlayerView) -> Option<&StackView> {
    view.stack
        .iter()
        .rev()
        .find(|item| item.controller != view.viewer)
}

/// Rank a card by how expendable it is: surplus lands first, then the most
/// expensive spell, then highest object id (deterministic).
fn rank(card: &CardView, lands: usize) -> (u8, u8, u8) {
    let is_land = card.card.def().is_land();
    let surplus = u8::from(is_land && lands >= 4);
    let spell = u8::from(!is_land);
    (surplus, spell, card_mana_value(card.card))
}

fn worst_cards(view: &PlayerView, count: u8) -> Vec<ObjectId> {
    let lands = lands_in_play(view);
    let mut hand = view.own_hand.clone();
    hand.sort_by(|a, b| {
        rank(b, lands)
            .cmp(&rank(a, lands))
            .then(b.id.0.cmp(&a.id.0))
    });
    hand.iter().take(count as usize).map(|c| c.id).collect()
}

/// The pool's catalog size, surfaced so callers can label the AI's knowledge.
pub fn pool_size() -> usize {
    catalog::pool_size()
}

/// Helper for tests and for the UI: the mana a color-coded land makes.
pub fn land_color(card: CardId) -> Option<ManaColor> {
    card.def().basic_mana
}

impl Policy for HeuristicPolicy {
    fn name(&self) -> &str {
        "heuristic"
    }

    fn decide(&mut self, view: &PlayerView) -> Decision {
        Decision::heuristic(self.choose(view))
    }
}
