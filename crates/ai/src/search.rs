//! Budgeted information-set search for practice play.
//!
//! The four guarantees, and how each is enforced:
//!
//! 1. **Engine authority.** Legality and state transitions stay in the
//!    deterministic engine. The search only submits actions that appear in the
//!    decision owner's `PlayerView.legal` offer, and the engine validates every
//!    submission before anything mutates. Nothing here re-implements a rule.
//! 2. **No hidden reads.** The search never reads a real opponent hand or the
//!    real future library order. The search world is *reconstructed* from the
//!    masked observation alone: public object ids straight from the view, plus
//!    a disclosed neutral belief deck (basic land alternating with a fixed
//!    vanilla creature) for every card that is genuinely hidden. A runtime
//!    visibility ledger bounds every object entry the reconstruction copies to
//!    ids that appeared in the view. The viewer's declared deck list —
//!    legitimately owned information — can refine the viewer's own future
//!    draws; when absent the neutral belief deck is used everywhere and the
//!    typed fidelity line says so.
//! 3. **Budgets, cancellation, progressive analysis, fallback.** A decision
//!    budget (engine decisions across rollouts), a wall-clock bound and a
//!    `SearchCancellation` flag bound the work; the running candidate scores
//!    update after each rollout round, so results arrive progressively. When
//!    search is unavailable for a decision point (non-priority pending
//!    decisions, mid-resolution or queued-trigger states, reconstruction
//!    failure) or no candidate could be scored at all, the caller falls back to
//!    [`fallback`]: the frozen heuristic's choice, labelled as a fallback with
//!    the concrete reason. It is never a silent default.
//! 4. **Honest comparison.** Rollout scores are practice outcomes only: win
//!    +1, loss −1, draw 0 as a scored sample; a capped or failed rollout is a
//!    separately counted "no result", never a win or loss. This module never
//!    reports "larger search is stronger play"; the small paired trial against
//!    the frozen heuristic lives in `tests/search.rs` and reports its
//!    limitations in plain text.
//!
//! Rules revision: Comprehensive Rules effective 2026-09-25; the engine's own
//! validator covers everything this module submits.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use aperture_core::counters::Counters;
use aperture_core::rng::Pcg32;
use aperture_core::state::{GameState, Object, Player, Turn};
use aperture_core::view::{CardView, OpponentView, PendingKind, PlayerView};
use aperture_core::{Action, CardId, DeckList, Event, Game, ManaColor, ObjectId, PlayerId, Zone};

use crate::heuristic::HeuristicPolicy;
use crate::{Decision, DecisionSource, Policy};

/// The result of one rollout.
#[derive(Debug, Clone, PartialEq)]
enum Rollout {
    /// Reached an engine result. `value`: viewer win +1 / loss −1 / draw 0;
    /// `was_result` is false only for outcomes this rollout did not earn.
    Scored { value: f32, was_result: bool },
    /// Ended without a result (cap/truncation/failure).
    Neutral { times: u32 },
    /// The deterministic search world refused the root action.
    RootRefused { detail: String },
}

/// Fixed seed salt so a given seed index always rebuilds the same world:
/// same observation + same seeds = the same report, reproducibly.
const SEED_SALT: u64 = 0x6172_0721_F0CA_8B30;
const SEED_STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;

/// Determinization seed for rollout round `index`.
pub fn determinization_seed(index: u64) -> u64 {
    SEED_SALT ^ (index + 1).wrapping_mul(SEED_STRIDE)
}

/// A cooperative cancellation flag shared by driver and search.
#[derive(Debug, Clone, Default)]
pub struct SearchCancellation(pub Arc<AtomicBool>);

impl SearchCancellation {
    pub fn new() -> Self {
        SearchCancellation(Arc::new(AtomicBool::new(false)))
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// How much work one search decision may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchConfig {
    /// Number of determinization seeds, in evaluation order.
    pub seeds: usize,
    /// Total engine decision points the search may consume across all rollouts.
    pub node_budget: usize,
    /// Per-rollout decision cap. A rollout that hits it contributes a neutral,
    /// separately counted outcome — never a win or a loss.
    pub max_rollout_nodes: usize,
    /// Wall-clock bound for the whole decision, in milliseconds.
    pub max_millis: u64,
    /// Maximum root candidates (engine-offered actions, in offer order).
    pub max_candidates: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            seeds: 4,
            node_budget: 24_000,
            max_rollout_nodes: 1_500,
            max_millis: 5_000,
            max_candidates: 12,
        }
    }
}

