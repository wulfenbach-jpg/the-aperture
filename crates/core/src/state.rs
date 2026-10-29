use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::card::{CardId, Effect, TargetSpec};
use crate::counters::Counters;
use crate::error::EngineError;
use crate::ids::{CommanderId, MAX_PLAYERS, MIN_PLAYERS, ObjectId, PlayerId, Zone};
use crate::layer::ContinuousKind;
use crate::mana::ManaPool;
use crate::rng::Pcg32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Step {
    Untap,
    Upkeep,
    Draw,
    PrecombatMain,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndCombat,
    PostcombatMain,
    EndStep,
    Cleanup,
}

pub const STEP_ORDER: [Step; 12] = [
    Step::Untap,
    Step::Upkeep,
    Step::Draw,
    Step::PrecombatMain,
    Step::BeginCombat,
    Step::DeclareAttackers,
    Step::DeclareBlockers,
    Step::CombatDamage,
    Step::EndCombat,
    Step::PostcombatMain,
    Step::EndStep,
    Step::Cleanup,
];

impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::Untap => "untap step",
            Step::Upkeep => "upkeep step",
            Step::Draw => "draw step",
            Step::PrecombatMain => "precombat main phase",
            Step::BeginCombat => "beginning of combat step",
            Step::DeclareAttackers => "declare attackers step",
            Step::DeclareBlockers => "declare blockers step",
            Step::CombatDamage => "combat damage step",
            Step::EndCombat => "end of combat step",
            Step::PostcombatMain => "postcombat main phase",
            Step::EndStep => "end step",
            Step::Cleanup => "cleanup step",
        }
    }

    pub fn index(self) -> usize {
        STEP_ORDER.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// Next step, plus whether the turn wraps.
    pub fn next(self) -> (Step, bool) {
        let i = self.index();
        if i + 1 >= STEP_ORDER.len() {
            (Step::Untap, true)
        } else {
            (STEP_ORDER[i + 1], false)
        }
    }

    pub fn is_main(self) -> bool {
        matches!(self, Step::PrecombatMain | Step::PostcombatMain)
    }

    /// Steps where a player receives priority (CR 502.4: no untap-step
    /// priority; CR 514.3: normally no cleanup-step priority).
    pub fn grants_priority(self) -> bool {
        !matches!(self, Step::Untap | Step::Cleanup)
    }

    pub fn is_combat(self) -> bool {
        matches!(
            self,
            Step::BeginCombat
                | Step::DeclareAttackers
                | Step::DeclareBlockers
                | Step::CombatDamage
                | Step::EndCombat
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderMode {
    Scry,
    Surveil,
}

/// One combat-damage-assignment choice: which attacking creature's controller
/// answers, the creature, and the permanent whose printed permission makes the
/// creature eligible. The source is captured per choice so a player with two
/// permission sources, or two players each with one, can never have a choice
/// validated against the wrong instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatDamageChoice {
    pub player: PlayerId,
    pub attacker: ObjectId,
    pub source: ObjectId,
}

/// A decision the engine is waiting on. While a `Pending` exists, no player has
/// priority (CR 608.2d: resolution choices are not priority).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pending {
    Mulligan {
        player: PlayerId,
    },
    BottomCards {
        player: PlayerId,
        count: u8,
    },
    DeclareAttackers {
        player: PlayerId,
    },
    DeclareBlockers {
        player: PlayerId,
    },
    /// Owner is looking at the top of their own library (`viewed`) and decides
    /// one card at a time. Decided cards are collected in `to_top`.
    OrderCard {
        player: PlayerId,
        viewed: Vec<ObjectId>,
        index: usize,
        to_top: Vec<ObjectId>,
        mode: OrderMode,
    },
    /// The Ring's own draw-then-discard instruction. Distinct from the
    /// cleanup-step discard so its answer resumes the resolving ability instead
    /// of ending the turn.
    EffectDiscard {
        player: PlayerId,
    },
    Discard {
        player: PlayerId,
        count: u8,
    },
    /// Crack the Earth's simultaneous sacrifice queue. `order` is the remaining
    /// unchosen live seats in APNAP order; `choices` holds public answers.
    ApnapSacrifice {
        player: PlayerId,
        order: Vec<PlayerId>,
        choices: BTreeMap<PlayerId, Option<ObjectId>>,
        /// Serialized last-known-source provenance after 800.4a removed/exiled
        /// the represented source while survivors still owe APNAP choices.
        source_left: Option<ApnapSourceLeft>,
    },
    /// CR 903.9a: the owner decides whether a commander that entered a
    /// graveyard/exile since the last SBA check returns to the command zone.
    CommanderReturn {
        /// Complete canonical APNAP queue. Answered entries remain public until
        /// the final owner answers, then the batch is applied together.
        order: Vec<CommanderReturnEntry>,
        /// Number of leading entries already answered.
        cursor: usize,
        /// `true` = accept command-zone return, aligned with `order[..cursor]`.
        answers: Vec<bool>,
    },
    /// CR 903.9b: the owner decides whether a commander that would move to hand
    /// or library is put into the command zone instead. The resolving spell is
    /// suspended on this exact source/cursor.
    CommanderReplacement {
        player: PlayerId,
        commander: CommanderId,
        object: ObjectId,
        from_zone: Zone,
        destination: Zone,
        source: ObjectId,
        cursor: u8,
    },
    /// CR 903.9b on a library->hand draw. `remaining` is the number of further
    /// cards that still must be drawn by the effect; `resume` distinguishes the
    /// draw step from a resolving Draw(n) effect so continuation is exact.
    CommanderDraw {
        player: PlayerId,
        commander: CommanderId,
        object: ObjectId,
        remaining: u8,
        resume: DrawResume,
    },
    /// CR 603.3d: a triggered ability that needs targets can be put on the
    /// stack only after its controller chooses them. `queued` holds the
    /// remaining triggers, still in APNAP order, that are put on the stack
    /// after this one.
    TriggerTarget {
        player: PlayerId,
        trigger: TriggerRef,
        legal: Vec<crate::action::Target>,
        queued: Vec<TriggerRef>,
        /// First chosen target for a two-target fight mode.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        first: Option<crate::action::Target>,
        /// CR 601.2c: the announced X of a set-targeting trigger ("destroy any
        /// number of target artifacts and/or enchantments with total mana value
        /// X or less", Rampaging Yao Guai). `Some` turns the choice into a set:
        /// targets are added one at a time while the running total stays within
        /// the budget and are committed by `Action::FinishTriggerTargets`. The
        /// field is skipped when absent, so every pre-existing pending choice
        /// serializes exactly as before.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        aggregate: Option<u8>,
        /// Targets already added to the set, in the order they were chosen.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        chosen: Vec<crate::action::Target>,
    },
    /// Ayula's printed "choose one" is chosen before targets (CR 603.3c, 700.2b).
    /// Reuses the existing answer action: true selects counters, false selects
    /// fight. Only legal modes are offered.
    TriggerMode {
        player: PlayerId,
        trigger: TriggerRef,
        queued: Vec<TriggerRef>,
    },
    /// CR 701.54a: choose a creature you control as Ring-bearer; None is legal
    /// only when no creature exists.
    RingTemptation {
        player: PlayerId,
        eligible: Vec<ObjectId>,
    },
    /// CR 704.5j: choose one legendary permanent to keep.
    LegendaryChoice {
        player: PlayerId,
        options: Vec<ObjectId>,
    },
    /// CR 603.5: a "you may" trigger is put on the stack regardless of the
    /// controller's intention; the choice is made when it resolves.
    TriggerChoice {
        player: PlayerId,
        trigger: TriggerRef,
    },
    /// Private look at the anchored top card. The card stays in the library
    /// until answered; no temporary removed-object holding is created.
    TopCardChoice {
        player: PlayerId,
        card: ObjectId,
        can_take: bool,
    },
    /// Incremental private typed land search. Chosen cards remain anchored in
    /// the library until Done, so no temporary hidden holding is needed.
    SearchLibrary {
        player: PlayerId,
        remaining: u8,
        chosen: Vec<ObjectId>,
        eligible: Vec<ObjectId>,
        #[serde(default, skip_serializing_if = "is_default_search_mode")]
        mode: LibrarySearchMode,
        /// CR 701.20a: the cards a "look at the top N" instruction actually
        /// examined, top first. Empty for a search (a search looks at the whole
        /// library); non-empty only for the look modes, where "put the rest on
        /// the bottom in a random order" (CR 701.20e) means exactly these cards
        /// and not the rest of the library.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        looked: Vec<ObjectId>,
    },
    /// Kamahl's Summons: every live seat, in APNAP order from the active player
    /// (CR 101.4, 608.2f), may reveal any number of creature cards from its own
    /// hand (CR 701.9a). `revealed` collects each public answer; the Bear tokens
    /// are created only after the last seat answers, so no seat's count depends
    /// on the order the table answered in.
    ApnapReveal {
        player: PlayerId,
        order: Vec<PlayerId>,
        revealed: BTreeMap<PlayerId, Vec<ObjectId>>,
    },
    /// CR 510.1 + 509.1h: "For each creature you control with no abilities, you
    /// may have that creature assign its combat damage as though it weren't
    /// blocked" (Ruxa, Patient Professor). Every eligible attacking creature is
    /// asked in APNAP order from the active player; only after the last answer is
    /// combat damage dealt, in one simultaneous batch (CR 510.2). The choice
    /// exists only for creatures the permission actually changes — a blocked
    /// attacker — because an unblocked attacker already assigns as though it
    /// weren't blocked.
    CombatDamageAssignment {
        /// Eligible attackers in active-anchored APNAP order, fixed when the
        /// combat damage step began.
        order: Vec<CombatDamageChoice>,
        /// Number of leading entries already answered.
        cursor: usize,
        /// `true` = assign as though unblocked, aligned with `order[..cursor]`.
        answers: Vec<bool>,
    },
    /// CR 702.75a (hideaway): the controller looks at up to N cards (the
    /// `looked` cards, top first, still anchored in their library) and exiles one
    /// of them face down, linked to `source`. The rest go to the bottom in a
    /// random order. At least one card is present whenever this pending exists;
    /// an empty library resolves the ability with nothing to look at.
    HideawayChoice {
        player: PlayerId,
        /// The permanent whose linked hideaway ability is resolving. Its
        /// battlefield identity is what the exiled card is linked to (the entry
        /// recorded in `GameState::hideaway`), so a permanent that leaves the
        /// battlefield before the answer makes the choice stale.
        source: ObjectId,
        looked: Vec<ObjectId>,
    },
    /// "If there are cards exiled with it, you may play one of them without
    /// paying its mana cost" (Evercoat Ursine). `options` are the linked exiled
    /// cards that are playable right now: any nonland card (the printed ruling
    /// plays it while the ability resolves, so normal timing does not apply) and
    /// a land only when the controller still has a land play available this turn.
    /// The answer either plays one card or declines; the ability has no other
    /// effect, so declining finishes it.
    PlayExiled {
        player: PlayerId,
        source: ObjectId,
        options: Vec<ObjectId>,
    },
}

/// One triggered ability waiting to be put on the stack (CR 603.2, 603.3).
///
/// The record carries the source's identity and card as *last known
/// information*: a "dies" trigger is queued after its source has already left
/// the battlefield, so resolution can never read the current zone for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerRef {
    /// The object that was the source when the event happened.
    pub source: ObjectId,
    pub card: CardId,
    /// The controller of the source at the moment the event happened.
    pub controller: PlayerId,
    /// Index into [`crate::ability::abilities`] for `card`.
    pub ability: u8,
    /// Monotonic arrival serial; keeps same-controller ordering deterministic.
    pub seq: u64,
    /// CR 202.3 + 603.2: the mana value of the spell that caused this trigger,
    /// captured when the trigger was queued. Only a "you cast a creature spell"
    /// trigger uses it; every other trigger carries 0 and — because zero is
    /// skipped — serializes and hashes exactly as it did before slice 22.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub spell_mana_value: u8,
    /// Entered object captured by a non-target "it" trigger. Never a target and
    /// never retargeted; dangling after a battlefield departure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected: Option<ObjectId>,
    /// Non-targeted affected player captured by a land-entry trigger (Zo-Zu).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected_player: Option<PlayerId>,
    /// The creature a "whenever another creature dies" trigger watched die,
    /// captured when the death was dispatched so the resolution can exile that
    /// exact card from its owner's graveyard (CR 603.6c). Absent for every other
    /// trigger, so the field is skipped and pre-slice-46 digests are unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dead_object: Option<ObjectId>,
    /// Effective source P/T captured when its battlefield incarnation leaves,
    /// used by a queued/stacked source-left P/T-setting ability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_pt_lki: Option<(i32, i32)>,
    /// Count captured by The Earth King's attack trigger.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub search_amount: u8,
    /// CR 107.3a + 202.3: the value announced for an `{X}` cost. Captured when
    /// the trigger is queued (the entering spell's X) and read by the printed
    /// instruction that names X. Zero is skipped, so every trigger queued by a
    /// card without X serializes and hashes exactly as it did before.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub x_value: u8,
}

/// Supported announced-X bound, shared by cast and serialized-state validation.
/// This is an implementation limit, not a Magic rules limit.
pub(crate) const MAX_X_VALUE: u8 = 20;

fn validate_saved_x(value: u8, permits_x: bool) -> Result<(), EngineError> {
    if value > MAX_X_VALUE || (value != 0 && !permits_x) {
        return Err(EngineError::InvalidState {
            reason: format!(
                "saved X {value} exceeds the supported maximum {MAX_X_VALUE} or is not bound to an X instruction"
            ),
        });
    }
    Ok(())
}

impl TriggerRef {
    pub(crate) fn validate_x(&self) -> Result<(), EngineError> {
        let permits_x = crate::ability::abilities(self.card)
            .get(self.ability as usize)
            .is_some_and(|ability| ability.target_spec() == TargetSpec::ArtifactOrEnchantment)
            && self.card.def().has_x_cost();
        validate_saved_x(self.x_value, permits_x)
    }
}

/// What one stack item is: a spell card, or an ability whose source never
/// leaves the battlefield merely because the ability is on the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackEntry {
    Spell,
    Activated { ability: u8 },
    Triggered { ability: u8 },
}

/// Strong serialized provenance for an APNAP source that left the game while
/// its each-player instruction was suspended (CR 800.4a + 608.2h).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApnapSourceLeft {
    pub source: ObjectId,
    pub card: CardId,
    /// Instruction cursor recorded when the queue was created.
    pub cursor: u8,
    pub prior_controller: PlayerId,
    pub owner: PlayerId,
    pub departed: PlayerId,
    pub zone: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawResume {
    Step,
    Effect {
        source: ObjectId,
        cursor: u8,
    },
    /// Serialized progress through "each player draws N": the next seat to
    /// draw, cards it still owes, completed live seats, and the live-seat count
    /// fixed when the instruction began. Old variants keep their encoding.
    EachPlayer {
        source: ObjectId,
        cursor: u8,
        next: PlayerId,
        cards_left: u8,
        per_seat: u8,
        done: u8,
        live_total: u8,
    },
}

/// One exact CR 903.9a owner choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderReturnEntry {
    pub player: PlayerId,
    pub commander: CommanderId,
    pub object: ObjectId,
    pub zone: Zone,
}

/// One declaration round of the pre-game mulligan procedure (CR 103.5).
/// Declarations are collected in starting-player turn order; only after the
/// whole round is collected are the mulligans executed as one batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MulliganRound {
    pub round: u8,
    /// Exact eligible seats for this round (previous mulliganers after round 1).
    pub eligible: Vec<PlayerId>,
    /// Seats that mulliganed in the previous round; empty in round 1.
    pub previous_mulliganers: Vec<PlayerId>,
    /// Seats locked in by a keep declaration across all rounds.
    pub kept: Vec<PlayerId>,
    /// Eligible seats in starting-player APNAP order (equal to `eligible`).
    pub order: Vec<PlayerId>,
    /// Index of the next seat that must declare.
    pub cursor: usize,
    /// `true` = keep, `false` = mulligan, for seats `order[..cursor]`.
    pub declarations: BTreeMap<PlayerId, bool>,
    /// Mulliganers whose post-redraw London bottom choice is still pending.
    pub bottom_queue: Vec<PlayerId>,
    /// Mulliganers whose bottom choice was already collected this batch.
    pub bottomed: Vec<PlayerId>,
}