/// The viewer's declared deck list. Only used for the viewer's own future
/// draws (legitimately owned information). `None` downgrades own-draw fidelity
/// to the neutral belief deck; the analysis' fidelity line reports it.
#[derive(Debug, Clone, Default)]
pub struct Knowledge {
    pub own_deck: Option<DeckList>,
}

/// Why search is unavailable for one decision point. Every reason is typed and
/// public; the caller surfaces it wherever the fallback is used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchUnavailable {
    /// The decision is a pending choice (mulligan, combat declaration,
    /// discard, Crack queue, commander choice, trigger target, "may" choice).
    /// Those subtrees need engine-side support this slice does not provide.
    PendingDecision { kind: String },
    /// A spell or ability is resolving, or triggered abilities are queued.
    StackBusy,
    /// The masked observation and the engine state disagree on public data.
    Inconsistent { detail: String },
    /// The engine's own state validator rejected the reconstruction.
    ReconstructionRejected { detail: String },
}

impl SearchUnavailable {
    pub fn label(&self) -> String {
        match self {
            SearchUnavailable::PendingDecision { kind } => {
                format!("non-priority decision (pending kind {kind}); search not enabled there")
            }
            SearchUnavailable::StackBusy => {
                "a stack item is resolving or triggers are queued; search not enabled there"
                    .to_string()
            }
            SearchUnavailable::Inconsistent { detail } => {
                format!("masked observation inconsistent: {detail}")
            }
            SearchUnavailable::ReconstructionRejected { detail } => {
                format!("engine rejected reconstruction: {detail}")
            }
        }
    }
}

/// One root candidate's running evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateScore {
    /// The engine's own label for the offered action.
    pub label: String,
    pub action: Action,
    /// Mean practice outcome over scored rollouts: win +1, loss −1, draw 0.
    /// `None` until a rollout reached an engine result.
    pub mean: Option<f32>,
    pub scored_samples: usize,
    /// Rollouts that ended without a result (cap/truncation/failure).
    pub rollouts_without_result: usize,
    /// How often the search world refused this root action as illegal.
    pub refused_times: usize,
}

/// A progressive snapshot: valid at any time, including after cancellation.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchAnalysis {
    /// Candidates in root offer order, updated after each rollout round.
    pub candidates: Vec<CandidateScore>,
    /// Engine decision points consumed so far.
    pub decisions_used: usize,
    pub cancelled: bool,
    /// Determinization fidelity disclosure.
    pub fidelity: String,
}

/// The durable outcome of one full search decision.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchOutcome {
    pub decision: Decision,
    pub analysis: SearchAnalysis,
}

/// Run one budgeted information-set search from the viewer's masked
/// observation; the real game stays the engine authority. Returns the search
/// decision plus the final analysis. Returns `Err(SearchUnavailable)` when
/// search cannot run for this decision point at all — the caller then uses
/// [`fallback`] and surfaces the typed reason.
pub fn search(
    real: &Game,
    view: &PlayerView,
    knowledge: &Knowledge,
    config: &SearchConfig,
    cancel: &SearchCancellation,
) -> Result<SearchOutcome, SearchUnavailable> {
    // 1. Decision-point gate: ordinary priority decisions only.
    if let Some(kind) = view.pending_kind {
        return Err(SearchUnavailable::PendingDecision {
            kind: pending_kind_name(kind),
        });
    }
    if !view.stack.is_empty() {
        return Err(SearchUnavailable::StackBusy);
    }

    // 2. Deterministic candidate order from the engine's own offer. Policies
    //    select only offered actions; every candidate is a `view.legal` entry.
    let mut candidates: Vec<(Action, CandidateScore)> = Vec::new();
    for legal in &view.legal {
        if candidates.len() >= config.max_candidates {
            break;
        }
        let action = legal.action.clone();
        if candidates.iter().any(|(seen, _)| *seen == action) {
            continue;
        }
        candidates.push((
            action.clone(),
            CandidateScore {
                label: legal.label.clone(),
                action,
                mean: None,
                scored_samples: 0,
                rollouts_without_result: 0,
                refused_times: 0,
            },
        ));
    }
    if candidates.is_empty() {
        return Err(SearchUnavailable::Inconsistent {
            detail: "the engine offered no legal action at this priority decision".to_string(),
        });
    }

    let ledger = visible_ids(view);
    let filler = BeliefFiller::from_view(view);
    let own_draws = own_remaining_draws(knowledge, view);
    let started = Instant::now();
    let mut decisions_used = 0usize;

    // 3. Rollout rounds: each candidate gets one rollout per determinization
    //    seed before any candidate gets a second, so a mid-run snapshot — and
    //    a cancellation — stays fair across candidates.
    'rounds: for round in 0..config.seeds {
        let seed = determinization_seed(round as u64);
        for (_, score) in candidates.iter_mut() {
            if cancel.is_cancelled()
                || started.elapsed() >= Duration::from_millis(config.max_millis)
                || decisions_used >= config.node_budget
            {
                break 'rounds;
            }
            let state = reconstruct(real, view, seed, &ledger, &filler, &own_draws)?;
            let mut world = Game::from_state(state).map_err(|err| {
                SearchUnavailable::ReconstructionRejected {
                    detail: err.to_string(),
                }
            })?;
            match run_rollout(
                &mut world,
                &RolloutContext {
                    viewer: view.viewer,
                    root: &score.action,
                    max_rollout_nodes: config.max_rollout_nodes,
                    cancel,
                    max_millis: config.max_millis,
                    started: &started,
                },
                &mut decisions_used,
            ) {
                Rollout::Scored { value, was_result } => {
                    if was_result {
                        let previous = score.mean.unwrap_or(0.0) * score.scored_samples as f32;
                        score.scored_samples += 1;
                        score.mean = Some((previous + value) / score.scored_samples as f32);
                    } else {
                        score.rollouts_without_result += 1;
                    }
                }
                Rollout::Neutral { times } => {
                    score.rollouts_without_result += times as usize;
                }
                Rollout::RootRefused { .. } => {
                    score.refused_times += 1;
                }
            }
        }
    }

    let scores = candidates
        .into_iter()
        .map(|(_, score)| score)
        .collect::<Vec<_>>();

    // 4. Best scored candidate, or the deterministic legal fallback.
    let mut analysis = SearchAnalysis {
        candidates: scores.clone(),
        decisions_used,
        cancelled: cancel.is_cancelled(),
        fidelity: fidelity_line(knowledge, view),
    };
    let decision = match best_candidate(&scores).map(|c| c.action.clone()) {
        Some(action) => Decision {
            action,
            source: DecisionSource::Search,
            note: None,
            preference: None,
        },
        None => {
            // Nothing was scored (budget exhausted before any result, or every
            // candidate refused/capped): the deterministic legal fallback
            // decides, and the analysis says so.
            analysis.fidelity = format!(
                "{} — no candidate reached a result; deterministic legal fallback used",
                analysis.fidelity
            );
            fallback(view)
        }
    };
    Ok(SearchOutcome { decision, analysis })
}

/// The deterministic legal fallback: the frozen heuristic's own decision. It
/// sees only the masked view and returns only offered actions; the caller still
/// submits it through the engine like any other decision.
pub fn fallback(view: &PlayerView) -> Decision {
    HeuristicPolicy::new().decide(view)
}

fn pending_kind_name(kind: PendingKind) -> String {
    match kind {
        PendingKind::Mulligan => "mulligan".to_string(),
        PendingKind::BottomCards { count } => format!("bottom cards (count {count})"),
        PendingKind::DeclareAttackers => "declare attackers".to_string(),
        PendingKind::DeclareBlockers => "declare blockers".to_string(),
        PendingKind::OrderCard { index, .. } => format!("order card (index {index})"),
        PendingKind::Discard { count } => format!("discard (count {count})"),
        PendingKind::ApnapSacrifice => "Crack the Earth APNAP queue".to_string(),
        PendingKind::ApnapReveal => "APNAP reveal queue".to_string(),
        PendingKind::CommanderReturn => "commander return".to_string(),
        PendingKind::CommanderReplacement => "commander replacement".to_string(),
        PendingKind::CommanderDraw => "commander draw replacement".to_string(),
        PendingKind::TriggerTarget => "trigger target".to_string(),
        PendingKind::TriggerChoice => "trigger choice".to_string(),
    }
}