impl Pending {
    pub fn player(&self) -> PlayerId {
        match self {
            Pending::Mulligan { player }
            | Pending::BottomCards { player, .. }
            | Pending::DeclareAttackers { player }
            | Pending::DeclareBlockers { player }
            | Pending::OrderCard { player, .. }
            | Pending::Discard { player, .. }
            | Pending::ApnapSacrifice { player, .. } => *player,
            Pending::ApnapReveal { player, .. } => *player,
            Pending::CombatDamageAssignment { order, cursor, .. } => order[*cursor].player,
            Pending::CommanderReturn { order, cursor, .. } => order[*cursor].player,
            Pending::CommanderReplacement { player, .. } => *player,
            Pending::CommanderDraw { player, .. } => *player,
            Pending::TriggerTarget { player, .. }
            | Pending::TriggerChoice { player, .. }
            | Pending::TriggerMode { player, .. }
            | Pending::RingTemptation { player, .. }
            | Pending::LegendaryChoice { player, .. }
            | Pending::EffectDiscard { player }
            | Pending::TopCardChoice { player, .. }
            | Pending::HideawayChoice { player, .. }
            | Pending::PlayExiled { player, .. }
            | Pending::SearchLibrary { player, .. } => *player,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Pending::Mulligan { player } => {
                format!("P{player}: keep or mulligan the opening hand (CR 103.5)")
            }
            Pending::BottomCards { player, count } => format!(
                "P{player}: put {count} card(s) on the bottom of the library (London mulligan, CR 103.5)"
            ),
            Pending::DeclareAttackers { player } => {
                format!("P{player}: declare attackers (CR 508.1a)")
            }
            Pending::DeclareBlockers { player } => {
                format!("P{player}: declare blockers (CR 509.1a)")
            }
            Pending::OrderCard {
                player,
                index,
                viewed,
                mode,
                ..
            } => {
                let verb = match mode {
                    OrderMode::Scry => "on top or on the bottom",
                    OrderMode::Surveil => "on top or into the graveyard",
                };
                format!(
                    "P{player}: {} card {} of {} — {verb} (CR {})",
                    match mode {
                        OrderMode::Scry => "scry",
                        OrderMode::Surveil => "surveil",
                    },
                    index + 1,
                    viewed.len(),
                    match mode {
                        OrderMode::Scry => "701.22a",
                        OrderMode::Surveil => "701.25a",
                    }
                )
            }
            Pending::EffectDiscard { player } => {
                format!("P{player}: discard a card (The Ring's attack trigger)")
            }
            Pending::Discard { player, count } => {
                format!("P{player}: discard {count} card(s) down to maximum hand size (CR 514.1)")
            }
            Pending::ApnapSacrifice { player, .. } => {
                format!("P{player}: choose a permanent to sacrifice (CR 101.4, 701.21a)")
            }
            Pending::ApnapReveal { player, .. } => format!(
                "P{player}: reveal any number of creature cards from hand (CR 101.4, 701.9a)"
            ),
            Pending::CombatDamageAssignment {
                order,
                cursor,
                answers,
            } => {
                let current = &order[*cursor];
                format!(
                    "P{}: may {} assign its combat damage as though it weren't blocked? (CR 510.1; {}/{} answered, {} chose to)",
                    current.player,
                    current.attacker,
                    cursor,
                    order.len(),
                    answers.iter().filter(|chosen| **chosen).count()
                )
            }
            Pending::CommanderReturn { order, cursor, .. } => {
                let current = &order[*cursor];
                format!(
                    "P{}: commander {} is in {} and may return to the command zone (CR 903.9a; {}/{} answered)",
                    current.player,
                    current.object,
                    current.zone.name(),
                    cursor,
                    order.len()
                )
            }
            Pending::CommanderDraw {
                player,
                object,
                remaining,
                ..
            } => format!(
                "P{player}: commander {object} would be drawn; put it into the command zone instead? (CR 903.9b, {remaining} more draw(s) pending)"
            ),
            Pending::CommanderReplacement {
                player,
                object,
                from_zone,
                destination,
                ..
            } => format!(
                "P{player}: commander {object} in the {} may be put into the command zone instead of the {} (CR 903.9b)",
                from_zone.name(),
                destination.name()
            ),
            Pending::TriggerTarget {
                player,
                trigger,
                legal,
                aggregate,
                chosen,
                ..
            } => match aggregate {
                Some(budget) => format!(
                    "P{player}: choose any number of target artifacts and/or enchantments with \
                     total mana value {budget} or less for {}'s triggered ability \
                     ({} chosen, {} legal target(s), CR 601.2c)",
                    trigger.card.name(),
                    chosen.len(),
                    legal.len()
                ),
                None => format!(
                    "P{player}: choose a target for {}'s triggered ability ({} legal target(s), CR 603.3d)",
                    trigger.card.name(),
                    legal.len()
                ),
            },
            Pending::TriggerChoice { player, trigger } => format!(
                "P{player}: choose whether to use {}'s optional triggered ability (CR 603.5)",
                trigger.card.name()
            ),
            Pending::TriggerMode { player, .. } => {
                format!("P{player}: choose Ayula's counter or fight mode (CR 603.3c, 700.2b)")
            }
            Pending::RingTemptation { player, .. } => {
                format!(
                    "P{player}: choose a creature you control to become your Ring-bearer (CR 701.54a)"
                )
            }
            Pending::LegendaryChoice { player, options } => {
                format!(
                    "P{player}: keep one of {} legendary permanents (CR 704.5j)",
                    options.len()
                )
            }
            Pending::SearchLibrary {
                player,
                remaining,
                chosen,
                mode,
                ..
            } => match mode {
                LibrarySearchMode::BasicLandToBattlefield => format!(
                    "P{player}: choose up to {remaining} basic land(s) to put onto the battlefield tapped ({} chosen)",
                    chosen.len()
                ),
                LibrarySearchMode::SnowLandToHand => format!(
                    "P{player}: choose up to {remaining} snow land card(s) to reveal and put into hand ({} chosen)",
                    chosen.len()
                ),
                LibrarySearchMode::AnyLandToBattlefield => format!(
                    "P{player}: choose up to {remaining} land card(s) to put onto the battlefield tapped ({} chosen)",
                    chosen.len()
                ),
                LibrarySearchMode::CreatureFromTopToBattlefieldAttacking => format!(
                    "P{player}: look at the top eight cards — you may put a creature card from among them onto the battlefield tapped and attacking ({} put)",
                    chosen.len()
                ),
                LibrarySearchMode::GoblinCardToHand => format!(
                    "P{player}: choose up to {remaining} Goblin card(s) to reveal and put into hand ({} chosen)",
                    chosen.len()
                ),
                LibrarySearchMode::GoblinPermanentToBattlefield => format!(
                    "P{player}: choose up to {remaining} Goblin permanent card(s) to put onto the battlefield ({} chosen)",
                    chosen.len()
                ),
            },
            Pending::TopCardChoice {
                player, can_take, ..
            } => {
                if *can_take {
                    format!(
                        "P{player}: reveal the looked-at card and put it into hand, or put it on the bottom"
                    )
                } else {
                    format!("P{player}: put the looked-at card on the bottom")
                }
            }
            Pending::HideawayChoice { player, looked, .. } => format!(
                "P{player}: hideaway — exile one of the {} looked-at card(s) face down, then put the rest on the bottom (CR 702.75a)",
                looked.len()
            ),
            Pending::PlayExiled {
                player, options, ..
            } => format!(
                "P{player}: you may play one of the {} card(s) exiled with this creature without paying its mana cost",
                options.len()
            ),
        }
    }
}

/// Explicit named game profile. There is no implicit switch: every profile
/// records the starting life it uses. The two existing 20-life profiles keep
/// their accepted names and behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Profile {
    /// Ordinary practice: 2–4 seats, 20 life, no command zone.
    Practice20,
    /// Named command-zone practice: one supported commander per seat, 20 life.
    CommandZone20,
    /// Named BOUNDED Commander-combat practice: one supported commander per
    /// seat starting at 40 life. Not the Commander format: no 100-card
    /// construction, partners, legend rule or native Commander table.
    CommanderCombat40,
}

impl Profile {
    pub const fn starting_life(self) -> i32 {
        match self {
            Profile::Practice20 | Profile::CommandZone20 => 20,
            Profile::CommanderCombat40 => 40,
        }
    }

    pub const fn has_command_zone(self) -> bool {
        matches!(self, Profile::CommandZone20 | Profile::CommanderCombat40)
    }

    pub const fn is_commander_combat(self) -> bool {
        matches!(self, Profile::CommanderCombat40)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Profile::Practice20 => "practice-20",
            Profile::CommandZone20 => "command-zone-practice-20",
            Profile::CommanderCombat40 => "commander-combat-practice-40",
        }
    }
}

/// One cumulative CR 903.10 commander-damage row, keyed by the stable
/// commander identity and the recipient seat. Never keyed by the transient
/// `ObjectId`, the card id or a controller: the designation is an attribute of
/// the card and survives every zone change (CR 903.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderDamageEntry {
    pub commander: CommanderId,
    pub recipient: PlayerId,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Object {
    pub id: ObjectId,
    pub card: CardId,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub zone: Zone,
    /// Stable commander identity when this object represents a commander card.
    pub commander: Option<CommanderId>,
    pub tapped: bool,
    /// Damage marked this turn (CR 120.3); cleared in cleanup (CR 514.2).
    pub damage: u32,
    /// Summoning sickness (CR 302.6).
    pub summoning_sick: bool,
    /// Layer 7b/7c delta from `recompute_layers` (CR 613.4b/c).
    ///
    /// This single delta holds every layer 7b and 7c result for the object
    /// relative to its *printed* power/toughness: a layer 7b base setter stores
    /// `base - printed` (Exuberant Wolfbear's until-end-of-turn setter and
    /// Circle of the Moon Druid's printed during-your-turn 4/2), printed layer
    /// 7c statics and until-end-of-turn modifiers store their additive amounts.
    /// `Object::power`/`toughness` then add the layer 7d counters (CR 613.4d,
    /// `Counters::power_delta`) on top, which is why 7d needs no delta here.
    /// Setting layers are recomputed in layer order, and 7c addition is
    /// commutative, so the single field is lossless.
    pub pump_power: i32,
    pub pump_toughness: i32,
    /// Legacy sandbox storage used by the native UI's editing tools only. The
    /// rules layer never reads it: typed counters live in `Object::counters`.
    pub plus_one_counters: u32,
    /// Typed counters on this permanent (CR 122). Only battlefield permanents
    /// may carry counters; leaving the battlefield clears them (CR 122.2) and
    /// the projections expose them as public information.
    ///
    /// `#[serde(default)]` keeps every pre-#16 v10 save loadable (counters were
    /// simply absent then), and `skip_serializing_if` keeps the serialized form
    /// of a counter-free state byte-identical to that format, so the canonical
    /// digest of an unchanged game does not move.
    #[serde(default, skip_serializing_if = "Counters::is_empty")]
    pub counters: Counters,
    /// CR 702.94a: the permanent this one is paired with (soulbond).
    ///
    /// Pairing is a state of the permanent, not a continuous effect: it is
    /// created when the printed "you may pair" trigger resolves, and it ends
    /// when the controller stops controlling both creatures. It is cleared at
    /// the single battlefield-exit point (`Game::detach_object`) and never
    /// trusted on its own: every reader goes through `Game::paired_with`, which
    /// re-validates the back-reference, both zones and the shared controller.
    /// Both attributes keep every pre-slice save and replay digest stable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paired_with: Option<ObjectId>,
}

impl Object {
    /// The *printed* card type (CR 205.2a). A layer 4 effect can add the
    /// creature card type to a noncreature permanent (Stuffed Bear's
    /// animation), so every rules question reads `Game::is_creature` instead;
    /// this predicate is the printed baseline that query starts from.
    pub fn is_creature(&self) -> bool {
        self.card.def().is_creature()
    }

    pub fn has_flying(&self) -> bool {
        self.card.def().has_flying()
    }

    pub fn has_defender(&self) -> bool {
        self.card.def().has_defender()
    }

    /// CR 702.19a: trample printed on the card. A layer 6 effect can also grant
    /// it (Goreclaw, Surrak and Goreclaw), so combat asks `Game::has_trample`.
    pub fn has_trample(&self) -> bool {
        self.card.def().has_trample()
    }

    /// CR 111.7: a token is not a card and ceases to exist when it leaves the
    /// battlefield.
    pub fn is_token(&self) -> bool {
        self.card.is_token()
    }

    pub fn power(&self) -> i32 {
        self.card.def().power.unwrap_or(0) + self.pump_power + self.counters.power_delta()
    }

    pub fn toughness(&self) -> i32 {
        self.card.def().toughness.unwrap_or(0)
            + self.pump_toughness
            + self.counters.toughness_delta()
    }

    pub fn name(&self) -> &'static str {
        self.card.name()
    }

    /// Keywords this permanent has. None are printed on the 28-card pool.
    pub fn keywords(&self) -> &'static [crate::tokens::Keyword] {
        crate::tokens::keywords(self.card)
    }

    pub fn has_keyword(&self, keyword: crate::tokens::Keyword) -> bool {
        self.keywords().contains(&keyword)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackItem {
    pub object: ObjectId,
    /// Card identity recorded when the item was put on the stack. For a spell
    /// this is the spell's card; for an ability it is the source's card as last
    /// known information, so resolution never depends on the source's current
    /// zone (or on it still existing).
    pub card: CardId,
    pub controller: PlayerId,
    pub targets: Vec<crate::action::Target>,
    /// Modal spell mode indices, resolved and projected in printed order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modes: Vec<u8>,
    /// Non-target object captured by the trigger's "it" wording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected: Option<ObjectId>,
    /// Non-targeted affected player captured by a land-entry trigger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected_player: Option<PlayerId>,
    /// The dead card a "whenever another creature dies" trigger watched, copied
    /// from `TriggerRef::dead_object` when the item was put on the stack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dead_object: Option<ObjectId>,
    /// Index of the next effect to apply during resolution (CR 608.2c).
    pub cursor: u8,
    /// Spell or ability. Ability entries resolve without moving their source
    /// into a graveyard for resolution (CR 603.3, 608.2).
    pub entry: StackEntry,
    /// Set once the controller has answered a CR 603.5 "you may" trigger.
    pub trigger_answered: bool,
    /// CR 202.3: mana value of the spell that caused this triggered ability, when
    /// the printed effect is sized by it (Dancing from Dark to Dawn). Copied from
    /// `TriggerRef::spell_mana_value` when the trigger is put on the stack and 0
    /// for spells and for every other ability, which keeps a zero-value item
    /// byte-identical to the pre-slice-22 serialization.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub spell_mana_value: u8,
    /// Effective source P/T captured before its battlefield departure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_pt_lki: Option<(i32, i32)>,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub search_amount: u8,
    /// CR 107.3a + 202.3: X announced for this spell, or copied into its
    /// printed X-reading triggered instruction (0 for unrelated abilities and
    /// spells without `{X}`). A resolving permanent spell reads it for entry
    /// counters and triggers, preserving it across save/load.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub x_value: u8,
}

/// `skip_serializing_if` helper for the slice-22 mana-value fields: a zero
/// (spell, or any ability that does not read a spell's mana value) is omitted so
/// existing saves, replays and digests are unchanged.
fn is_zero_u8(value: &u8) -> bool {
    *value == 0
}

/// Stable registry entry for one physical commander card. The per-zone object
/// id changes on every zone transition; `CommanderId` never does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commander {
    pub id: CommanderId,
    pub owner: PlayerId,
    pub card: CardId,
    /// Current per-zone object. Re-keyed on every commander zone transition.
    pub object: ObjectId,
    /// Successful paid casts from the command zone (CR 903.8).
    pub casts_from_command_zone: u32,
    /// Armed when the commander entered a graveyard/exile and has not yet been
    /// offered the CR 903.9a choice. Cleared when the choice is answered.
    pub return_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub deck_name: String,
    pub library: Vec<ObjectId>,
    pub hand: Vec<ObjectId>,
    pub graveyard: Vec<ObjectId>,
    pub exile: Vec<ObjectId>,
    pub command_zone: Vec<ObjectId>,
    pub life: i32,
    pub mana: ManaPool,
    pub mulligans: u8,
    /// Set when a draw was attempted with an empty library (CR 104.3c/704.5b).
    pub drew_from_empty: bool,
    pub lost: bool,
}