/// Deterministic best: highest mean, then most scored samples, then earliest
/// offer order (stable max over the offer-ordered vec).
fn best_candidate(scores: &[CandidateScore]) -> Option<&CandidateScore> {
    scores.iter().filter(|c| c.mean.is_some()).max_by(|a, b| {
        a.mean
            .partial_cmp(&b.mean)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.scored_samples.cmp(&b.scored_samples))
    })
}

fn fidelity_line(knowledge: &Knowledge, view: &PlayerView) -> String {
    let own = knowledge
        .own_deck
        .as_ref()
        .map(|deck| {
            format!(
                "own future draws sampled from declared deck \"{}\" minus visible cards",
                deck.name
            )
        })
        .unwrap_or_else(|| "own future draws from the neutral belief deck".to_string());
    format!(
        "opponents: neutral belief deck ({} opponent(s)); no real hand or library order read; {own}",
        view.opponents.len()
    )
}

/// The viewer's remaining deck composition (declared list minus the cards the
/// viewer can already see across their own zones), used only for the viewer's
/// own future draws. `None` when no list is declared.
fn own_remaining_draws(knowledge: &Knowledge, view: &PlayerView) -> Option<Vec<CardId>> {
    let deck = knowledge.own_deck.as_ref()?;
    let mut remaining: Vec<CardId> = deck.expand();
    for card in view.own_hand.iter().chain(&view.own_battlefield) {
        remove_one(&mut remaining, card.card);
    }
    for card in view.own_graveyard.iter().chain(&view.own_exile) {
        remove_one(&mut remaining, card.card);
    }
    for card in &view.own_command_zone {
        remove_one(&mut remaining, card.card);
    }
    // Deterministic shuffle: a reproducible sample of the true remaining
    // composition. The real future order is unknown and is never read.
    let mut rng = Pcg32::new(SEED_SALT ^ SEED_STRIDE);
    if remaining.len() > 1 {
        for i in (1..remaining.len()).rev() {
            let j = rng.below((i + 1) as u32) as usize;
            remaining.swap(i, j);
        }
    }
    Some(remaining)
}

fn remove_one(pool: &mut Vec<CardId>, card: CardId) {
    if let Some(pos) = pool.iter().position(|c| *c == card) {
        pool.remove(pos);
    }
}

/// Every object id the masked view exposes. The reconstruction may copy object
/// entries ONLY for ids in this ledger; anything else is hidden state this
/// module never touches.
fn visible_ids(view: &PlayerView) -> Vec<ObjectId> {
    let mut ids: Vec<ObjectId> = Vec::new();
    fn push(ids: &mut Vec<ObjectId>, card: &CardView) {
        if !ids.contains(&card.id) {
            ids.push(card.id);
        }
    }
    for card in &view.own_hand {
        push(&mut ids, card);
    }
    for card in &view.own_battlefield {
        push(&mut ids, card);
    }
    for card in &view.own_graveyard {
        push(&mut ids, card);
    }
    for card in &view.own_exile {
        push(&mut ids, card);
    }
    for card in &view.own_command_zone {
        push(&mut ids, card);
    }
    for opponent in &view.opponents {
        for card in &opponent.battlefield {
            push(&mut ids, card);
        }
        for card in &opponent.graveyard {
            push(&mut ids, card);
        }
        for card in &opponent.exile {
            push(&mut ids, card);
        }
        for card in &opponent.command_zone {
            push(&mut ids, card);
        }
    }
    ids
}

/// The disclosed neutral belief deck: alternating basic land (of a color the
/// opponent shows on the battlefield; forest when none shows) and a fixed
/// vanilla creature. Deliberately bland and deterministic, so rollouts measure
/// the decision — not a guessed novel deck.
struct BeliefFiller {
    land: CardId,
    creature: CardId,
}

impl BeliefFiller {
    fn from_view(view: &PlayerView) -> Self {
        let mut color = ManaColor::Green;
        for opponent in &view.opponents {
            for card in &opponent.battlefield {
                if let Some(shown) = card.card.def().basic_mana {
                    color = shown;
                }
            }
        }
        let land = aperture_core::catalog::all()
            .iter()
            .find(|def| def.basic_mana == Some(color))
            .map(|def| def.id)
            .unwrap_or(CardId::Forest);
        BeliefFiller {
            land,
            creature: CardId::GrizzlyBears,
        }
    }

    fn card(&self, index: usize) -> CardId {
        if index.is_multiple_of(2) {
            self.land
        } else {
            self.creature
        }
    }
}

/// Rebuild a search world state from the masked view alone.
///
/// **Permitted reads of the real state, bounded by the ledger:** object entries
/// only for object ids that appear in the view (public zones plus the viewer's
/// own hand), plus seat-agnostic bookkeeping that no seat learns anything
/// private from (next-object/effect/timestamp counters, activation-use
/// counts, seat names/deck labels). Opponent hands and library membership are
/// never read; they are re-dealt from the belief deck with the real counts.
/// No seat's real library order is ever used.
///
/// Typed counters (CR 122) travel with the copied public objects, so a
/// permanent's counters and its counter-adjusted characteristics survive into
/// the world; fabricated belief objects carry none. Cards revealed from a
/// hidden zone are deliberately NOT modelled: a reveal stays inside a zone list
/// this function rebuilds (re-dealt belief entries for opponents, real ids for
/// the viewer's own hand or a supplied known library), so the world has no
/// place to put the id and reports no reveals at all.
fn reconstruct(
    real: &Game,
    view: &PlayerView,
    seed: u64,
    ledger: &[ObjectId],
    filler: &BeliefFiller,
    own_draws: &Option<Vec<CardId>>,
) -> Result<GameState, SearchUnavailable> {
    let real_state = &real.state;
    if !real_state.trigger_queue.is_empty() || real_state.pending.is_some() {
        return Err(SearchUnavailable::StackBusy);
    }
    let mut rng = Pcg32::new(seed);
    let mut next_object = real_state.next_object;
    let mut objects: BTreeMap<ObjectId, Object> = BTreeMap::new();

    // Public and own objects, copied by id. Hidden objects refuse to be
    // touched; a ledger id missing from the state is a typed inconsistency.
    for id in ledger {
        let object = real_state
            .objects
            .get(id)
            .ok_or_else(|| SearchUnavailable::Inconsistent {
                detail: format!("view-visible object {id} is missing from the engine state"),
            })?;
        objects.insert(object.id, object.clone());
    }

    let belief_object = |id: ObjectId, card: CardId, seat: PlayerId, zone: Zone| Object {
        id,
        card,
        owner: seat,
        controller: seat,
        zone,
        commander: None,
        tapped: false,
        damage: 0,
        summoning_sick: false,
        pump_power: 0,
        pump_toughness: 0,
        plus_one_counters: 0,
        // A fabricated belief object is unknown by construction: no counter
        // information is invented for it (CR 122 counters are real state).
        counters: Counters::new(),
        // A fabricated belief object is likewise never paired (CR 702.94a
        // pairing is real state, and no hidden-information guess invents one).
        paired_with: None,
    };

    let mut players = Vec::with_capacity(real_state.players.len());
    for (index, seat) in real_state.players.iter().enumerate() {
        let seat_id = index as PlayerId;
        let is_viewer = seat_id == view.viewer;
        let opponent = if is_viewer {
            None
        } else {
            view.opponents.iter().find(|o| o.seat == seat_id)
        };

        // Public zone membership straight from the view.
        let mut graveyard = Vec::new();
        let mut exile = Vec::new();
        let mut command_zone = Vec::new();
        if is_viewer {
            for card in &view.own_graveyard {
                graveyard.push(card.id);
            }
            for card in &view.own_exile {
                exile.push(card.id);
            }
            for card in &view.own_command_zone {
                command_zone.push(card.id);
            }
        } else if let Some(opponent) = opponent {
            for card in &opponent.graveyard {
                graveyard.push(card.id);
            }
            for card in &opponent.exile {
                exile.push(card.id);
            }
            for card in &opponent.command_zone {
                command_zone.push(card.id);
            }
        }

        // Hidden zones: beliefs with the real counts. The viewer's hand keeps
        // its real ids (legitimately visible); opponents get fresh belief ids.
        let mut hand = Vec::new();
        let library_count;
        let mut own_queue = own_draws.clone().unwrap_or_default();
        if is_viewer {
            for card in &view.own_hand {
                hand.push(card.id);
            }
            library_count = view.own_library_count;
        } else {
            let opponent: &OpponentView =
                opponent.ok_or_else(|| SearchUnavailable::Inconsistent {
                    detail: format!(
                        "seat P{seat_id} is neither the viewer nor an opponent in view"
                    ),
                })?;
            library_count = opponent.library_count;
            for i in 0..opponent.hand_count {
                let id = ObjectId(next_object);
                next_object += 1;
                objects.insert(id, belief_object(id, filler.card(i), seat_id, Zone::Hand));
                hand.push(id);
            }
        }

        let mut library = Vec::with_capacity(library_count);
        for i in 0..library_count {
            let id = ObjectId(next_object);
            next_object += 1;
            let card = if is_viewer && !own_queue.is_empty() {
                own_queue.remove(0)
            } else {
                filler.card(i)
            };
            objects.insert(id, belief_object(id, card, seat_id, Zone::Library));
            library.push(id);
        }
        // Deterministic Fisher–Yates over the belief library: reproducible,
        // and deliberately NOT the real library order.
        if library.len() > 1 {
            for i in (1..library.len()).rev() {
                let j = rng.below((i + 1) as u32) as usize;
                library.swap(i, j);
            }
        }

        players.push(Player {
            id: seat.id,
            name: seat.name.clone(),
            deck_name: seat.deck_name.clone(),
            library,
            hand,
            graveyard,
            exile,
            command_zone,
            life: if is_viewer {
                view.own_life
            } else {
                opponent.map(|o| o.life).unwrap_or_else(|| seat.life)
            },
            mana: if is_viewer {
                view.own_mana_pool
            } else {
                Default::default()
            },
            mulligans: if is_viewer { view.own_mulligans } else { 0 },
            drew_from_empty: seat.drew_from_empty,
            lost: seat.lost,
        });
    }

    // Turn, copied from the masked view's public parts; priority is placed on
    // the viewer so the search world has the SAME decision point reality has.
    // The engine drives everything after that.
    let turn = Turn {
        number: view.turn_number,
        active: view.active,
        step: view.step,
        priority: Some(view.viewer),
        passed: 0,
        lands_played: real_state.turn.lands_played,
        attackers: view.attackers.clone(),
        blocked: real_state.turn.blocked.clone(),
        blockers: view.blockers.clone(),
        // Public combat facts, like `blocked`/`defender` above: the attack and
        // block declarations were announced to every seat, and CR 506.5's
        // "alone" reading is about those declarations, so the search world may
        // take them from the real state without leaking anything hidden.
        declared_attackers: real_state.turn.declared_attackers.clone(),
        declared_blockers: real_state.turn.declared_blockers.clone(),
        combat_damage_done: real_state.turn.combat_damage_done,
        defender: real_state.turn.defender,
    };

    Ok(GameState {
        seed,
        first_player: real_state.first_player,
        rng,
        objects,
        creature_died_this_turn: real_state.creature_died_this_turn,
        // Forced initializer for the field slice 35 added to `GameState`; the
        // search projection copies the same value so its state stays truthful.
        draw_replacements: real_state.draw_replacements.clone(),
        nontoken_creature_deaths_this_turn: real_state.nontoken_creature_deaths_this_turn.clone(),
        commanders: real_state.commanders.clone(),
        profile: real_state.profile,
        commander_damage: real_state.commander_damage.clone(),
        retired_commanders: real_state.retired_commanders.clone(),
        prevent_all_combat_damage_turn: real_state.prevent_all_combat_damage_turn,
        // Deliberate simplification: a card revealed from a hidden zone is
        // public information, but its id stays inside a zone list this
        // reconstruction rebuilds — re-dealt belief entries for the opponent,
        // real ids only for the viewer's own hand or a supplied known library.
        // The ledger therefore never copies a revealed object, and naming an id
        // the world does not place would leave a state that fails the engine's
        // zone-container invariant (and is currently masked by the
        // `revealed_hidden` projection). The world models no reveals; nothing
        // hidden is read to build this field.
        revealed: Vec::new(),
        players,
        stack: Vec::new(),
        turn,
        pending: None,
        mulligan: None,
        resolving: None,
        next_object,
        trigger_queue: Vec::new(),
        next_trigger_seq: 0,
        activation_uses: real_state.activation_uses.clone(),
        skip_next_untap: real_state.skip_next_untap.clone(),
        effects: Vec::new(),
        shields: Vec::new(),
        regeneration_shields: Vec::new(),
        next_effect: real_state.next_effect,
        next_timestamp: real_state.next_timestamp,
        revision: 0,
        decision: 0,
        outcome: None,
        rules_revision: real_state.rules_revision.clone(),
    })
}