impl Player {
    pub fn hand_size(&self) -> usize {
        self.hand.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Turn {
    pub number: u32,
    pub active: PlayerId,
    pub step: Step,
    /// `None` means no player has priority right now (mid-resolution, mid
    /// pending choice, or between steps).
    pub priority: Option<PlayerId>,
    /// Seat bitmask of players who have passed consecutively in the current
    /// priority round (CR 117.4). Bit `i` is seat `i`; at most four seats.
    pub passed: u8,
    /// Lands played by the active player this turn (CR 305.2).
    pub lands_played: u8,
    pub attackers: Vec<ObjectId>,
    /// Attackers that became blocked when blockers were declared (CR 509.1h):
    /// they stay blocked even if every blocker later leaves the battlefield.
    pub blocked: Vec<ObjectId>,
    /// blocker -> attacker pairs.
    pub blockers: Vec<(ObjectId, ObjectId)>,
    /// CR 509.1g: a creature remains blocking after all creatures it blocked
    /// leave combat. Pairs alone cannot represent it then. Only those unpaired
    /// live blockers are stored here; normal combats keep the field omitted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unpaired_blockers: Vec<ObjectId>,
    /// CR 506.5: "attacks alone" / "blocks alone" are properties of the
    /// *declaration*, not of the live combat state. These two lists are that
    /// declaration record: written once when the declaration is submitted and
    /// never pruned when a creature leaves combat. A surviving companion did
    /// not "attack alone", but may now BE attacking alone. Witness's past-state
    /// predicate reads the live lists instead. Cleared at end of combat
    /// (CR 511.3) and turn start. Kept serialized for existing slice-47+ saves;
    /// empty fields stay omitted to preserve earlier no-declaration digests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declared_attackers: Vec<ObjectId>,
    /// Distinct creatures declared as blockers this combat (one blocker
    /// blocking two attackers is a single blocking creature), for the same
    /// CR 506.5 reason as `declared_attackers`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declared_blockers: Vec<ObjectId>,
    pub combat_damage_done: bool,
    /// The defending seat chosen in this combat's attacker declaration. In the
    /// bounded M1 model every attacker in one declaration attacks this seat.
    pub defender: Option<PlayerId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Win {
        winner: PlayerId,
        reason: String,
    },
    /// Engine projection of CR 104.4a (all remaining players lose
    /// simultaneously). Not a Comprehensive Rules term. Unexercised by the
    /// current card pool.
    Draw {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RingState {
    pub temptations: u8,
    pub bearer: Option<ObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sacrifice_at_end_of_combat: Vec<ObjectId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LibrarySearchMode {
    #[default]
    BasicLandToBattlefield,
    SnowLandToHand,
    AnyLandToBattlefield,
    /// Owlbear Cub: the pending choice is a "look at the top eight cards"
    /// (CR 701.20a), not a search. `looked` holds the examined cards in order;
    /// answering puts at most one of `eligible` onto the battlefield tapped and
    /// attacking the defending player, and then moves the *looked* cards that
    /// were not chosen to the bottom in a random order (CR 701.20e) without
    /// reordering the rest of the library.
    CreatureFromTopToBattlefieldAttacking,
    /// Goblin Matron: "you may search your library for a Goblin card, reveal
    /// that card, put it into your hand, then shuffle." A real search of the
    /// whole library (CR 701.23), filtered by the Goblin creature subtype read
    /// from the printed type line (CR 205.3m, `CardDef::has_subtype`), so a
    /// Goblin card in any zone-independent sense — creature cards only in this
    /// pool — is eligible and nothing else is.
    GoblinCardToHand,
    /// Moggcatcher: "search your library for a Goblin permanent card, put it
    /// onto the battlefield, then shuffle." A real whole-library search
    /// (CR 701.23) filtered by the Goblin subtype on the printed type line and
    /// by CR 110.4a (a permanent card only). The found card is put onto the
    /// battlefield, not revealed into hand.
    GoblinPermanentToBattlefield,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegenerationShield {
    pub object: ObjectId,
}

/// CR 702.75a: one card exiled face down *with* a specific permanent.
///
/// The link is the whole printed meaning of "cards exiled with it"; it is not a
/// property of the exiled card's zone, and it dies with the permanent's
/// battlefield identity (CR 400.7), which is why it names the exact
/// [`ObjectId`] rather than a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HideawayLink {
    /// The exiled card, in its owner's exile zone and in `face_down_exile`.
    pub card: ObjectId,
    /// The permanent it was exiled with.
    pub source: ObjectId,
    /// The player who controlled the exiling permanent, and therefore the one
    /// who may look at the face-down card (CR 406.3). Control changes of the
    /// permanent are not tracked; see the slice-60 documentation.
    pub controller: PlayerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub seed: u64,
    /// Who plays first. In a two-player game that player skips their first draw
    /// step (CR 103.8a); in 3/4-seat free-for-all they draw (CR 103.8c, 800.7).
    pub first_player: PlayerId,
    pub rng: Pcg32,
    pub objects: BTreeMap<ObjectId, Object>,
    /// Stable per-physical-commander registry; empty for ordinary practice
    /// games and exactly one entry per seat for command-zone practice.
    pub commanders: Vec<Commander>,
    /// Explicit named profile; the only source of starting life and of whether
    /// a command zone exists.
    pub profile: Profile,
    /// Cumulative commander combat damage, canonical order by
    /// `(commander, recipient)`. Empty outside `Profile::CommanderCombat40`.
    pub commander_damage: Vec<CommanderDamageEntry>,
    /// Commander identities whose owner has left the game. Historic damage by
    /// these identities stays in `commander_damage` (CR 903.10a is over the
    /// course of the game); this registry only keeps the ledger source-valid.
    pub retired_commanders: Vec<CommanderId>,
    /// Turn for which a full Fog-style combat-damage shield is active. Cleared
    /// in cleanup (CR 514.2); serialized so save/replay preserve it exactly.
    pub prevent_all_combat_damage_turn: Option<u32>,
    /// Ordered seats. `players.len()` is the only seat-count source; seats are
    /// never renumbered when a player leaves the game.
    pub players: Vec<Player>,
    pub stack: Vec<StackItem>,
    pub turn: Turn,
    pub pending: Option<Pending>,
    /// Pre-game mulligan declaration/execution state (CR 103.5). `None` after
    /// the opening procedure finishes.
    pub mulligan: Option<MulliganRound>,
    /// Object id of the stack item currently resolving (CR 608.2), if any.
    pub resolving: Option<ObjectId>,
    pub next_object: u32,
    /// Triggered abilities that have triggered but are not on the stack yet
    /// (CR 603.2, 603.3). Serialized so save/replay preserve the exact queue.
    pub trigger_queue: Vec<TriggerRef>,
    /// Next arrival serial for `trigger_queue`.
    pub next_trigger_seq: u64,
    /// Cards currently revealed from a hidden zone (CR 701.16a). Public
    /// information for every viewer while the card stays in that hidden zone;
    /// see `crate::view`. Serialized so save/replay keep the same visibility,
    /// skipped when empty so a state with nothing revealed keeps the pre-#16
    /// serialized form (and therefore its digest).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revealed: Vec<ObjectId>,
    /// Any creature died this turn. Only maintained while a morbid/count card is
    /// present, so pre-slice-17 replays keep their digest.
    #[serde(default, skip_serializing_if = "is_false")]
    pub creature_died_this_turn: bool,
    /// Nontoken creatures owned by each seat that moved battlefield→graveyard
    /// this turn, indexed by owner. Empty/skipped preserves old digests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nontoken_creature_deaths_this_turn: Vec<u32>,
    /// Per-seat count of armed one-shot draw replacements (Words of Wilding,
    /// CR 614.1a): "the next time you would draw a card this turn, create a 2/2
    /// green Bear creature token instead". Indexed by the seat that would draw,
    /// grown only when a replacement is armed and cleared in cleanup (CR 514.2),
    /// so every state that never resolved one keeps its pre-slice-35 serialized
    /// form and digest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub draw_replacements: Vec<u32>,
    /// Per-turn "activate only once each turn" usage, keyed by source object.
    /// Bit `n` records that ability index `n` was activated this turn.
    pub activation_uses: BTreeMap<ObjectId, u8>,
    /// Battlefield objects that "don't untap during your next untap step"
    /// (CR 502.3, Spectral Bears). The entry is consumed by that permanent's
    /// controller's next untap step, which leaves it tapped exactly once;
    /// entries for objects that left the battlefield are dropped.
    ///
    /// Skipped when empty so every state that never used the effect keeps the
    /// pre-slice-19 serialized form and therefore its digest.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub skip_next_untap: BTreeSet<ObjectId>,
    /// Continuous effects with their layer, timestamp and duration (CR 613).
    /// Serialized so a save or replay reproduces the exact layer state.
    ///
    /// Empty collections and zero counters are skipped so a state that uses no
    /// effect still serializes byte-for-byte as it did before the effect
    /// pipeline existed. The canonical digest is taken over this JSON, so
    /// skipping the empty form is what keeps pre-existing saves and replays
    /// verifiable instead of failing as digest mismatches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<crate::layer::ContinuousEffect>,
    /// One-shot prevention shields (CR 615), in creation order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shields: Vec<crate::layer::DamageShield>,
    /// CR 701.19 regeneration shields, keyed to the object that created them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regeneration_shields: Vec<RegenerationShield>,
    /// Per-seat The Ring state; empty/skipped when untouched.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rings: Vec<RingState>,
    /// CR 117.3c: the activator receives the next priority grant after an
    /// activated ability completes its post-payment checkpoint. Cleared as soon
    /// as that grant is delivered; omitted when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation_priority: Option<PlayerId>,
    /// Seats that declared at least one attacker this turn ("Raid — if you
    /// attacked this turn"). Empty and skipped until an attack is declared;
    /// cleared only when the turn number advances, not on cleanup.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attacked_this_turn: Vec<PlayerId>,
    /// Per-seat count of Goblin permanents that have entered the battlefield
    /// under that seat's control this turn, indexed by seat ("the number of
    /// Goblins that entered the battlefield under your control this turn" —
    /// Hobgoblin Bandit Lord). Tokens count: a token entering is a permanent
    /// entering (CR 111.1). Empty and skipped until the first Goblin entry of
    /// the turn, so every earlier save and replay digest is byte-identical; the
    /// whole vector is cleared when the turn number advances, exactly like the
    /// attack history above.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub goblins_entered_this_turn: Vec<u32>,
    /// CR 702.75a + 400.7: hideaway's linked exile. One entry per card exiled
    /// face down by a hideaway ability, naming the exact battlefield object it
    /// was exiled *with*. Empty and skipped when nothing was ever exiled face
    /// down, so every pre-slice-60 save and replay digest is byte-identical.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hideaway: Vec<HideawayLink>,
    /// Identity of a card exiled face down (CR 406.3): every viewer except the
    /// player who may look at it sees a face-down card. Kept as a set so the
    /// public view can never leak a hidden identity, and skipped when empty for
    /// the same digest reason as `hideaway`.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub face_down_exile: BTreeSet<ObjectId>,
    /// Next rule-visible effect id.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub next_effect: u32,
    /// Next rule-visible timestamp (CR 613.7 order, never wall-clock).
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub next_timestamp: u64,
    /// Incremented on every accepted state mutation.
    pub revision: u64,
    /// Serial of the current decision point; submissions must match it.
    pub decision: u64,
    pub outcome: Option<Outcome>,
    pub rules_revision: String,
}

impl GameState {
    pub fn player(&self, p: PlayerId) -> &Player {
        self.players
            .get(p as usize)
            .unwrap_or_else(|| panic!("seat P{p} is not in this game"))
    }

    pub fn try_player(&self, p: PlayerId) -> Option<&Player> {
        self.players.get(p as usize)
    }

    /// Number of ordered seats. `players.len()` is the single source of truth.
    pub fn seat_count(&self) -> usize {
        self.players.len()
    }

    pub fn is_live(&self, p: PlayerId) -> bool {
        self.try_player(p).is_some_and(|player| !player.lost)
    }

    pub fn live_count(&self) -> usize {
        self.players.iter().filter(|p| !p.lost).count()
    }

    pub fn live_seats(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.lost)
            .map(|(index, _)| index as PlayerId)
    }

    /// Bitmask of live seats, for the pass-round comparison (max four seats).
    pub fn live_mask(&self) -> u8 {
        self.live_seats().fold(0u8, |mask, p| mask | (1u8 << p))
    }

    /// Next live seat in turn order after `after`, wrapping and skipping
    /// eliminated seats (CR 800.4k). Departed seats are never renumbered.
    pub fn next_live_seat_after(&self, after: PlayerId) -> Option<PlayerId> {
        let n = self.seat_count();
        if n == 0 {
            return None;
        }
        for offset in 1..=n {
            let index = (after as usize + offset) % n;
            if self.players[index].lost {
                continue;
            }
            return Some(index as PlayerId);
        }
        None
    }

    /// Cumulative combat damage `commander` has dealt to `recipient`
    /// (CR 903.10a). Zero when no row exists.
    pub fn commander_damage_dealt(&self, commander: CommanderId, recipient: PlayerId) -> u32 {
        self.commander_damage
            .iter()
            .find(|entry| entry.commander == commander && entry.recipient == recipient)
            .map(|entry| entry.amount)
            .unwrap_or(0)
    }

    /// Record combat damage dealt to a player by a commander's stable identity.
    /// Rows stay canonical so save/replay and digest output are deterministic.
    pub fn add_commander_damage(
        &mut self,
        commander: CommanderId,
        recipient: PlayerId,
        amount: u32,
    ) {
        if amount == 0 {
            return;
        }
        match self
            .commander_damage
            .iter_mut()
            .find(|entry| entry.commander == commander && entry.recipient == recipient)
        {
            Some(entry) => entry.amount = entry.amount.saturating_add(amount),
            None => {
                self.commander_damage.push(CommanderDamageEntry {
                    commander,
                    recipient,
                    amount,
                });
                self.commander_damage
                    .sort_unstable_by_key(|entry| (entry.commander.0, entry.recipient));
            }
        }
    }

    /// Prune ledger rows only for a recipient who left the game (their loss is
    /// already settled). Rows for a commander whose owner left are historic:
    /// no further damage can arrive, but prior damage is not erased.
    pub fn prune_commander_damage(&mut self) {
        let live = self.live_mask();
        self.commander_damage
            .retain(|entry| live & (1u8 << entry.recipient) != 0);
    }

    /// True when `id` is referenced by any live engine structure. Used only to
    /// prove an internal `Zone::Removed` token archive is inert; ordinary object
    /// references are validated by their owning structures.
    fn object_is_referenced(&self, id: ObjectId) -> bool {
        let target_ref = |target: &crate::action::Target| match target {
            crate::action::Target::Permanent(object) | crate::action::Target::Spell(object) => {
                *object == id
            }
            crate::action::Target::Card(object) => *object == id,
            crate::action::Target::Player(_) => false,
        };
        let trigger_ref =
            |trigger: &TriggerRef| trigger.source == id || trigger.affected == Some(id);
        if self.resolving == Some(id)
            || self.trigger_queue.iter().any(trigger_ref)
            || self.revealed.contains(&id)
            || self.activation_uses.contains_key(&id)
            || self.commanders.iter().any(|commander| commander.object == id)
            || self
                .stack
                .iter()
                .any(|item| {
                    item.object == id
                        || item.affected == Some(id)
                        || item.targets.iter().any(target_ref)
                })
            || self.turn.attackers.contains(&id)
            || self.turn.blocked.contains(&id)
            || self.turn.unpaired_blockers.contains(&id)
            || self
                .turn
                .blockers
                .iter()
                .any(|(blocker, attacker)| *blocker == id || *attacker == id)
            || self.effects.iter().any(|effect| {
                effect.source == Some(id) || effect.target == id
            })
            || self.regeneration_shields.iter().any(|shield| shield.object == id)
            || self.shields.iter().any(|shield| {
                shield.source == Some(id)
                    || matches!(shield.target, crate::layer::ShieldTarget::Permanent(object) if object == id)
            })
        {
            return true;
        }
        if let Some(pending) = &self.pending {
            let hit = match pending {
                Pending::OrderCard { viewed, to_top, .. } => {
                    viewed.contains(&id) || to_top.contains(&id)
                }
                Pending::ApnapSacrifice {
                    choices,
                    source_left,
                    ..
                } => {
                    choices.values().any(|choice| *choice == Some(id))
                        || source_left.is_some_and(|provenance| provenance.source == id)
                }
                Pending::ApnapReveal { revealed, .. } => {
                    revealed.values().any(|cards| cards.contains(&id))
                }
                Pending::CommanderReturn { order, .. } => {
                    order.iter().any(|entry| entry.object == id)
                }
                Pending::CommanderReplacement { object, source, .. } => {
                    *object == id || *source == id
                }
                Pending::CommanderDraw { object, .. } => *object == id,
                Pending::CombatDamageAssignment { order, .. } => order
                    .iter()
                    .any(|choice| choice.attacker == id || choice.source == id),
                Pending::TriggerTarget {
                    trigger,
                    legal,
                    queued,
                    first,
                    chosen,
                    ..
                } => {
                    trigger_ref(trigger)
                        || legal.iter().any(target_ref)
                        || queued.iter().any(trigger_ref)
                        || first.as_ref().is_some_and(target_ref)
                        || chosen.iter().any(target_ref)
                }
                Pending::TriggerChoice { trigger, .. } => trigger_ref(trigger),
                Pending::TriggerMode {
                    trigger, queued, ..
                } => trigger_ref(trigger) || queued.iter().any(trigger_ref),
                Pending::RingTemptation { eligible, .. } => eligible.contains(&id),
                Pending::LegendaryChoice { options, .. } => options.contains(&id),
                Pending::TopCardChoice { card, .. } => *card == id,
                Pending::HideawayChoice { looked, .. } => looked.contains(&id),
                Pending::PlayExiled { options, .. } => options.contains(&id),
                Pending::SearchLibrary {
                    chosen, eligible, ..
                } => chosen.contains(&id) || eligible.contains(&id),
                Pending::Mulligan { .. }
                | Pending::BottomCards { .. }
                | Pending::DeclareAttackers { .. }
                | Pending::DeclareBlockers { .. }
                | Pending::Discard { .. }
                | Pending::EffectDiscard { .. } => false,
            };
            if hit {
                return true;
            }
        }
        false
    }

    /// Reject invalid seat/state configurations with a typed error instead of a
    /// panic or a silent repair.
    pub fn validate(&self) -> Result<(), EngineError> {
        let n = self.seat_count();
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&n) {
            return Err(EngineError::InvalidSeats {
                reason: format!("{n} seats supplied; supported range is 2..=4"),
            });
        }
        for (index, player) in self.players.iter().enumerate() {
            if player.id != index as PlayerId {
                return Err(EngineError::InvalidSeats {
                    reason: format!(
                        "seat {index} carries id P{}; seats must equal their index",
                        player.id
                    ),
                });
            }
        }
        let in_range = |p: PlayerId| (p as usize) < n;
        if !in_range(self.first_player) {
            return Err(EngineError::InvalidState {
                reason: format!("first_player P{} is out of range", self.first_player),
            });
        }
        if !in_range(self.turn.active) {
            return Err(EngineError::InvalidState {
                reason: format!("active seat P{} is out of range", self.turn.active),
            });
        }
        if let Some(p) = self.turn.priority
            && (!in_range(p) || !self.is_live(p))
        {
            return Err(EngineError::InvalidState {
                reason: format!("priority points at eliminated/out-of-range seat P{p}"),
            });
        }
        if let Some(p) = self.turn.defender
            && !in_range(p)
        {
            return Err(EngineError::InvalidState {
                reason: format!("defender P{p} is out of range"),
            });
        }
        if self.turn.passed >> n != 0 {
            return Err(EngineError::InvalidState {
                reason: format!(
                    "pass bitmask {} contains bits outside the {n} seats",
                    self.turn.passed
                ),
            });
        }
        if self.turn.passed & !self.live_mask() != 0 {
            return Err(EngineError::InvalidState {
                reason: format!(
                    "pass bitmask {} contains an eliminated seat",
                    self.turn.passed
                ),
            });
        }
        // A Ring projection is either absent (never materialized) or exactly one
        // canonical slot per seat. Validate every stored reference before any
        // decision code indexes it.
        if let Some(seat) = self.activation_priority
            && !in_range(seat)
        {
            return Err(EngineError::InvalidState {
                reason: "activation priority seat is out of range".to_string(),
            });
        }
        if !self.rings.is_empty() {
            if self.rings.len() != n {
                return Err(EngineError::InvalidState {
                    reason: "Ring state must be empty or have exactly one slot per seat"
                        .to_string(),
                });
            }
            let mut bearers: Vec<ObjectId> = Vec::new();
            for (seat, ring) in self.rings.iter().enumerate() {
                if ring.temptations > 4 {
                    return Err(EngineError::InvalidState {
                        reason: format!("P{seat} Ring level exceeds 4"),
                    });
                }
                if let Some(bearer) = ring.bearer {
                    if ring.temptations == 0 {
                        return Err(EngineError::InvalidState {
                            reason: format!("P{seat} has a Ring-bearer at level 0"),
                        });
                    }
                    if bearers.contains(&bearer) {
                        return Err(EngineError::InvalidState {
                            reason: format!("Ring-bearer {bearer} belongs to two seats"),
                        });
                    }
                    let Some(object) = self.objects.get(&bearer) else {
                        return Err(EngineError::InvalidState {
                            reason: format!("Ring-bearer {bearer} is missing"),
                        });
                    };
                    if object.zone != Zone::Battlefield || object.controller != seat as PlayerId {
                        return Err(EngineError::InvalidState {
                            reason: format!("Ring-bearer {bearer} is not on P{seat}'s battlefield"),
                        });
                    }
                    bearers.push(bearer);
                }
            }
        }
        {
            let mut seen: Vec<PlayerId> = Vec::new();
            for seat in &self.attacked_this_turn {
                if !in_range(*seat) || seen.contains(seat) {
                    return Err(EngineError::InvalidState {
                        reason: "attack history contains an invalid or duplicate seat".to_string(),
                    });
                }
                seen.push(*seat);
            }
        }
        // CR 514/500.5: the Goblin-entry history is per seat and monotonic within
        // a turn, so a saved vector longer than the table is not a state this
        // engine can produce. A zero entry is legitimate: the vector is grown to
        // the seat count by the first Goblin entry of the turn, and seats that
        // have entered nothing so far simply hold zero.
        if self.goblins_entered_this_turn.len() > self.seat_count() {
            return Err(EngineError::InvalidState {
                reason: "Goblin entry history has more seats than the table".to_string(),
            });
        }
        if let Some(pending) = &self.pending {
            let ring_candidates = |player: PlayerId| -> Vec<ObjectId> {
                let mut ids: Vec<ObjectId> = self
                    .objects
                    .values()
                    .filter(|object| {
                        object.zone == Zone::Battlefield
                            && object.controller == player
                            && object.card.def().is_creature()
                    })
                    .map(|object| object.id)
                    .collect();
                ids.sort_unstable();
                ids
            };
            match pending {
                Pending::RingTemptation { player, eligible } => {
                    let expected = ring_candidates(*player);
                    let mut actual = eligible.clone();
                    actual.sort_unstable();
                    let unique = {
                        let mut ids = actual.clone();
                        ids.dedup();
                        ids.len() == actual.len()
                    };
                    let top_ok = self.stack.last().is_some_and(|item| {
                        item.card == CardId::DunedainRangers
                            && matches!(item.entry, StackEntry::Triggered { .. })
                            && item.controller == *player
                    });
                    if !unique || actual != expected || !top_ok {
                        return Err(EngineError::InvalidState {
                            reason: "Ring temptation candidates are not the resolving Rangers' full live controlled creature set"
                                .to_string(),
                        });
                    }
                }
                Pending::EffectDiscard { player } => {
                    let top_ok = self.stack.last().is_some_and(|item| {
                        item.card == CardId::TheRing
                            && matches!(item.entry, StackEntry::Triggered { .. })
                            && item.controller == *player
                    });
                    if !top_ok {
                        return Err(EngineError::InvalidState {
                            reason: "effect discard is not bound to The Ring trigger".to_string(),
                        });
                    }
                }
                Pending::LegendaryChoice { player, options } => {
                    let mut seen: Vec<ObjectId> = Vec::new();
                    let mut names: Vec<&'static str> = Vec::new();
                    for id in options {
                        if seen.contains(id) {
                            return Err(EngineError::InvalidState {
                                reason: "legendary choice repeats an option".to_string(),
                            });
                        }
                        let Some(object) = self.objects.get(id) else {
                            return Err(EngineError::InvalidState {
                                reason: "legendary choice references a missing object".to_string(),
                            });
                        };
                        // Every option is a permanent the SBA owner controls, and
                        // CR 704.5j applies to legendary permanents: an ordinary
                        // same-name card is not a legend-rule choice.
                        if object.zone != Zone::Battlefield
                            || object.controller != *player
                            || !self.is_live(object.controller)
                        {
                            return Err(EngineError::InvalidState {
                                reason: "legendary choice option is not a live permanent the deciding seat controls"
                                    .to_string(),
                            });
                        }
                        if !self.is_legendary_effective(object) {
                            return Err(EngineError::InvalidState {
                                reason: "legendary choice option is not a legendary permanent"
                                    .to_string(),
                            });
                        }
                        seen.push(*id);
                        names.push(object.card.def().name);
                    }
                    if names.len() < 2 || names.iter().any(|name| *name != names[0]) {
                        return Err(EngineError::InvalidState {
                            reason: "legendary choice is not a same-name legendary group"
                                .to_string(),
                        });
                    }
                    // The offer must be the complete group: a save cannot hide a
                    // third legend or offer a truncated choice.
                    let mut group: Vec<ObjectId> = self
                        .objects
                        .values()
                        .filter(|object| {
                            object.zone == Zone::Battlefield
                                && object.controller == *player
                                && object.card.def().name == names[0]
                                && self.is_legendary_effective(object)
                        })
                        .map(|object| object.id)
                        .collect();
                    group.sort_unstable();
                    let mut offered = seen.clone();
                    offered.sort_unstable();
                    if offered != group {
                        return Err(EngineError::InvalidState {
                            reason: "legendary choice does not list the whole legendary group"
                                .to_string(),
                        });
                    }
                }
                Pending::HideawayChoice {
                    player,
                    source,
                    looked,
                } => {
                    if looked.is_empty() {
                        return Err(EngineError::InvalidState {
                            reason: "hideaway look offers no card".to_string(),
                        });
                    }
                    let mut unique = looked.clone();
                    unique.sort_unstable();
                    unique.dedup();
                    if unique.len() != looked.len() {
                        return Err(EngineError::InvalidState {
                            reason: "hideaway look repeats a card".to_string(),
                        });
                    }
                    let source_ok = self.objects.get(source).is_some_and(|object| {
                        object.zone == Zone::Battlefield && object.controller == *player
                    });
                    if !source_ok {
                        return Err(EngineError::InvalidState {
                            reason:
                                "hideaway look names a source that is not this seat's permanent"
                                    .to_string(),
                        });
                    }
                    // The offer must be exactly the current top of the library,
                    // so a save cannot smuggle a card from the middle of it.
                    let library = &self.player(*player).library;
                    if looked.len() > library.len() || library[..looked.len()] != looked[..] {
                        return Err(EngineError::InvalidState {
                            reason: "hideaway look is not the current top of that seat's library"
                                .to_string(),
                        });
                    }
                    let stack_ok = self.stack.last().is_some_and(|item| {
                        item.card == CardId::EvercoatUrsine
                            && matches!(item.entry, StackEntry::Triggered { .. })
                            && item.controller == *player
                    });
                    if !stack_ok {
                        return Err(EngineError::InvalidState {
                            reason: "hideaway look is not bound to the resolving hideaway trigger"
                                .to_string(),
                        });
                    }
                }
                Pending::PlayExiled {
                    player,
                    source,
                    options,
                } => {
                    if options.is_empty() {
                        return Err(EngineError::InvalidState {
                            reason: "hideaway play offers no card".to_string(),
                        });
                    }
                    let mut unique = options.clone();
                    unique.sort_unstable();
                    unique.dedup();
                    if unique.len() != options.len() {
                        return Err(EngineError::InvalidState {
                            reason: "hideaway play repeats a card".to_string(),
                        });
                    }
                    let source_ok = self.objects.get(source).is_some_and(|object| {
                        object.zone == Zone::Battlefield && object.controller == *player
                    });
                    if !source_ok {
                        return Err(EngineError::InvalidState {
                            reason:
                                "hideaway play names a source that is not this seat's permanent"
                                    .to_string(),
                        });
                    }
                    for id in options {
                        let Some(object) = self.objects.get(id) else {
                            return Err(EngineError::InvalidState {
                                reason: "hideaway play names a missing card".to_string(),
                            });
                        };
                        if object.zone != Zone::Exile || !self.face_down_exile.contains(id) {
                            return Err(EngineError::InvalidState {
                                reason: "hideaway play names a card that is not face down in exile"
                                    .to_string(),
                            });
                        }
                        let linked = self.hideaway.iter().any(|link| {
                            link.card == *id && link.source == *source && link.controller == *player
                        });
                        if !linked {
                            return Err(EngineError::InvalidState {
                                reason: "hideaway play names a card exiled with another object"
                                    .to_string(),
                            });
                        }
                        if object.card.def().is_land() && self.turn.lands_played != 0 {
                            return Err(EngineError::InvalidState {
                                reason: "hideaway play offers a land although this turn's land play is used"
                                    .to_string(),
                            });
                        }
                    }
                    let stack_ok = self.stack.last().is_some_and(|item| {
                        item.card == CardId::EvercoatUrsine
                            && matches!(item.entry, StackEntry::Triggered { .. })
                            && item.controller == *player
                    });
                    if !stack_ok {
                        return Err(EngineError::InvalidState {
                            reason: "hideaway play is not bound to the resolving hideaway trigger"
                                .to_string(),
                        });
                    }
                }
                _ => {}
            }
            if self.turn.priority.is_some() {
                return Err(EngineError::InvalidState {
                    reason: "a pending choice cannot coexist with a priority holder (CR 608.2d)"
                        .to_string(),
                });
            }
            // Structural bounds must be checked before `player()` indexes the
            // canonical queue; malformed saves are typed errors, never panics.
            if let Pending::CommanderReturn {
                order,
                cursor,
                answers,
            } = pending
                && (order.is_empty() || *cursor >= order.len() || answers.len() != *cursor)
            {
                return Err(EngineError::InvalidState {
                    reason: "commander return cursor/answers are not a valid non-empty prefix"
                        .to_string(),
                });
            }
            let owner = pending.player();
            if !in_range(owner) || !self.is_live(owner) {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "pending choice belongs to eliminated/out-of-range seat P{owner}"
                    ),
                });
            }
            if let Pending::ApnapSacrifice {
                player,
                order,
                choices,
                source_left,
            } = pending
            {
                if order.is_empty() || order[0] != *player {
                    return Err(EngineError::InvalidState {
                        reason: "APNAP pending player is not the first remaining seat".to_string(),
                    });
                }
                let mut seen: Vec<PlayerId> = Vec::new();
                for seat in order {
                    if !in_range(*seat) || !self.is_live(*seat) || seen.contains(seat) {
                        return Err(EngineError::InvalidState {
                            reason: format!("APNAP order contains invalid/duplicate P{seat}"),
                        });
                    }
                    seen.push(*seat);
                }
                let mut chosen_objects: Vec<ObjectId> = Vec::new();
                for (seat, choice) in choices {
                    if !in_range(*seat)
                        || !self.is_live(*seat)
                        || order.contains(seat)
                        || seen.contains(seat)
                    {
                        return Err(EngineError::InvalidState {
                            reason: format!("APNAP choices contain invalid seat P{seat}"),
                        });
                    }
                    seen.push(*seat);
                    match choice {
                        Some(id) => {
                            let Some(object) = self.objects.get(id) else {
                                return Err(EngineError::InvalidState {
                                    reason: format!("APNAP choice references missing object {id}"),
                                });
                            };
                            if object.zone != Zone::Battlefield || object.controller != *seat {
                                return Err(EngineError::InvalidState {
                                    reason: format!(
                                        "APNAP choice {id} is not controlled by P{seat}"
                                    ),
                                });
                            }
                            if chosen_objects.contains(id) {
                                return Err(EngineError::InvalidState {
                                    reason: format!("APNAP choice {id} was chosen twice"),
                                });
                            }
                            chosen_objects.push(*id);
                        }
                        None => {
                            if !self.battlefield(*seat).is_empty() {
                                return Err(EngineError::InvalidState {
                                    reason: format!(
                                        "P{seat} chose None while controlling a permanent"
                                    ),
                                });
                            }
                        }
                    }
                }
                let mut live: Vec<PlayerId> = self.live_seats().collect();
                live.sort_unstable();
                seen.sort_unstable();
                if seen != live {
                    return Err(EngineError::InvalidState {
                        reason: "APNAP order+choices do not cover every live seat".to_string(),
                    });
                }
                let anchor = self.turn.active;
                let mut full: Vec<PlayerId> = Vec::new();
                for offset in 0..n {
                    let seat = ((anchor as usize + offset) % n) as PlayerId;
                    if self.is_live(seat) {
                        full.push(seat);
                    }
                }
                let answered: Vec<PlayerId> = full.iter().copied().take(choices.len()).collect();
                let mut choice_seats: Vec<PlayerId> = choices.keys().copied().collect();
                choice_seats.sort_unstable();
                let mut prefix = answered.clone();
                prefix.sort_unstable();
                if choice_seats != prefix {
                    return Err(EngineError::InvalidState {
                        reason: "APNAP answers are not a prefix of active-anchored turn order"
                            .to_string(),
                    });
                }
                let expected: Vec<PlayerId> = full.iter().copied().skip(choices.len()).collect();
                if *order != expected {
                    return Err(EngineError::InvalidState {
                        reason: "APNAP order is not the remaining active-anchored suffix"
                            .to_string(),
                    });
                }
                let Some(resolving) = self.resolving else {
                    return Err(EngineError::InvalidState {
                        reason: "APNAP queue is not bound to a resolving spell".to_string(),
                    });
                };
                let Some(source) = self.objects.get(&resolving) else {
                    return Err(EngineError::InvalidState {
                        reason: format!("APNAP queue source object {resolving} is missing"),
                    });
                };
                let crack_index = source.card.def().spell_spec().and_then(|spec| {
                    spec.effects
                        .iter()
                        .position(|effect| *effect == Effect::EachPlayerSacrifice)
                });
                let source_ok = match source_left {
                    None => {
                        // Normal suspended queue: exact source on top of stack
                        // with the recorded Crack cursor.
                        self.stack.last().is_some_and(|top| {
                            top.object == resolving
                                && crack_index.is_some_and(|index| top.cursor as usize == index + 1)
                        })
                    }
                    Some(provenance) => {
                        // Strong serialized provenance: no phantom stack entry,
                        // exact object/card/cursor/owner/controller/departure
                        // facts, and a legitimate 800.4a source zone.
                        provenance.source == resolving
                            && !self.stack.iter().any(|item| item.object == resolving)
                            && provenance.card == source.card
                            && provenance.card.def().spell_spec().is_some_and(|spec| {
                                provenance.cursor >= 1
                                    && spec.effects.get(provenance.cursor as usize - 1)
                                        == Some(&Effect::EachPlayerSacrifice)
                            })
                            && provenance.prior_controller == provenance.departed
                            && !self.is_live(provenance.departed)
                            && provenance.owner == source.owner
                            && provenance.prior_controller == source.controller
                            && provenance.zone == source.zone
                            && match provenance.zone {
                                Zone::Removed => provenance.owner == provenance.departed,
                                Zone::Exile => provenance.owner != provenance.departed,
                                _ => false,
                            }
                    }
                };
                if !source_ok {
                    return Err(EngineError::InvalidState {
                        reason:
                            "APNAP queue source/cursor/provenance is not a resolving Crack the Earth effect"
                                .to_string(),
                    });
                }
            }
            if let Pending::ApnapReveal {
                player,
                order,
                revealed,
            } = pending
            {
                if order.is_empty() || order[0] != *player {
                    return Err(EngineError::InvalidState {
                        reason: "APNAP reveal pending player is not the first remaining seat"
                            .to_string(),
                    });
                }
                let anchor = self.turn.active;
                let mut full: Vec<PlayerId> = Vec::new();
                for offset in 0..n {
                    let seat = ((anchor as usize + offset) % n) as PlayerId;
                    if self.is_live(seat) {
                        full.push(seat);
                    }
                }
                let mut keys: Vec<PlayerId> = revealed.keys().copied().collect();
                keys.sort_unstable();
                let mut answered: Vec<PlayerId> =
                    full.iter().copied().take(revealed.len()).collect();
                answered.sort_unstable();
                if keys != answered {
                    return Err(EngineError::InvalidState {
                        reason:
                            "APNAP reveal answers are not a prefix of active-anchored turn order"
                                .to_string(),
                    });
                }
                let expected: Vec<PlayerId> = full.iter().copied().skip(revealed.len()).collect();
                if *order != expected {
                    return Err(EngineError::InvalidState {
                        reason: "APNAP reveal order is not the remaining active-anchored suffix"
                            .to_string(),
                    });
                }
                for (seat, cards) in revealed {
                    let hand = &self.player(*seat).hand;
                    let mut seen: Vec<ObjectId> = Vec::new();
                    for id in cards {
                        if seen.contains(id) {
                            return Err(EngineError::InvalidState {
                                reason: format!("P{seat} revealed {id} twice"),
                            });
                        }
                        seen.push(*id);
                        if !hand.contains(id) {
                            return Err(EngineError::InvalidState {
                                reason: format!("P{seat} revealed {id}, which is not in hand"),
                            });
                        }
                        let Some(object) = self.objects.get(id) else {
                            return Err(EngineError::InvalidState {
                                reason: format!("revealed object {id} is missing"),
                            });
                        };
                        if !object.card.def().is_creature() || object.zone != Zone::Hand {
                            return Err(EngineError::InvalidState {
                                reason: format!("revealed {id} is not a creature card in hand"),
                            });
                        }
                    }
                }
                let Some(resolving) = self.resolving else {
                    return Err(EngineError::InvalidState {
                        reason: "APNAP reveal queue is not bound to a resolving spell".to_string(),
                    });
                };
                let Some(source) = self.objects.get(&resolving) else {
                    return Err(EngineError::InvalidState {
                        reason: format!("APNAP reveal source object {resolving} is missing"),
                    });
                };
                let reveal_index = source.card.def().spell_spec().and_then(|spec| {
                    spec.effects.iter().position(|effect| {
                        *effect == Effect::EachPlayerMayRevealCreatureCardsForBears
                    })
                });
                let source_ok = self.stack.last().is_some_and(|top| {
                    top.object == resolving
                        && reveal_index.is_some_and(|index| top.cursor as usize == index + 1)
                });
                if !source_ok {
                    return Err(EngineError::InvalidState {
                        reason:
                            "APNAP reveal queue source/cursor is not a resolving Kamahl's Summons"
                                .to_string(),
                    });
                }
            }
            // Validate all serialized trigger references, including queued siblings.
            match pending {
                Pending::TriggerTarget {
                    trigger, queued, ..
                }
                | Pending::TriggerMode {
                    trigger, queued, ..
                } => {
                    trigger.validate_x()?;
                    self.validate_affected_player(
                        trigger.card,
                        trigger.ability,
                        trigger.affected_player,
                        "pending trigger",
                    )?;
                    for next in queued {
                        next.validate_x()?;
                        self.validate_affected_player(
                            next.card,
                            next.ability,
                            next.affected_player,
                            "queued sibling trigger",
                        )?;
                    }
                }
                Pending::TriggerChoice { trigger, .. } => {
                    trigger.validate_x()?;
                    self.validate_affected_player(
                        trigger.card,
                        trigger.ability,
                        trigger.affected_player,
                        "pending trigger",
                    )?;
                }
                _ => {}
            }
            // Use the same printed-mode and live-candidate guard as answer submission.
            if matches!(pending, Pending::TriggerTarget { .. }) {
                crate::engine::Game {
                    state: self.clone(),
                }
                .validate_trigger_target_pending()?;
            }
        }
        if let Some(mulligan) = &self.mulligan {
            if mulligan.round == 0 {
                return Err(EngineError::InvalidState {
                    reason: "mulligan round 0 is invalid".to_string(),
                });
            }
            if mulligan.eligible.is_empty() || mulligan.order != mulligan.eligible {
                return Err(EngineError::InvalidState {
                    reason: "mulligan order must exactly equal the eligible set".to_string(),
                });
            }
            let mut seen: Vec<PlayerId> = Vec::new();
            for seat in &mulligan.order {
                if !in_range(*seat) || !self.is_live(*seat) || seen.contains(seat) {
                    return Err(EngineError::InvalidState {
                        reason: format!("mulligan order contains invalid/duplicate P{seat}"),
                    });
                }
                seen.push(*seat);
            }
            let mut live: Vec<PlayerId> = self.live_seats().collect();
            live.sort_unstable();
            if mulligan.round == 1 {
                if !mulligan.previous_mulliganers.is_empty() || !mulligan.kept.is_empty() {
                    return Err(EngineError::InvalidState {
                        reason: "first mulligan round cannot carry kept/previous seats".to_string(),
                    });
                }
                let mut expected: Vec<PlayerId> = Vec::new();
                for offset in 0..n {
                    let seat = ((self.first_player as usize + offset) % n) as PlayerId;
                    if self.is_live(seat) {
                        expected.push(seat);
                    }
                }
                if mulligan.eligible != expected {
                    return Err(EngineError::InvalidState {
                        reason: "first mulligan round must start at the starting player and cover every live seat"
                            .to_string(),
                    });
                }
            } else {
                if mulligan.previous_mulliganers != mulligan.eligible {
                    return Err(EngineError::InvalidState {
                        reason: "later mulligan round is not exactly the previous mulliganers"
                            .to_string(),
                    });
                }
                let mut accounted = mulligan.kept.clone();
                accounted.extend(mulligan.eligible.iter().copied());
                accounted.sort_unstable();
                accounted.dedup();
                if accounted != live {
                    return Err(EngineError::InvalidState {
                        reason: "kept + eligible seats must cover exactly every live seat"
                            .to_string(),
                    });
                }
                if mulligan
                    .kept
                    .iter()
                    .any(|seat| !self.is_live(*seat) || mulligan.eligible.contains(seat))
                {
                    return Err(EngineError::InvalidState {
                        reason: "kept seats must be live and disjoint from eligible seats"
                            .to_string(),
                    });
                }
                if mulligan
                    .eligible
                    .iter()
                    .any(|seat| self.player(*seat).mulligans == 0)
                {
                    return Err(EngineError::InvalidState {
                        reason: "later-round eligible seats must have taken a mulligan".to_string(),
                    });
                }
            }
            let mut expected_order: Vec<PlayerId> = Vec::new();
            for offset in 0..n {
                let seat = ((self.first_player as usize + offset) % n) as PlayerId;
                if self.is_live(seat) && mulligan.eligible.contains(&seat) {
                    expected_order.push(seat);
                }
            }
            if mulligan.order != expected_order {
                return Err(EngineError::InvalidState {
                    reason: "mulligan order is not the starting-player anchored eligible sequence"
                        .to_string(),
                });
            }
            if mulligan.cursor > mulligan.order.len()
                || mulligan.declarations.len() != mulligan.cursor
            {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "mulligan cursor {} does not match {} declaration(s)",
                        mulligan.cursor,
                        mulligan.declarations.len()
                    ),
                });
            }
            for (seat, keep) in &mulligan.declarations {
                if !mulligan.order[..mulligan.cursor].contains(seat) {
                    return Err(EngineError::InvalidState {
                        reason: format!("mulligan declaration for undeclared seat P{seat}"),
                    });
                }
                if *keep && mulligan.bottom_queue.iter().any(|queued| queued == seat) {
                    return Err(EngineError::InvalidState {
                        reason: format!("kept seat P{seat} is still awaiting a bottom choice"),
                    });
                }
            }
            match &self.pending {
                Some(Pending::Mulligan { player }) => {
                    if !mulligan.bottom_queue.is_empty() || !mulligan.bottomed.is_empty() {
                        return Err(EngineError::InvalidState {
                            reason: "declaration phase cannot carry a bottom queue".to_string(),
                        });
                    }
                    if mulligan.cursor >= mulligan.order.len()
                        || mulligan.order[mulligan.cursor] != *player
                    {
                        return Err(EngineError::InvalidState {
                            reason: format!("mulligan pending P{player} is not the next declarer"),
                        });
                    }
                    if mulligan.eligible.iter().any(|seat| {
                        self.player(*seat).mulligans as u16 != (mulligan.round - 1) as u16
                    }) {
                        return Err(EngineError::InvalidState {
                            reason: "declaration-phase mulligan counts must equal round - 1"
                                .to_string(),
                        });
                    }
                }
                Some(Pending::BottomCards { player, count }) => {
                    if mulligan.cursor != mulligan.order.len()
                        || mulligan.declarations.len() != mulligan.order.len()
                    {
                        return Err(EngineError::InvalidState {
                            reason: "bottom phase requires every eligible seat to have declared"
                                .to_string(),
                        });
                    }
                    let paid = |mulligans: u8| {
                        if n >= 3 {
                            mulligans.saturating_sub(1)
                        } else {
                            mulligans
                        }
                    };
                    let paid_count = paid(self.player(*player).mulligans);
                    if paid_count == 0 || *count != paid_count {
                        return Err(EngineError::InvalidState {
                            reason: format!(
                                "BottomCards count {count} does not match paid mulligans {paid_count}"
                            ),
                        });
                    }
                    for seat in &mulligan.order {
                        let expected = match mulligan.declarations.get(seat) {
                            Some(false) => mulligan.round as u16,
                            Some(true) => (mulligan.round - 1) as u16,
                            None => {
                                return Err(EngineError::InvalidState {
                                    reason: "bottom phase has an undeclared eligible seat"
                                        .to_string(),
                                });
                            }
                        };
                        if self.player(*seat).mulligans as u16 != expected {
                            return Err(EngineError::InvalidState {
                                reason:
                                    "bottom-phase mulligan counts do not match keep/mulligan declarations"
                                        .to_string(),
                            });
                        }
                    }
                    let owed: Vec<PlayerId> = mulligan
                        .order
                        .iter()
                        .copied()
                        .filter(|seat| {
                            mulligan.declarations.get(seat) == Some(&false)
                                && paid(self.player(*seat).mulligans) > 0
                        })
                        .collect();
                    let mut combined = mulligan.bottomed.clone();
                    combined.extend(mulligan.bottom_queue.iter().copied());
                    if combined != owed {
                        return Err(EngineError::InvalidState {
                            reason:
                                "paid bottom queue is not the complete ordered remaining suffix"
                                    .to_string(),
                        });
                    }
                    if mulligan.bottom_queue.first() != Some(player) {
                        return Err(EngineError::InvalidState {
                            reason: format!(
                                "bottom pending P{player} is not the next bottom choice"
                            ),
                        });
                    }
                    let starting_hand = |seat: PlayerId| {
                        let player = self.player(seat);
                        (player.hand.len() + player.library.len()).min(7)
                    };
                    for seat in &mulligan.bottomed {
                        let expected = starting_hand(*seat)
                            .saturating_sub(paid(self.player(*seat).mulligans) as usize);
                        if self.player(*seat).hand.len() != expected {
                            return Err(EngineError::InvalidState {
                                reason: format!(
                                    "P{seat} is marked bottomed but its hand size is not the paid-bottom result"
                                ),
                            });
                        }
                    }
                    for seat in &mulligan.bottom_queue {
                        if self.player(*seat).hand.len() != starting_hand(*seat) {
                            return Err(EngineError::InvalidState {
                                reason: format!(
                                    "P{seat} is awaiting a bottom but its hand is not the redrawn hand"
                                ),
                            });
                        }
                    }
                }
                _ => {
                    return Err(EngineError::InvalidState {
                        reason: "mulligan state without a mulligan/bottom pending choice"
                            .to_string(),
                    });
                }
            }
        } else if matches!(
            self.pending,
            Some(Pending::Mulligan { .. } | Pending::BottomCards { .. })
        ) {
            return Err(EngineError::InvalidState {
                reason: "mulligan pending choice without mulligan state".to_string(),
            });
        }
        if let Some(Pending::TopCardChoice {
            player,
            card,
            can_take,
        }) = &self.pending
        {
            let eligible = self.try_object(*card).is_some_and(|object| {
                object.zone == Zone::Library
                    && (object.card.def().is_creature()
                        || object.card.def().type_line.contains("Planeswalker"))
            });
            if !self.is_live(*player)
                || self.player(*player).library.first() != Some(card)
                || *can_take != eligible
            {
                return Err(EngineError::InvalidState {
                    reason: "top-card choice is not anchored to the eligible top of the player's library"
                        .to_string(),
                });
            }
        }
        if let Some(Pending::SearchLibrary {
            player,
            chosen,
            eligible,
            remaining,
            mode,
            looked,
        }) = &self.pending
        {
            if !self.is_live(*player) || self.resolving.is_none() {
                return Err(EngineError::InvalidState {
                    reason: "library search must belong to a live seat inside a resolution"
                        .to_string(),
                });
            }
            // Guard Titania before branching on an untrusted saved mode.
            let modal_source = self.stack.last().filter(|item| {
                item.entry == StackEntry::Spell
                    && item.card == CardId::TitaniasCommand
                    && self.resolving == Some(item.object)
            });
            if *mode == LibrarySearchMode::AnyLandToBattlefield || modal_source.is_some() {
                let anchored = modal_source.is_some_and(|item| {
                    let search_cursor = 1 + u8::from(item.modes.contains(&0));
                    item.controller == *player
                        && item.modes.contains(&1)
                        && item.cursor == search_cursor
                        && *mode == LibrarySearchMode::AnyLandToBattlefield
                        && chosen.len() + usize::from(*remaining) == 2
                });
                if !anchored {
                    return Err(EngineError::InvalidState {
                        reason:
                            "modal land search is not anchored to its selected two-land instruction"
                                .to_string(),
                    });
                }
            }
            let cub_source = self.stack.last().filter(|item| {
                item.card == CardId::OwlbearCub && self.resolving == Some(item.object)
            });
            if *mode == LibrarySearchMode::CreatureFromTopToBattlefieldAttacking
                || cub_source.is_some()
            {
                let anchored = cub_source.is_some_and(|item| {
                    item.entry == (StackEntry::Triggered { ability: 0 })
                        && item.controller == *player
                        && item.cursor == 1
                        && self
                            .turn
                            .defender
                            .is_some_and(|defender| self.is_live(defender))
                        && *mode == LibrarySearchMode::CreatureFromTopToBattlefieldAttacking
                });
                if !anchored {
                    return Err(EngineError::InvalidState {
                        reason: "private look is not anchored to its resolving Cub instruction"
                            .to_string(),
                    });
                }
            }
            if *mode == LibrarySearchMode::CreatureFromTopToBattlefieldAttacking {
                // CR 701.20a: a look, not a search. The examined cards must be
                // exactly the top of the library, in order, and nothing but the
                // looked-at cards may be referenced; only a creature card among
                // them can be eligible, and at most one card can be put.
                let library = &self.player(*player).library;
                if looked.len() != library.len().min(8) {
                    return Err(EngineError::InvalidState {
                        reason: "a look must examine exactly the top min(8, library size) cards"
                            .to_string(),
                    });
                }
                if library[..looked.len()] != looked[..] {
                    return Err(EngineError::InvalidState {
                        reason: "the looked-at cards are not the top of that player's library"
                            .to_string(),
                    });
                }
                if usize::from(*remaining) + chosen.len() != 1 {
                    return Err(EngineError::InvalidState {
                        reason: "a look may put at most one creature card onto the battlefield"
                            .to_string(),
                    });
                }
                let is_looked_creature = |id: &ObjectId| {
                    looked.contains(id)
                        && self.try_object(*id).is_some_and(|object| {
                            object.zone == Zone::Library
                                && object.owner == *player
                                && object.card.def().is_creature()
                        })
                };
                for id in eligible {
                    if !is_looked_creature(id) {
                        return Err(EngineError::InvalidState {
                            reason: format!(
                                "the look offers {id}, which is not a creature card among the looked-at cards"
                            ),
                        });
                    }
                }
                let expected_eligible: Vec<ObjectId> = looked
                    .iter()
                    .copied()
                    .filter(|id| is_looked_creature(id))
                    .collect();
                if *eligible != expected_eligible {
                    return Err(EngineError::InvalidState {
                        reason: "the look must list exactly its eligible creature cards once"
                            .to_string(),
                    });
                }
                for id in chosen {
                    if !eligible.contains(id) {
                        return Err(EngineError::InvalidState {
                            reason: "the look chose a card that was never eligible".to_string(),
                        });
                    }
                }
            } else {
                if !looked.is_empty() {
                    return Err(EngineError::InvalidState {
                        reason: "a land search cannot carry a private top-card look".to_string(),
                    });
                }
                if matches!(
                    mode,
                    LibrarySearchMode::SnowLandToHand
                        | LibrarySearchMode::GoblinCardToHand
                        | LibrarySearchMode::GoblinPermanentToBattlefield
                ) && chosen.len() + usize::from(*remaining) != 1
                {
                    return Err(EngineError::InvalidState {
                        reason: "a single-card search must select at most one card, with remaining capacity accounting for its choice".to_string(),
                    });
                }
                let library = &self.player(*player).library;
                let is_search_candidate = |id: &ObjectId| {
                    self.try_object(*id).is_some_and(|object| {
                        object.zone == Zone::Library
                            && object.owner == *player
                            && match mode {
                                LibrarySearchMode::BasicLandToBattlefield => {
                                    object.card.def().is_land()
                                        && object.card.def().basic_mana.is_some()
                                }
                                LibrarySearchMode::SnowLandToHand => {
                                    object.card.def().is_land() && object.card.def().is_snow()
                                }
                                LibrarySearchMode::AnyLandToBattlefield => {
                                    object.card.def().is_land()
                                }
                                LibrarySearchMode::CreatureFromTopToBattlefieldAttacking => false,
                                LibrarySearchMode::GoblinCardToHand => {
                                    object.card.def().is_goblin()
                                }
                                LibrarySearchMode::GoblinPermanentToBattlefield => {
                                    object.card.def().is_goblin()
                                        && object.card.def().is_permanent_card()
                                }
                            }
                    }) && library.contains(id)
                };
                let mut seen_eligible: Vec<ObjectId> = Vec::new();
                for id in eligible {
                    if !is_search_candidate(id) {
                        return Err(EngineError::InvalidState {
                            reason: format!(
                                "library search references {id}, which is not a matching card in that player's library"
                            ),
                        });
                    }
                    if seen_eligible.contains(id) {
                        return Err(EngineError::InvalidState {
                            reason: format!("library search lists eligible card {id} twice"),
                        });
                    }
                    seen_eligible.push(*id);
                }
                let mut seen_chosen: Vec<ObjectId> = Vec::new();
                for id in chosen {
                    if !is_search_candidate(id) {
                        return Err(EngineError::InvalidState {
                            reason: format!(
                                "library search references {id}, which is not a matching card in that player's library"
                            ),
                        });
                    }
                    if seen_chosen.contains(id) {
                        return Err(EngineError::InvalidState {
                            reason: format!("library search chose {id} twice"),
                        });
                    }
                    if !eligible.contains(id) {
                        return Err(EngineError::InvalidState {
                            reason: "library search chose a card that was never eligible"
                                .to_string(),
                        });
                    }
                    seen_chosen.push(*id);
                }
            }
        }
        if let Some(Pending::CommanderReturn {
            order,
            cursor,
            answers,
        }) = &self.pending
        {
            if order.is_empty() || *cursor >= order.len() || answers.len() != *cursor {
                return Err(EngineError::InvalidState {
                    reason: "commander return cursor/answers are not a valid prefix".to_string(),
                });
            }
            let n = self.seat_count();
            let anchor = self.turn.active;
            let mut seen_commanders: Vec<CommanderId> = Vec::new();
            let mut seen_owners: Vec<PlayerId> = Vec::new();
            let mut offsets: Vec<usize> = Vec::new();
            for entry in order {
                if !self.is_live(entry.player) {
                    return Err(EngineError::InvalidState {
                        reason: format!("commander return owner P{} is not live", entry.player),
                    });
                }
                if seen_commanders.contains(&entry.commander) || seen_owners.contains(&entry.player)
                {
                    return Err(EngineError::InvalidState {
                        reason: "commander return queue duplicates a commander or owner"
                            .to_string(),
                    });
                }
                seen_commanders.push(entry.commander);
                seen_owners.push(entry.player);
                let Some(registry) = self.commanders.iter().find(|c| c.id == entry.commander)
                else {
                    return Err(EngineError::InvalidState {
                        reason: format!("commander return references missing {}", entry.commander),
                    });
                };
                if registry.owner != entry.player
                    || registry.object != entry.object
                    || !registry.return_eligible
                {
                    return Err(EngineError::InvalidState {
                        reason: "commander return entry does not match an armed registry"
                            .to_string(),
                    });
                }
                let Some(object) = self.objects.get(&entry.object) else {
                    return Err(EngineError::InvalidState {
                        reason: format!("commander return object {} is missing", entry.object),
                    });
                };
                if object.zone != entry.zone || !matches!(entry.zone, Zone::Graveyard | Zone::Exile)
                {
                    return Err(EngineError::InvalidState {
                        reason: "commander return object/zone is stale".to_string(),
                    });
                }
                offsets.push((entry.player as usize + n - anchor as usize) % n);
            }
            if offsets.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(EngineError::InvalidState {
                    reason: "commander return queue is not in active-anchored APNAP order"
                        .to_string(),
                });
            }
            let armed: Vec<CommanderId> = self
                .commanders
                .iter()
                .filter(|commander| {
                    commander.return_eligible
                        && self.objects.get(&commander.object).is_some_and(|object| {
                            matches!(object.zone, Zone::Graveyard | Zone::Exile)
                        })
                })
                .map(|commander| commander.id)
                .collect();
            let mut expected = armed.clone();
            let mut actual = seen_commanders.clone();
            expected.sort_unstable();
            actual.sort_unstable();
            if expected != actual {
                return Err(EngineError::InvalidState {
                    reason: "commander return queue does not exactly cover every armed commander"
                        .to_string(),
                });
            }
        }
        match &self.pending {
            Some(Pending::CommanderReplacement {
                player,
                commander,
                object,
                from_zone,
                destination,
                source,
                cursor,
            }) => {
                if !matches!(destination, Zone::Hand | Zone::Library) {
                    return Err(EngineError::InvalidState {
                        reason: "commander replacement destination is not hand/library".to_string(),
                    });
                }
                let Some(registry) = self.commanders.iter().find(|c| c.id == *commander) else {
                    return Err(EngineError::InvalidState {
                        reason: "commander replacement references a missing commander".to_string(),
                    });
                };
                if registry.owner != *player || registry.object != *object {
                    return Err(EngineError::InvalidState {
                        reason: "commander replacement registry/object mismatch".to_string(),
                    });
                }
                let Some(current) = self.objects.get(object) else {
                    return Err(EngineError::InvalidState {
                        reason: "commander replacement object is missing".to_string(),
                    });
                };
                if current.zone != *from_zone
                    || !matches!(*from_zone, Zone::Battlefield | Zone::Graveyard)
                {
                    return Err(EngineError::InvalidState {
                        reason: "commander replacement from_zone is stale".to_string(),
                    });
                }
                let Some(top) = self.stack.last() else {
                    return Err(EngineError::InvalidState {
                        reason: "commander replacement has no stack source".to_string(),
                    });
                };
                if top.object != *source || top.cursor != *cursor || self.resolving != Some(*source)
                {
                    return Err(EngineError::InvalidState {
                        reason: "commander replacement source/cursor does not match resolution"
                            .to_string(),
                    });
                }
                let produced = cursor.checked_sub(1).and_then(|index| {
                    let source_object = self.objects.get(source)?;
                    // The instruction a suspension is anchored to belongs to the
                    // thing on the stack: a spell's printed effects, or a
                    // triggered/activated ability's effects. `cursor` counts
                    // instructions already taken, so `cursor - 1` is the one that
                    // suspended. Reading only spell effects would let a forged
                    // source choose its own destination.
                    let effect = match top.entry {
                        StackEntry::Spell => source_object
                            .card
                            .def()
                            .spell_spec()
                            .and_then(|spec| spec.effects.get(index as usize)),
                        StackEntry::Triggered { ability } | StackEntry::Activated { ability } => {
                            crate::ability::abilities(source_object.card)
                                .get(ability as usize)
                                .and_then(|ability| ability.effects().get(index as usize))
                        }
                    }?;
                    match effect {
                        Effect::ReturnToHand => Some((Zone::Hand, false)),
                        Effect::PutOnTopOfLibrary => Some((Zone::Library, false)),
                        // CR 903.9b: "from anywhere" — a commander card returned
                        // from a graveyard to hand is offered the command zone,
                        // and its target is a card, not a permanent.
                        Effect::ReturnGraveyardCardToHand => Some((Zone::Hand, true)),
                        _ => None,
                    }
                });
                let (expected_destination, card_target) = match produced {
                    Some(produced) => produced,
                    None => {
                        return Err(EngineError::InvalidState {
                            reason:
                                "commander replacement does not match its producer effect/target"
                                    .to_string(),
                        });
                    }
                };
                let expected_from = if card_target {
                    Zone::Graveyard
                } else {
                    Zone::Battlefield
                };
                let expected_target = if card_target {
                    crate::action::Target::Card(*object)
                } else {
                    crate::action::Target::Permanent(*object)
                };
                if expected_destination != *destination
                    || *from_zone != expected_from
                    || top.targets.as_slice() != [expected_target]
                {
                    return Err(EngineError::InvalidState {
                        reason: "commander replacement does not match its producer effect/target"
                            .to_string(),
                    });
                }
            }
            Some(Pending::CombatDamageAssignment {
                order,
                cursor,
                answers,
            }) => {
                if order.is_empty() || *cursor >= order.len() || answers.len() != *cursor {
                    return Err(EngineError::InvalidState {
                        reason: "combat damage assignment cursor/answers are not a valid prefix"
                            .to_string(),
                    });
                }
                // CR 510.1: the choice exists only while combat damage has not
                // been dealt yet, in the combat damage step. A pending outside
                // that window could not have been produced by the engine.
                if self.turn.step != Step::CombatDamage || self.turn.combat_damage_done {
                    return Err(EngineError::InvalidState {
                        reason:
                            "combat damage assignment pending is outside the combat damage step"
                                .to_string(),
                    });
                }
                let n = self.seat_count();
                let anchor = self.turn.active;
                let mut offsets: Vec<usize> = Vec::new();
                let mut seen_attackers: Vec<ObjectId> = Vec::new();
                for choice in order {
                    if !in_range(choice.player) || !self.is_live(choice.player) {
                        return Err(EngineError::InvalidState {
                            reason: format!(
                                "combat damage assignment names non-live seat P{}",
                                choice.player
                            ),
                        });
                    }
                    let Some(source) = self.objects.get(&choice.source) else {
                        return Err(EngineError::InvalidState {
                            reason: "combat damage assignment source is missing".to_string(),
                        });
                    };
                    if source.zone != Zone::Battlefield
                        || source.card != CardId::RuxaPatientProfessor
                        || source.controller != choice.player
                    {
                        return Err(EngineError::InvalidState {
                            reason: "combat damage assignment source is not this seat's Ruxa on the battlefield"
                                .to_string(),
                        });
                    }
                    let Some(attacker) = self.objects.get(&choice.attacker) else {
                        return Err(EngineError::InvalidState {
                            reason: "combat damage assignment attacker is missing".to_string(),
                        });
                    };
                    if attacker.zone != Zone::Battlefield
                        || !attacker.is_creature()
                        || attacker.controller != choice.player
                        || !self.turn.attackers.contains(&choice.attacker)
                        || !self.turn.blocked.contains(&choice.attacker)
                    {
                        return Err(EngineError::InvalidState {
                            reason: "combat damage assignment attacker is not a blocked attacker of that seat"
                                .to_string(),
                        });
                    }
                    if !self.object_has_no_abilities(choice.attacker) {
                        return Err(EngineError::InvalidState {
                            reason: "combat damage assignment attacker has abilities".to_string(),
                        });
                    }
                    if seen_attackers.contains(&choice.attacker) {
                        return Err(EngineError::InvalidState {
                            reason: "combat damage assignment repeats an attacker".to_string(),
                        });
                    }
                    seen_attackers.push(choice.attacker);
                    offsets.push((choice.player as usize + n - anchor as usize) % n);
                }
                if offsets.windows(2).any(|pair| pair[0] > pair[1]) {
                    return Err(EngineError::InvalidState {
                        reason:
                            "combat damage assignment queue is not in active-anchored APNAP order"
                                .to_string(),
                    });
                }
            }
            Some(Pending::CommanderDraw {
                player,
                commander,
                object,
                remaining,
                resume,
            }) => {
                let Some(registry) = self.commanders.iter().find(|c| c.id == *commander) else {
                    return Err(EngineError::InvalidState {
                        reason: "commander draw references a missing commander".to_string(),
                    });
                };
                if registry.owner != *player || registry.object != *object {
                    return Err(EngineError::InvalidState {
                        reason: "commander draw registry/object mismatch".to_string(),
                    });
                }
                let Some(current) = self.objects.get(object) else {
                    return Err(EngineError::InvalidState {
                        reason: "commander draw object is missing".to_string(),
                    });
                };
                if current.zone != Zone::Library
                    || !in_range(*player)
                    || self.player(*player).library.first() != Some(object)
                {
                    return Err(EngineError::InvalidState {
                        reason: "commander draw object is not the top of its owner's library"
                            .to_string(),
                    });
                }
                match resume {
                    DrawResume::Step => {
                        if self.turn.step != Step::Draw
                            || self.turn.active != *player
                            || *remaining != 0
                            || self.resolving.is_some()
                            || !self.stack.is_empty()
                        {
                            return Err(EngineError::InvalidState {
                                reason: "commander draw step resume context is stale".to_string(),
                            });
                        }
                    }
                    DrawResume::EachPlayer {
                        source,
                        cursor,
                        next,
                        cards_left,
                        per_seat,
                        done,
                        live_total,
                    } => {
                        let Some(top) = self.stack.last() else {
                            return Err(EngineError::InvalidState {
                                reason: "each-player draw has no stack source".to_string(),
                            });
                        };
                        if top.object != *source
                            || top.cursor != *cursor
                            || self.resolving != Some(*source)
                            || *cursor == 0
                            || *next != *player
                            || *per_seat == 0
                            || !(1..=*per_seat).contains(cards_left)
                            || *done >= *live_total
                        {
                            return Err(EngineError::InvalidState {
                                reason: "each-player draw resume progress is stale".to_string(),
                            });
                        }
                    }
                    DrawResume::Effect { source, cursor } => {
                        let Some(top) = self.stack.last() else {
                            return Err(EngineError::InvalidState {
                                reason: "commander draw has no stack source".to_string(),
                            });
                        };
                        if top.object != *source
                            || top.cursor != *cursor
                            || self.resolving != Some(*source)
                            || *cursor == 0
                            || top.controller != *player
                        {
                            return Err(EngineError::InvalidState {
                                reason: "commander draw source/cursor does not match resolution"
                                    .to_string(),
                            });
                        }
                        let Some(item) = self.stack.last() else {
                            return Err(EngineError::InvalidState {
                                reason: "commander draw source is missing".to_string(),
                            });
                        };
                        let Some(spec) = self
                            .objects
                            .get(source)
                            .and_then(|o| o.card.def().spell_spec())
                        else {
                            return Err(EngineError::InvalidState {
                                reason: "commander draw source is not a spell".to_string(),
                            });
                        };
                        let index = (*cursor as usize) - 1;
                        let Some(Effect::Draw(n)) = spec.effects.get(index) else {
                            return Err(EngineError::InvalidState {
                                reason: "commander draw cursor is not a Draw effect".to_string(),
                            });
                        };
                        // The cursor indexes instructions, not cards drawn.
                        // Opt/Consider draw after a preceding Scry/Surveil.
                        if *remaining >= *n {
                            return Err(EngineError::InvalidState {
                                reason: "commander draw remaining count does not match Draw(n)"
                                    .to_string(),
                            });
                        }
                        let _ = item;
                    }
                }
            }
            _ => {}
        }
        if let Some(turn) = self.prevent_all_combat_damage_turn
            && turn != self.turn.number
        {
            return Err(EngineError::InvalidState {
                reason: format!(
                    "combat-damage shield is for turn {turn} but the state is turn {}",
                    self.turn.number
                ),
            });
        }
        for object in self.objects.values() {
            if !in_range(object.owner) || !in_range(object.controller) {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "object {} has out-of-range owner P{} or controller P{}",
                        object.id, object.owner, object.controller
                    ),
                });
            }
            // CR 122.2: counters exist only on battlefield permanents; a save
            // that carries counters in any other zone is not a reachable state.
            if !object.counters.is_empty() && object.zone != Zone::Battlefield {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "object {} carries {} counter(s) in the {} zone",
                        object.id,
                        object.counters.describe(),
                        object.zone.name()
                    ),
                });
            }
            // CR 111.7/800.4a: a token exists only on the battlefield. The one
            // exception is internal Removed archival for a token whose owner has
            // already left the game; that record is not a live rules object and
            // must be inert. Any other off-battlefield token is invalid.
            let archival = object.zone == Zone::Removed && !self.is_live(object.owner);
            if object.card.is_token() && object.zone != Zone::Battlefield && !archival {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "token {} ({}) is in the {} zone; tokens cease to exist off the battlefield",
                        object.id,
                        object.card.name(),
                        object.zone.name()
                    ),
                });
            }
            if archival && object.card.is_token() && self.object_is_referenced(object.id) {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "archived token {} is still referenced by a live zone/stack/combat/pending/effect structure",
                        object.id
                    ),
                });
            }
        }
        // Bidirectional container/identity checks. The list->object pass below
        // is not enough: a duplicated or missing zone entry, a key/object id
        // mismatch, or a rewound allocator must be rejected, including sandbox
        // saves.
        let mut list_counts: BTreeMap<ObjectId, u32> = BTreeMap::new();
        for player in &self.players {
            for ids in [
                &player.library,
                &player.hand,
                &player.graveyard,
                &player.exile,
                &player.command_zone,
            ] {
                for id in ids {
                    *list_counts.entry(*id).or_insert(0) += 1;
                }
            }
        }
        // Cards looked at by a pending scry/surveil are drained from the
        // library list but remain Library-zone objects held by the pending
        // choice. Count those virtual holdings exactly once.
        let mut held_counts: BTreeMap<ObjectId, u32> = BTreeMap::new();
        if let Some(Pending::OrderCard {
            player,
            viewed,
            index,
            to_top,
            mode,
            ..
        }) = &self.pending
        {
            if viewed.is_empty() || *index >= viewed.len() {
                return Err(EngineError::InvalidState {
                    reason: "pending OrderCard index is outside its viewed cards".to_string(),
                });
            }
            let Some(resolving) = self.resolving else {
                return Err(EngineError::InvalidState {
                    reason: "pending OrderCard has no resolving source".to_string(),
                });
            };
            let Some(top) = self.stack.last() else {
                return Err(EngineError::InvalidState {
                    reason: "pending OrderCard has no stack source".to_string(),
                });
            };
            if top.object != resolving || top.controller != *player || top.cursor == 0 {
                return Err(EngineError::InvalidState {
                    reason: "pending OrderCard source/controller/cursor is stale".to_string(),
                });
            }
            let Some(spec) = self
                .objects
                .get(&resolving)
                .and_then(|object| object.card.def().spell_spec())
            else {
                return Err(EngineError::InvalidState {
                    reason: "pending OrderCard source is not a spell".to_string(),
                });
            };
            let executed = top.cursor as usize - 1;
            let expected_count = match spec.effects.get(executed) {
                Some(Effect::Scry(n)) if *mode == OrderMode::Scry => *n,
                Some(Effect::Surveil(n)) if *mode == OrderMode::Surveil => *n,
                _ => {
                    return Err(EngineError::InvalidState {
                        reason: "pending OrderCard mode does not match its resolving effect"
                            .to_string(),
                    });
                }
            };
            if expected_count != 1 || viewed.len() != expected_count as usize {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "pending OrderCard viewed count {} does not match supported {expected_count}-card ordering effect",
                        viewed.len()
                    ),
                });
            }
            if to_top.iter().any(|id| !viewed[..*index].contains(id)) {
                return Err(EngineError::InvalidState {
                    reason: "pending OrderCard to_top contains an unanswered/current card"
                        .to_string(),
                });
            }
            let mut seen_viewed: Vec<ObjectId> = Vec::new();
            for (position, id) in viewed.iter().enumerate() {
                if seen_viewed.contains(id) {
                    return Err(EngineError::InvalidState {
                        reason: format!("pending OrderCard repeats viewed object {id}"),
                    });
                }
                seen_viewed.push(*id);
                let processed = position < *index;
                let kept_by_choice = to_top.contains(id);
                match self.objects.get(id) {
                    Some(object) if object.owner == *player && object.zone == Zone::Library => {
                        *held_counts.entry(*id).or_insert(0) += 1;
                    }
                    None if processed && !kept_by_choice && *mode == OrderMode::Surveil => {
                        // A surveyed card (possibly a re-keyed commander) has
                        // already left the library; the historical id is not a
                        // live container reference.
                    }
                    Some(object)
                        if processed
                            && !kept_by_choice
                            && *mode == OrderMode::Scry
                            && object.owner == *player
                            && object.zone == Zone::Library =>
                    {
                        *held_counts.entry(*id).or_insert(0) += 1;
                    }
                    _ => {
                        return Err(EngineError::InvalidState {
                            reason: format!(
                                "pending OrderCard object {id} is not a valid library holding owned by P{player}"
                            ),
                        });
                    }
                }
            }
            let mut seen_top: Vec<ObjectId> = Vec::new();
            for id in to_top {
                if seen_top.contains(id) || !seen_viewed.contains(id) {
                    return Err(EngineError::InvalidState {
                        reason: format!(
                            "pending OrderCard to_top {id} is duplicate or not among viewed cards"
                        ),
                    });
                }
                seen_top.push(*id);
            }
        }
        let mut spell_counts: BTreeMap<ObjectId, u32> = BTreeMap::new();
        for item in &self.stack {
            match item.entry {
                StackEntry::Spell => {
                    validate_saved_x(item.x_value, item.card.def().has_x_cost())?;
                    if item.card.def().has_x_cost()
                        && !self
                            .objects
                            .get(&item.object)
                            .is_some_and(|object| object.card == item.card)
                    {
                        return Err(EngineError::InvalidState {
                            reason: "X spell identity does not match its stack object".to_string(),
                        });
                    }
                }
                StackEntry::Triggered { ability } => {
                    let permits_x = item.card.def().has_x_cost()
                        && crate::ability::abilities(item.card)
                            .get(ability as usize)
                            .is_some_and(|spec| {
                                spec.target_spec() == TargetSpec::ArtifactOrEnchantment
                            });
                    validate_saved_x(item.x_value, permits_x)?;
                }
                StackEntry::Activated { .. } => validate_saved_x(item.x_value, false)?,
            }
            if item.entry == StackEntry::Spell {
                *spell_counts.entry(item.object).or_insert(0) += 1;
            }
        }
        for (key, object) in &self.objects {
            if *key != object.id {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "object map key {} does not equal object.id {}",
                        key, object.id
                    ),
                });
            }
            let listed = list_counts.get(key).copied().unwrap_or(0);
            let held = held_counts.get(key).copied().unwrap_or(0);
            let spells = spell_counts.get(key).copied().unwrap_or(0);
            // Ability stack entries are source references, not zone containers:
            // any number of activated/triggered abilities may share one source.
            // Only a spell object is itself a stack container, and it must be
            // exactly one.
            let ok = match object.zone {
                Zone::Battlefield | Zone::Removed => {
                    listed == 0
                        && held == 0
                        && spells == 0
                        && (object.zone != Zone::Removed || spells == 0)
                }
                Zone::Stack => listed == 0 && held == 0 && spells == 1,
                Zone::Library => listed + held == 1 && spells == 0,
                Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Command => {
                    listed == 1 && held == 0 && spells == 0
                }
            };
            if !ok {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "object {key} in {} has {listed} zone-list and {spells} spell stack container entr(ies); exactly one matching container is required for a spell",
                        object.zone.name()
                    ),
                });
            }
        }
        if self.objects.keys().any(|id| id.0 >= self.next_object) {
            return Err(EngineError::InvalidState {
                reason: format!(
                    "next_object {} does not exceed every allocated object id",
                    self.next_object
                ),
            });
        }
        if self.profile.has_command_zone() {
            if self.commanders.len() != self.live_count() {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "{} needs one commander per live seat; {} registry entr(ies) for {} live seat(s)",
                        self.profile.label(),
                        self.commanders.len(),
                        self.live_count()
                    ),
                });
            }
            for seat in self.live_seats() {
                if self.commanders.iter().filter(|c| c.owner == seat).count() != 1 {
                    return Err(EngineError::InvalidState {
                        reason: format!("live P{seat} does not have exactly one commander"),
                    });
                }
            }
        } else if !self.commanders.is_empty()
            || self
                .players
                .iter()
                .any(|player| !player.command_zone.is_empty())
        {
            return Err(EngineError::InvalidState {
                reason: "ordinary practice game carries command-zone state".to_string(),
            });
        }
        if !self.profile.is_commander_combat() && !self.commander_damage.is_empty() {
            return Err(EngineError::InvalidState {
                reason: format!(
                    "{} cannot carry commander combat damage",
                    self.profile.label()
                ),
            });
        }
        let mut seen_damage: Vec<(CommanderId, PlayerId)> = Vec::new();
        let mut previous: Option<(CommanderId, PlayerId)> = None;
        for entry in &self.commander_damage {
            if entry.amount == 0 || entry.amount >= 21 {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "{} -> P{} carries {} damage; a settled live game must be 1..=20",
                        entry.commander, entry.recipient, entry.amount
                    ),
                });
            }
            if !in_range(entry.recipient) || !self.is_live(entry.recipient) {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "{} damage recipient P{} is eliminated/out-of-range",
                        entry.commander, entry.recipient
                    ),
                });
            }
            let live_source = self
                .commanders
                .iter()
                .any(|commander| commander.id == entry.commander);
            if !live_source && !self.retired_commanders.contains(&entry.commander) {
                return Err(EngineError::InvalidState {
                    reason: format!("commander damage references unknown {}", entry.commander),
                });
            }
            let key = (entry.commander, entry.recipient);
            if seen_damage.contains(&key) {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "duplicate commander damage row for {} -> P{}",
                        entry.commander, entry.recipient
                    ),
                });
            }
            if previous.is_some_and(|previous| previous >= key) {
                return Err(EngineError::InvalidState {
                    reason: "commander damage rows are not in canonical order".to_string(),
                });
            }
            previous = Some(key);
            seen_damage.push(key);
        }
        for (index, retired) in self.retired_commanders.iter().enumerate() {
            if self.commanders.iter().any(|live| live.id == *retired) {
                return Err(EngineError::InvalidState {
                    reason: format!("retired {} is still in the live registry", retired),
                });
            }
            if index > 0 && self.retired_commanders[index - 1] >= *retired {
                return Err(EngineError::InvalidState {
                    reason: "retired commander ids are not strictly ascending".to_string(),
                });
            }
        }
        // Provenance partition: setup allocates exactly CommanderId 1..=seat_count,
        // one per original seat, and every identity ends either live or retired.
        // A fabricated/out-of-range/omitted id therefore fails, with no per-id
        // owner table and no special case.
        if self.profile.has_command_zone() {
            // At most four seats, so identity provenance is a fixed u8 mask:
            // no heap allocation or sort on the success path.
            let mut mask = 0u8;
            let mut add = |id: CommanderId| -> Result<(), EngineError> {
                let raw = id.0;
                if raw == 0 || raw > n as u32 {
                    return Err(EngineError::InvalidState {
                        reason: format!(
                            "commander identity {id} is outside the original 1..={n} seat allocation"
                        ),
                    });
                }
                let bit = 1u8 << (raw - 1);
                if mask & bit != 0 {
                    return Err(EngineError::InvalidState {
                        reason: format!(
                            "commander identity {id} appears more than once across live/retired"
                        ),
                    });
                }
                mask |= bit;
                Ok(())
            };
            for commander in &self.commanders {
                add(commander.id)?;
            }
            for retired in &self.retired_commanders {
                add(*retired)?;
            }
            let expected = if n == 8 {
                u8::MAX
            } else {
                (1u16 << n) as u8 - 1
            };
            if mask != expected {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "commander identity provenance mask {mask:#04x} != expected {expected:#04x}; every original seat identity must be live or retired exactly once"
                    ),
                });
            }
        } else if !self.retired_commanders.is_empty() {
            return Err(EngineError::InvalidState {
                reason: format!(
                    "{} cannot carry retired commander identities",
                    self.profile.label()
                ),
            });
        }
        let mut seen: Vec<CommanderId> = Vec::new();
        for commander in &self.commanders {
            if !in_range(commander.owner) {
                return Err(EngineError::InvalidState {
                    reason: format!("{} has out-of-range owner", commander.id),
                });
            }
            if !commander.card.is_supported_commander() {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "{} is not in the supported commander profile",
                        commander.card.name()
                    ),
                });
            }
            if seen.contains(&commander.id) {
                return Err(EngineError::InvalidState {
                    reason: format!("duplicate commander identity {}", commander.id),
                });
            }
            seen.push(commander.id);
            let Some(object) = self.objects.get(&commander.object) else {
                return Err(EngineError::InvalidState {
                    reason: format!("{} points at a missing object", commander.id),
                });
            };
            if object.commander != Some(commander.id)
                || object.owner != commander.owner
                || object.card != commander.card
                || object.zone == Zone::Removed
            {
                return Err(EngineError::InvalidState {
                    reason: format!("{} registry/object mismatch", commander.id),
                });
            }
        }
        for object in self.objects.values() {
            if let Some(id) = object.commander {
                let Some(registry) = self.commanders.iter().find(|c| c.id == id) else {
                    return Err(EngineError::InvalidState {
                        reason: format!("object {} carries missing commander {id}", object.id),
                    });
                };
                if registry.object != object.id {
                    return Err(EngineError::InvalidState {
                        reason: format!("object {} is not the current object for {id}", object.id),
                    });
                }
            }
        }
        for (index, player) in self.players.iter().enumerate() {
            let p = index as PlayerId;
            for (zone, ids) in [
                (Zone::Library, &player.library),
                (Zone::Hand, &player.hand),
                (Zone::Graveyard, &player.graveyard),
                (Zone::Exile, &player.exile),
                (Zone::Command, &player.command_zone),
            ] {
                for id in ids {
                    let Some(object) = self.objects.get(id) else {
                        return Err(EngineError::InvalidState {
                            reason: format!("P{p} {} references missing object {id}", zone.name()),
                        });
                    };
                    if object.owner != p || object.zone != zone {
                        return Err(EngineError::InvalidState {
                            reason: format!(
                                "P{p} {} references object {id} owned by P{} in {}",
                                zone.name(),
                                object.owner,
                                object.zone.name()
                            ),
                        });
                    }
                }
            }
        }
        for trigger in &self.trigger_queue {
            trigger.validate_x()?;
            self.validate_affected_player(
                trigger.card,
                trigger.ability,
                trigger.affected_player,
                "queued trigger",
            )?;
            if let Some(affected) = trigger.affected
                && !self.objects.contains_key(&affected)
            {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "queued trigger {} references missing affected object {affected}",
                        trigger.source
                    ),
                });
            }
        }
        for item in &self.stack {
            if item.card == CardId::TitaniasCommand && item.entry == StackEntry::Spell {
                let valid_modes = item.modes.len() == 2
                    && item.modes.iter().all(|mode| *mode < 4)
                    && item.modes[0] != item.modes[1];
                let valid_targets = if item.modes.contains(&0) {
                    matches!(item.targets.as_slice(), [crate::action::Target::Player(p)] if in_range(*p))
                } else {
                    item.targets.is_empty()
                };
                if !valid_modes || !valid_targets || item.cursor > 2 {
                    return Err(EngineError::InvalidState {
                        reason: "Titania's Command must retain exactly two distinct printed modes and their targets".to_string(),
                    });
                }
            } else if !item.modes.is_empty() {
                return Err(EngineError::InvalidState {
                    reason: "nonmodal stack item carries modal selections".to_string(),
                });
            }
            if let StackEntry::Triggered { ability } = item.entry {
                self.validate_affected_player(
                    item.card,
                    ability,
                    item.affected_player,
                    "stack trigger",
                )?;
            }
            let Some(object) = self.objects.get(&item.object) else {
                return Err(EngineError::InvalidState {
                    reason: format!("stack references missing object {}", item.object),
                });
            };
            if let Some(affected) = item.affected
                && !self.objects.contains_key(&affected)
            {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "stack item {} references missing affected object {affected}",
                        item.object
                    ),
                });
            }
            if object.zone != Zone::Stack
                && !matches!(
                    item.entry,
                    StackEntry::Triggered { .. } | StackEntry::Activated { .. }
                )
            {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "stack object {} is in the {} zone",
                        item.object,
                        object.zone.name()
                    ),
                });
            }
        }
        let source_left_apnap = matches!(
            &self.pending,
            Some(Pending::ApnapSacrifice {
                source_left: Some(provenance),
                ..
            }) if self.resolving == Some(provenance.source)
        );
        if let Some(resolving) = self.resolving
            && !source_left_apnap
            && !self.stack.iter().any(|item| item.object == resolving)
        {
            return Err(EngineError::InvalidState {
                reason: format!("resolving object {resolving} is not on the stack"),
            });
        }
        for id in self
            .turn
            .attackers
            .iter()
            .chain(self.turn.blocked.iter())
            .chain(self.turn.unpaired_blockers.iter())
        {
            let Some(object) = self.objects.get(id) else {
                return Err(EngineError::InvalidState {
                    reason: format!("combat references missing object {id}"),
                });
            };
            if object.zone != Zone::Battlefield {
                return Err(EngineError::InvalidState {
                    reason: format!("combat object {id} is not on the battlefield"),
                });
            }
        }
        if !self.turn.unpaired_blockers.is_empty() {
            let valid_step = matches!(
                self.turn.step,
                Step::DeclareBlockers | Step::CombatDamage | Step::EndCombat
            );
            let canonical = self
                .turn
                .unpaired_blockers
                .windows(2)
                .all(|pair| pair[0] < pair[1]);
            let game = crate::engine::Game {
                state: self.clone(),
            };
            if !valid_step
                || !canonical
                || self.turn.defender.is_none()
                || self.turn.unpaired_blockers.iter().any(|id| {
                    !game.is_creature(&self.objects[id])
                        || self.objects[id].controller != self.turn.defender.unwrap()
                        || self.turn.attackers.contains(id)
                        || self.turn.blockers.iter().any(|(blocker, _)| blocker == id)
                })
            {
                return Err(EngineError::InvalidState {
                    reason:
                        "unpaired blocking creatures are not canonical live defenders in combat"
                            .into(),
                });
            }
        }
        for (blocker, attacker) in &self.turn.blockers {
            for id in [blocker, attacker] {
                let Some(object) = self.objects.get(id) else {
                    return Err(EngineError::InvalidState {
                        reason: format!("combat references missing object {id}"),
                    });
                };
                if object.zone != Zone::Battlefield {
                    return Err(EngineError::InvalidState {
                        reason: format!("combat object {id} is not on the battlefield"),
                    });
                }
            }
        }
        if let Some(Pending::OrderCard {
            viewed,
            index,
            to_top,
            ..
        }) = &self.pending
        {
            if viewed.is_empty() || *index >= viewed.len() {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "pending OrderCard index {index} is outside {} viewed card(s)",
                        viewed.len()
                    ),
                });
            }
            for id in viewed.iter().chain(to_top.iter()) {
                if !self.objects.contains_key(id) {
                    return Err(EngineError::InvalidState {
                        reason: format!("pending OrderCard references missing object {id}"),
                    });
                }
            }
        }
        for item in &self.stack {
            if !in_range(item.controller) {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "stack object {} has out-of-range controller P{}",
                        item.object, item.controller
                    ),
                });
            }
        }
        if let Some(Outcome::Win { winner, .. }) = &self.outcome
            && (!in_range(*winner) || !self.is_live(*winner))
        {
            return Err(EngineError::InvalidState {
                reason: format!("recorded winner P{winner} is not a live seat"),
            });
        }
        Ok(())
    }

    pub fn object(&self, id: ObjectId) -> &Object {
        self.objects
            .get(&id)
            .expect("object id present in state objects")
    }

    pub fn try_object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }

    /// CR 113.6: "no abilities" is a property of the object's whole ability set —
    /// its printed abilities, any intrinsic mana ability (CR 605.1a) and
    /// anything currently granting it an ability. This one predicate is shared by
    /// Ruxa's graveyard target set, its layer 7c anthem and its combat-damage
    /// permission, so the three printed clauses of one card can never disagree
    /// about which creatures qualify. A missing object is never eligible.
    /// CR 613.1d + CR 205.3: effective subtype, shared by the engine's public
    /// subtype query and the no-abilities predicate. Layer 4 replacement wins in
    /// timestamp order; an additive layer 4 line counts as well.
    pub fn object_has_subtype(&self, object: &Object, subtype: &str) -> bool {
        let mut effective: Option<crate::layer::SubtypeLine> = None;
        let mut added = false;
        if object.zone == Zone::Battlefield && self.turn.active == object.controller {
            for static_ability in crate::ability::static_abilities(object.card) {
                if let crate::ability::StaticAbility::SelfBecomesSubtypeDuringYourTurn {
                    line,
                    ..
                } = static_ability
                {
                    effective = Some(*line);
                }
            }
        }
        for effect in self.effects.iter().filter(|effect| {
            effect.layer == crate::layer::Layer::Type && effect.target == object.id
        }) {
            match effect.kind {
                crate::layer::ContinuousKind::SetCreatureSubtypes(line) => effective = Some(line),
                crate::layer::ContinuousKind::AddCreatureSubtypes(line) if line.has(subtype) => {
                    added = true;
                }
                _ => {}
            }
        }
        if added {
            return true;
        }
        match effective {
            Some(line) => line.has(subtype),
            None => object.card.def().has_subtype(subtype),
        }
    }

    /// CR 613.1d + CR 110.1: effective creature status as shared by the engine
    /// and the no-abilities predicate. An additive layer 4 type grant animates a
    /// noncreature permanent; ending the effect reverts it.
    pub fn object_is_creature(&self, object: &Object) -> bool {
        object.card.def().is_creature()
            || self.effects.iter().any(|effect| {
                effect.layer == crate::layer::Layer::Type
                    && effect.target == object.id
                    && matches!(
                        effect.kind,
                        crate::layer::ContinuousKind::AddCreatureSubtypes(_)
                    )
            })
    }

    fn validate_affected_player(
        &self,
        card: CardId,
        ability: u8,
        affected_player: Option<PlayerId>,
        label: &str,
    ) -> Result<(), EngineError> {
        if let Some(seat) = affected_player
            && (seat as usize) >= self.seat_count()
        {
            return Err(EngineError::InvalidState {
                reason: format!("{label} affected player P{seat} is out of range"),
            });
        }
        let required = card == CardId::ZoZuThePunisher && ability == 0;
        if required && affected_player.is_none() {
            return Err(EngineError::InvalidState {
                reason: format!("{label} is missing Zo-Zu's captured affected player"),
            });
        }
        if !required && affected_player.is_some() {
            return Err(EngineError::InvalidState {
                reason: format!("{label} carries an affected player without a consumer"),
            });
        }
        Ok(())
    }

    pub fn object_has_no_abilities(&self, id: ObjectId) -> bool {
        let Some(object) = self.objects.get(&id) else {
            return false;
        };
        let card = object.card;
        let def = card.def();
        if !crate::ability::abilities(card).is_empty()
            || !crate::ability::static_abilities(card).is_empty()
            || !crate::tokens::keywords(card).is_empty()
        {
            return false;
        }
        // CR 605.1a: an intrinsic mana ability is an ability even though it is
        // not in the printed rules text (every basic land, Sol Ring).
        if def.basic_mana.is_some()
            || def.artifact_mana.is_some()
            || card.dynamic_mana().is_some()
            || card.sacrifice_mana().is_some()
        {
            return false;
        }
        // Printed keyword abilities this engine executes.
        if def.has_trample()
            || def.has_flying()
            || def.has_flash()
            || def.has_defender()
            || def.has_islandwalk()
        {
            return false;
        }
        // CR 122.1b: a keyword counter grants its keyword for as long as it is
        // on the permanent.
        if object.counters.get(crate::counters::CounterKind::Trample) > 0 {
            return false;
        }
        // Abilities granted by a continuous effect on this exact object:
        // keyword grants and every keyword an Equipment is granting it.
        let granted = self.effects.iter().any(|effect| {
            effect.target == object.id
                && matches!(
                    effect.kind,
                    ContinuousKind::GrantsTrample
                        | ContinuousKind::GrantsHaste
                        | ContinuousKind::Equipped { haste: true, .. }
                        | ContinuousKind::Equipped { shroud: true, .. }
                        | ContinuousKind::Equipped { hexproof: true, .. }
                )
        });
        if granted {
            return false;
        }
        // Abilities granted by another permanent's printed static ("other
        // creatures you control have trample").
        // A static grant to "creatures you control" reaches battlefield
        // permanents only, not creature cards in that player's graveyard.
        let granted_by_static = object.zone == Zone::Battlefield && self.objects.values().any(|source| {
            source.zone == Zone::Battlefield
                && source.id != object.id
                && crate::ability::static_abilities(source.card)
                    .iter()
                    .any(|ability| {
                        matches!(
                            ability,
                            crate::ability::StaticAbility::OtherControlledCreaturesHaveTrample { .. }
                        ) && source.controller == object.controller || matches!(
                            ability,
                            crate::ability::StaticAbility::OtherControlledCreaturesWithAnySubtypeHaveHaste {
                                subtypes,
                                ..
                            } if source.controller == object.controller
                                && self.object_is_creature(object)
                                && subtypes
                                    .iter()
                                    .any(|subtype| self.object_has_subtype(object, subtype))
                        ) || matches!(
                            ability,
                            crate::ability::StaticAbility::ControlledCreaturesWithAnySubtypeHaveHaste {
                                subtypes,
                                ..
                            } if source.controller == object.controller
                                && self.object_is_creature(object)
                                && subtypes
                                    .iter()
                                    .any(|subtype| self.object_has_subtype(object, subtype))
                        ) || matches!(
                            ability,
                            crate::ability::StaticAbility::AllOtherCreaturesWithAnySubtypeHaveMountainwalk {
                                subtypes,
                                ..
                            } if self.object_is_creature(object)
                                && subtypes
                                    .iter()
                                    .any(|subtype| self.object_has_subtype(object, subtype))
                        )
                    })
        });
        !granted_by_static
    }

    /// CR 704.5j + CR 701.54c: a permanent is legendary when its printed type
    /// line says so, or when it is its controller's Ring-bearer at Ring level 1+.
    /// One predicate so the SBA, the loader and the direct-answer guard agree.
    pub(crate) fn is_legendary_effective(&self, object: &Object) -> bool {
        object.card.def().type_line.contains("Legendary")
            || (object.zone == Zone::Battlefield
                && self
                    .rings
                    .get(object.controller as usize)
                    .is_some_and(|ring| ring.temptations >= 1 && ring.bearer == Some(object.id)))
    }

    pub fn battlefield(&self, p: PlayerId) -> Vec<&Object> {
        self.objects
            .values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == p)
            .collect()
    }

    pub fn battlefield_creatures(&self, p: PlayerId) -> Vec<&Object> {
        self.battlefield(p)
            .into_iter()
            .filter(|o| o.is_creature())
            .collect()
    }

    pub fn find_permanent(&self, p: PlayerId, card: CardId) -> Option<&Object> {
        self.objects
            .values()
            .find(|o| o.zone == Zone::Battlefield && o.controller == p && o.card == card)
    }

    pub fn hand_cards(&self, p: PlayerId) -> Vec<&Object> {
        self.player(p)
            .hand
            .iter()
            .filter_map(|id| self.try_object(*id))
            .collect()
    }

    pub fn is_over(&self) -> bool {
        self.outcome.is_some()
    }

    /// The player who must act now, if any. An eliminated seat never owns a
    /// decision, even while its turn continues (CR 800.4j).
    pub fn decision_owner(&self) -> Option<PlayerId> {
        if self.outcome.is_some() {
            return None;
        }
        if let Some(pending) = &self.pending {
            let owner = pending.player();
            return self.is_live(owner).then_some(owner);
        }
        self.turn.priority.filter(|p| self.is_live(*p))
    }
}

/// Serde helper: skip a zero `u32` so state written before the effect pipeline
/// keeps its original canonical JSON (and therefore its recorded digest).
fn is_default_search_mode(mode: &LibrarySearchMode) -> bool {
    *mode == LibrarySearchMode::BasicLandToBattlefield
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

/// Serde helper for `next_timestamp`, same reason as [`is_zero_u32`].
fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}