/// Context carried through a rollout: budgets plus the root action.
struct RolloutContext<'a> {
    viewer: PlayerId,
    root: &'a Action,
    max_rollout_nodes: usize,
    cancel: &'a SearchCancellation,
    max_millis: u64,
    started: &'a Instant,
}

/// Drive one rollout in the search world: submit the root action first, then
/// let the frozen heuristic play every seat (the rollout opponent is a model,
/// not an oracle on hidden cards). Applies cancellation, wall-clock and node
/// budgets at every engine decision point.
fn run_rollout(world: &mut Game, ctx: &RolloutContext<'_>, decisions_used: &mut usize) -> Rollout {
    let RolloutContext {
        viewer,
        root,
        max_rollout_nodes,
        cancel,
        max_millis,
        started,
    } = *ctx;
    let mut events: Vec<Event> = Vec::new();
    let mut nodes = 0usize;
    let mut first = true;

    while !world.state.is_over() {
        if cancel.is_cancelled()
            || (max_millis != u64::MAX && started.elapsed() >= Duration::from_millis(max_millis))
        {
            return Rollout::Neutral { times: 1 };
        }
        if nodes >= max_rollout_nodes {
            return Rollout::Neutral { times: 1 };
        }
        let Some(owner) = world.decision_owner() else {
            return if first {
                Rollout::RootRefused {
                    detail: "the search world has no decision point".to_string(),
                }
            } else {
                Rollout::Neutral { times: 1 }
            };
        };
        let view = world.view_for(owner, &events);
        let offered: Option<Action> = if first {
            // Policies select only offered actions, so the root must be in
            // the search world's own engine-generated offer to be attempted.
            view.legal
                .iter()
                .find(|legal| legal.action == *root)
                .map(|legal| legal.action.clone())
        } else {
            Some(HeuristicPolicy::new().decide(&view).action)
        };
        let Some(action) = offered else {
            return if first {
                Rollout::RootRefused {
                    detail: "root action was not in the search world's offer".to_string(),
                }
            } else {
                Rollout::Neutral { times: 1 }
            };
        };
        match world.submit_action(owner, action) {
            Ok(new_events) => {
                events.extend(new_events);
                nodes += 1;
                *decisions_used += 1;
                first = false;
            }
            Err(err) if first => {
                return Rollout::RootRefused {
                    detail: err.to_string(),
                };
            }
            Err(_) => {
                // A rollout-internal heuristic disagreement: end without a
                // result rather than fabricating an outcome.
                return Rollout::Neutral { times: 1 };
            }
        }
    }

    match &world.state.outcome {
        Some(aperture_core::state::Outcome::Win { winner, .. }) => Rollout::Scored {
            value: if *winner == viewer { 1.0 } else { -1.0 },
            was_result: true,
        },
        Some(aperture_core::state::Outcome::Draw { .. }) => Rollout::Scored {
            value: 0.0,
            was_result: true,
        },
        None => Rollout::Neutral { times: 1 },
    }
}
