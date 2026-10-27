//! Executable rules-conformance fixtures (schema `aperture.conformance/1`).
//!
//! Each JSON case states a symbolic initial position, the actions a seat
//! submits, the decisions/events the engine must produce, and the resulting
//! state. Cases carry the Comprehensive Rules revision, the SHA-256 of the
//! pinned official text, and the exact rule references they exercise; see
//! `docs/rules-reference.md` for provenance and the format specification.
//!
//! These fixtures are two-seat practice-slice evidence, not a claim of full
//! Magic coverage. The multi-blocker case is marked as the engine's bounded
//! lethal-first policy (CR 510.1c lets the attacker choose), so a passing
//! fixture never launders a simplification into "official rules".

mod common;

use std::collections::{BTreeMap, BTreeSet};

use aperture_core::state::STEP_ORDER;
use aperture_core::*;
use common::{fixture, object_mut};
use serde::Deserialize;

const MANIFEST: &str = include_str!("fixtures/rules/manifest.json");
const TIMING: &str = include_str!("fixtures/rules/timing-priority.json");
const SBA: &str = include_str!("fixtures/rules/state-based-actions.json");
const CLEANUP: &str = include_str!("fixtures/rules/cleanup.json");
const MULTI_BLOCKER: &str = include_str!("fixtures/rules/multi-blocker-damage.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    rules_revision: String,
    source_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseFile {
    schema: String,
    rules_revision: String,
    source_sha256: String,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    description: String,
    rule_refs: Vec<String>,
    #[serde(default)]
    bounded_policy: Option<String>,
    initial: Initial,
    steps: Vec<StepExpectation>,
    #[serde(default)]
    expect: FinalExpect,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Initial {
    seed: u64,
    #[serde(default = "one")]
    turn: u32,
    active: String,
    step: String,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    life: BTreeMap<String, i32>,
    #[serde(default)]
    zones: BTreeMap<String, ZoneContents>,
    /// Battlefield-only overlays such as marked damage or a pump effect.
    #[serde(default)]
    permanents: BTreeMap<String, PermanentState>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ZoneContents {
    #[serde(default)]
    hand: Vec<String>,
    #[serde(default)]
    battlefield: Vec<String>,
    #[serde(default)]
    library: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FinalZones {
    #[serde(default)]
    hand: Option<Vec<String>>,
    #[serde(default)]
    battlefield: Option<Vec<String>>,
    #[serde(default)]
    library: Option<Vec<String>>,
    #[serde(default)]
    graveyard: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PermanentState {
    #[serde(default)]
    damage: Option<u32>,
    #[serde(default)]
    pump_power: Option<i32>,
    #[serde(default)]
    pump_toughness: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StepExpectation {
    #[serde(default)]
    player: Option<String>,
    action: ActionSpec,
    /// `accepted` (default) or `rejected`.
    #[serde(default)]
    expect: Option<String>,
    /// Structured rejection contract; required when `expect` is `rejected`.
    #[serde(default)]
    error: Option<ErrorSpec>,
    /// Expected events, in order. Other engine events may appear between them.
    #[serde(default)]
    events: Vec<EventSpec>,
    #[serde(default)]
    expect_pending: Option<String>,
    #[serde(default)]
    expect_priority: Option<String>,
    #[serde(default)]
    expect_step: Option<String>,
    #[serde(default)]
    expect_turn: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ActionSpec {
    Pass,
    PlayLand {
        card: String,
    },
    CastSpell {
        card: String,
    },
    ActivateManaAbility {
        land: String,
    },
    DeclareAttackers {
        attackers: Vec<String>,
        /// Required when more than one opponent is live; omitted in the
        /// two-seat fixtures, where the only live opponent is unambiguous.
        #[serde(default)]
        defender: Option<String>,
    },
    DeclareBlockers {
        blocks: Vec<BlockSpec>,
    },
    Discard {
        cards: Vec<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockSpec {
    blocker: String,
    attacker: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ErrorSpec {
    /// `EngineError::BadTiming`: a spell rejected for timing.
    BadTiming { name: String },
    /// `EngineError::Illegal`: a typed illegal action rejection.
    Illegal { action: String },
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventSpec {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    player: Option<String>,
    #[serde(default)]
    card: Option<String>,
    #[serde(default)]
    attacker: Option<String>,
    #[serde(default)]
    blocker: Option<String>,
    #[serde(default)]
    amount: Option<u32>,
    #[serde(default)]
    step: Option<String>,
    #[serde(default)]
    turn: Option<u32>,
    #[serde(default)]
    active: Option<String>,
    #[serde(default)]
    winner: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FinalExpect {
    #[serde(default)]
    life: BTreeMap<String, i32>,
    #[serde(default)]
    turn: Option<u32>,
    #[serde(default)]
    step: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    pending: Option<String>,
    #[serde(default)]
    zones: BTreeMap<String, FinalZones>,
    #[serde(default)]
    battlefield_state: BTreeMap<String, PermanentState>,
    #[serde(default)]
    outcome: Option<OutcomeExpect>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutcomeExpect {
    winner: String,
}

#[test]
fn conformance_manifest_pins_the_engine_rules_revision() {
    let manifest: Manifest = serde_json::from_str(MANIFEST).expect("manifest parses");
    assert_eq!(manifest.schema, "aperture.conformance/1");
    assert_eq!(
        manifest.rules_revision, RULES_REVISION,
        "conformance evidence and the engine must cite the same revision"
    );
    assert_eq!(
        manifest.source_sha256.len(),
        64,
        "SHA-256 is a 32-byte hex digest"
    );
}

#[test]
fn timing_and_priority_cases() {
    run_cases(TIMING);
}

#[test]
fn state_based_action_cases() {
    run_cases(SBA);
}

#[test]
fn cleanup_cases() {
    run_cases(CLEANUP);
}

#[test]
fn multi_blocker_damage_cases() {
    run_cases(MULTI_BLOCKER);
}

fn run_cases(json: &str) {
    let manifest: Manifest = serde_json::from_str(MANIFEST).expect("manifest parses");
    let file: CaseFile = serde_json::from_str(json).expect("case file parses");
    assert_eq!(file.schema, "aperture.conformance/1");
    assert_eq!(
        file.rules_revision, manifest.rules_revision,
        "fixtures must not silently target a different revision"
    );
    assert_eq!(
        file.source_sha256, manifest.source_sha256,
        "fixtures must name the same pinned official text as the manifest"
    );
    assert!(!file.cases.is_empty(), "a case file must contain cases");
    for case in &file.cases {
        run_case(case);
    }
}

fn run_case(case: &Case) {
    assert!(
        !case.description.is_empty(),
        "{}: every fixture must describe the behavior it pins",
        case.id
    );
    if let Some(policy) = &case.bounded_policy {
        assert!(
            !policy.is_empty(),
            "{}: a bounded-policy note must state the divergence",
            case.id
        );
    }
    assert!(
        !case.rule_refs.is_empty(),
        "{}: every fixture must cite the rules it exercises",
        case.id
    );
    let mut game = build_initial(&case.initial, &case.id);
    for (index, step) in case.steps.iter().enumerate() {
        let context = format!("{} step {}", case.id, index + 1);
        let actor = match &step.player {
            Some(name) => parse_player(name, &context),
            None => game
                .decision_owner()
                .unwrap_or_else(|| panic!("{context}: no player has a decision")),
        };
        let action = resolve_action(&game, actor, &step.action, &context);
        let before_revision = game.revision();
        let result = game.submit_action(actor, action);
        let expect = step.expect.as_deref().unwrap_or("accepted");
        match expect {
            "accepted" => {
                let events = result.unwrap_or_else(|error| {
                    panic!("{context}: expected acceptance, got rejection: {error}")
                });
                assert_events(&step.events, &events, &context);
            }
            "rejected" => match result {
                Ok(_) => panic!("{context}: expected rejection, action was accepted"),
                Err(error) => assert_error(step, &error, &context),
            },
            other => panic!("{context}: unknown expectation {other:?}"),
        }
        if expect == "rejected" {
            assert_eq!(
                game.revision(),
                before_revision,
                "{context}: a rejected action must not mutate the state"
            );
        }
        if let Some(pending) = &step.expect_pending {
            assert_eq!(
                pending_name(game.state.pending.as_ref()),
                pending.as_str(),
                "{context}: unexpected pending decision"
            );
        }
        if let Some(priority) = &step.expect_priority {
            assert_eq!(
                priority_name(game.state.turn.priority),
                priority.as_str(),
                "{context}: unexpected priority holder"
            );
        }
        if let Some(step_name) = &step.expect_step {
            assert_eq!(
                step_name_of(game.state.turn.step),
                normalize(step_name),
                "{context}: unexpected step"
            );
        }
        if let Some(turn) = step.expect_turn {
            assert_eq!(game.state.turn.number, turn, "{context}: unexpected turn");
        }
    }
    assert_final(&game, &case.expect, &case.id);
}

/// Assert the typed rejection a step declares. Only engine error variants and
/// their structured fields are pinned; the human-readable `reason()` text is not
/// a contract and must not be enshrined here.
fn assert_error(step: &StepExpectation, error: &EngineError, context: &str) {
    let Some(expected) = &step.error else {
        panic!("{context}: a rejected step must declare its structured error");
    };
    match (expected, error) {
        (ErrorSpec::BadTiming { name }, EngineError::BadTiming { name: got, .. }) => {
            assert_eq!(got, name, "{context}: wrong card rejected");
        }
        (ErrorSpec::Illegal { action }, EngineError::Illegal { action: got, .. }) => {
            assert_eq!(got, action, "{context}: wrong action rejected");
        }
        _ => panic!("{context}: rejection {error:?} does not match the declared error contract"),
    }
}

fn build_initial(initial: &Initial, case: &str) -> Game {
    let active = parse_player(&initial.active, case);
    let step = parse_step(&initial.step, case);
    let mut builder = fixture(initial.seed);
    for (player, life) in &initial.life {
        builder = builder.life(parse_player(player, case), *life);
    }
    for (player, zones) in &initial.zones {
        let p = parse_player(player, case);
        builder = builder.hand(p, &cards(&zones.hand));
        builder = builder.library(p, &cards(&zones.library));
        builder = builder.battlefield(p, &cards(&zones.battlefield));
    }
    let mut game = builder.build();
    game.state.turn.number = initial.turn;
    game.state.turn.active = active;
    game.state.turn.step = step;
    game.state.turn.priority = initial
        .priority
        .as_ref()
        .map(|name| parse_player(name, case));
    game.state.turn.passed = 0;
    game.state.turn.lands_played = 0;
    for (name, state) in &initial.permanents {
        let object = unique_permanent(&game, name, case);
        apply_permanent_state(&mut game, object, state);
    }
    game
}

fn apply_permanent_state(game: &mut Game, id: ObjectId, state: &PermanentState) {
    let object = object_mut(&mut game.state, id);
    if let Some(damage) = state.damage {
        object.damage = damage;
    }
    if let Some(power) = state.pump_power {
        object.pump_power = power;
    }
    if let Some(toughness) = state.pump_toughness {
        object.pump_toughness = toughness;
    }
    // Issue 15: a fixture pump is a layer-7c continuous effect, not a bare
    // field. The typed record keeps the layer recomputation from clearing the
    // override the fixture asked for, and keeps the fixture honest about what
    // a "-2 toughness" premise means (CR 613.4c).
    let power = state.pump_power.unwrap_or(0);
    let toughness = state.pump_toughness.unwrap_or(0);
    if power != 0 || toughness != 0 {
        let card = game.state.object(id).card;
        let controller = game.state.object(id).controller;
        let effect_id = game.state.next_effect;
        game.state.next_effect += 1;
        game.state.effects.push(ContinuousEffect {
            id: effect_id,
            timestamp: 1,
            source: None,
            card,
            controller,
            layer: Layer::Modifying,
            target: id,
            kind: ContinuousKind::ModifyPowerToughness { power, toughness },
            duration: EffectDuration::UntilEndOfTurn,
        });
    }
}

fn unique_permanent(game: &Game, name: &str, context: &str) -> ObjectId {
    let card = catalog::by_name(name).unwrap_or_else(|| panic!("{context}: unknown card {name:?}"));
    let mut matches: Vec<ObjectId> = game
        .state
        .objects
        .values()
        .filter(|o| o.zone == Zone::Battlefield && o.card == card)
        .map(|o| o.id)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "{context}: expected exactly one {name} on the battlefield, found {}",
        matches.len()
    );
    matches.pop().expect("one match")
}

fn cards(names: &[String]) -> Vec<CardId> {
    names
        .iter()
        .map(|name| catalog::by_name(name).unwrap_or_else(|| panic!("unknown card {name:?}")))
        .collect()
}

fn resolve_action(game: &Game, actor: PlayerId, spec: &ActionSpec, context: &str) -> Action {
    match spec {
        ActionSpec::Pass => Action::Pass,
        ActionSpec::PlayLand { card } => Action::PlayLand {
            card: take_named(game, actor, Zone::Hand, card, context),
        },
        ActionSpec::CastSpell { card } => Action::CastSpell {
            card: take_named(game, actor, Zone::Hand, card, context),
            targets: Vec::new(),
        },
        ActionSpec::ActivateManaAbility { land } => Action::ActivateManaAbility {
            land: take_untapped_named(game, actor, land, context),
        },
        ActionSpec::DeclareAttackers {
            attackers,
            defender,
        } => {
            let mut used = BTreeSet::new();
            let ids: Vec<ObjectId> = attackers
                .iter()
                .map(|name| {
                    take_named_used(game, actor, Zone::Battlefield, name, &mut used, context)
                })
                .collect();
            let defender = match defender {
                Some(name) => Some(parse_player(name, context)),
                None if ids.is_empty() => None,
                None => {
                    let mut live = game.state.live_seats().filter(|seat| *seat != actor);
                    let only = live.next().unwrap_or_else(|| {
                        panic!("{context}: no live opponent for the attack declaration")
                    });
                    if live.next().is_some() {
                        panic!(
                            "{context}: declare_attackers must name a defender when multiple opponents are live"
                        );
                    }
                    Some(only)
                }
            };
            Action::DeclareAttackers {
                attackers: ids,
                defender,
            }
        }
        ActionSpec::DeclareBlockers { blocks } => {
            let mut used = BTreeSet::new();
            Action::DeclareBlockers {
                blocks: blocks
                    .iter()
                    .map(|block| BlockDecl {
                        blocker: take_named_used(
                            game,
                            actor,
                            Zone::Battlefield,
                            &block.blocker,
                            &mut used,
                            context,
                        ),
                        attacker: take_named(
                            game,
                            opponent_of(game, actor, context),
                            Zone::Battlefield,
                            &block.attacker,
                            context,
                        ),
                    })
                    .collect(),
            }
        }
        ActionSpec::Discard { cards } => {
            let mut used = BTreeSet::new();
            Action::Discard {
                cards: cards
                    .iter()
                    .map(|name| take_named_used(game, actor, Zone::Hand, name, &mut used, context))
                    .collect(),
            }
        }
    }
}

fn take_untapped_named(game: &Game, owner: PlayerId, name: &str, context: &str) -> ObjectId {
    named_ids(game, owner, Zone::Battlefield, name)
        .into_iter()
        .find(|id| !game.state.object(*id).tapped)
        .unwrap_or_else(|| {
            panic!("{context}: expected an untapped {name:?} controlled by P{owner}")
        })
}

fn take_named(game: &Game, owner: PlayerId, zone: Zone, name: &str, context: &str) -> ObjectId {
    let mut used = BTreeSet::new();
    take_named_used(game, owner, zone, name, &mut used, context)
}

fn take_named_used(
    game: &Game,
    owner: PlayerId,
    zone: Zone,
    name: &str,
    used: &mut BTreeSet<ObjectId>,
    context: &str,
) -> ObjectId {
    let ids = named_ids(game, owner, zone, name);
    let id = ids
        .iter()
        .copied()
        .find(|id| !used.contains(id))
        .unwrap_or_else(|| {
            panic!(
                "{context}: expected an unused {name:?} in P{owner}'s {}",
                zone.name()
            )
        });
    used.insert(id);
    id
}

fn named_ids(game: &Game, owner: PlayerId, zone: Zone, name: &str) -> Vec<ObjectId> {
    let Some(card) = catalog::by_name(name) else {
        return Vec::new();
    };
    match zone {
        Zone::Battlefield => game
            .state
            .objects
            .values()
            .filter(|o| o.zone == Zone::Battlefield && o.card == card && o.controller == owner)
            .map(|o| o.id)
            .collect(),
        Zone::Hand => game
            .state
            .player(owner)
            .hand
            .iter()
            .copied()
            .filter(|id| game.state.try_object(*id).is_some_and(|o| o.card == card))
            .collect(),
        other => panic!(
            "conformance fixtures cannot reference the {} zone",
            other.name()
        ),
    }
}

fn assert_events(expected: &[EventSpec], actual: &[Event], context: &str) {
    let mut cursor = 0;
    for spec in expected {
        let mut found = None;
        for (offset, event) in actual.iter().enumerate().skip(cursor) {
            if event_matches(spec, event) {
                found = Some(offset);
                break;
            }
        }
        let Some(offset) = found else {
            panic!(
                "{context}: expected event {spec:?} was not produced after event index {cursor}; actual: {actual:?}"
            );
        };
        cursor = offset + 1;
    }
}

fn event_matches(spec: &EventSpec, event: &Event) -> bool {
    let player_ok = |p: PlayerId| {
        spec.player
            .as_deref()
            .is_none_or(|s| parse_player(s, "event") == p)
    };
    let card_ok = |c: CardId| spec.card.as_deref().is_none_or(|s| c.name() == s);
    match (spec.kind.as_str(), event) {
        ("TurnBegin", Event::TurnBegin { turn, active }) => {
            spec.turn.is_none_or(|t| *turn == t)
                && spec
                    .active
                    .as_deref()
                    .is_none_or(|name| parse_player(name, "event") == *active)
        }
        ("StepBegin", Event::StepBegin { step }) => spec
            .step
            .as_deref()
            .is_none_or(|s| step_name_of(*step) == normalize(s)),
        ("Priority", Event::Priority { player }) => player_ok(*player),
        ("Pass", Event::Pass { player }) => player_ok(*player),
        ("LandPlayed", Event::LandPlayed { player, card, .. }) => {
            player_ok(*player) && card_ok(*card)
        }
        ("ManaTapped", Event::ManaTapped { player, .. }) => player_ok(*player),
        ("Cast", Event::Cast { player, card, .. }) => player_ok(*player) && card_ok(*card),
        ("Resolve", Event::Resolve { card, .. }) => card_ok(*card),
        ("Draw", Event::Draw { player, card, .. }) => player_ok(*player) && card_ok(*card),
        ("Discarded", Event::Discarded { player, card }) => player_ok(*player) && card_ok(*card),
        ("AttackersDeclared", Event::AttackersDeclared { player, attackers }) => {
            player_ok(*player) && spec.amount.is_none_or(|a| attackers.len() == a as usize)
        }
        ("BlockersDeclared", Event::BlockersDeclared { player, blocks }) => {
            player_ok(*player) && spec.amount.is_none_or(|a| blocks.len() == a as usize)
        }
        (
            "CombatDamage",
            Event::CombatDamage {
                attacker_card,
                blocker,
                amount,
                ..
            },
        ) => {
            spec.attacker
                .as_deref()
                .is_none_or(|s| attacker_card.name() == s)
                && match (&spec.blocker, blocker) {
                    (None, None) => true,
                    (Some(expected), Some((_, card))) => card.name() == expected,
                    _ => false,
                }
                && spec.amount.is_none_or(|a| *amount == a)
        }
        ("Destroyed", Event::Destroyed { card, .. }) => card_ok(*card),
        ("GameOver", Event::GameOver { winner, .. }) => spec
            .winner
            .as_deref()
            .is_none_or(|name| parse_player(name, "event") == *winner),
        (kind, _) if !known_event_kind(kind) => {
            panic!("event spec {kind:?} is not supported by this runner")
        }
        _ => false,
    }
}

fn known_event_kind(kind: &str) -> bool {
    matches!(
        kind,
        "TurnBegin"
            | "StepBegin"
            | "Priority"
            | "Pass"
            | "LandPlayed"
            | "ManaTapped"
            | "Cast"
            | "Resolve"
            | "Draw"
            | "Discarded"
            | "AttackersDeclared"
            | "BlockersDeclared"
            | "CombatDamage"
            | "Destroyed"
            | "GameOver"
    )
}

fn assert_final(game: &Game, expect: &FinalExpect, case: &str) {
    let context = format!("{case} final state");
    for (player, life) in &expect.life {
        let p = parse_player(player, &context);
        assert_eq!(
            game.state.player(p).life,
            *life,
            "{context}: unexpected life for P{p}"
        );
    }
    if let Some(turn) = expect.turn {
        assert_eq!(game.state.turn.number, turn, "{context}: unexpected turn");
    }
    if let Some(step) = &expect.step {
        assert_eq!(
            step_name_of(game.state.turn.step),
            normalize(step),
            "{context}: unexpected step"
        );
    }
    if let Some(priority) = &expect.priority {
        assert_eq!(
            priority_name(game.state.turn.priority),
            priority.as_str(),
            "{context}: unexpected priority holder"
        );
    }
    if let Some(pending) = &expect.pending {
        assert_eq!(
            pending_name(game.state.pending.as_ref()),
            pending.as_str(),
            "{context}: unexpected pending decision"
        );
    }
    for (player, zones) in &expect.zones {
        let p = parse_player(player, &context);
        let actual = game.state.player(p);
        if let Some(expected) = &zones.hand {
            assert_card_multiset(
                game,
                &actual.hand,
                expected,
                &format!("{context}: P{p} hand"),
            );
        }
        if let Some(expected) = &zones.library {
            assert_card_order(
                game,
                &actual.library,
                expected,
                &format!("{context}: P{p} library"),
            );
        }
        if let Some(expected) = &zones.graveyard {
            assert_card_multiset(
                game,
                &actual.graveyard,
                expected,
                &format!("{context}: P{p} graveyard"),
            );
        }
        if let Some(expected) = &zones.battlefield {
            let ids: Vec<ObjectId> = game.state.battlefield(p).iter().map(|o| o.id).collect();
            assert_card_multiset(
                game,
                &ids,
                expected,
                &format!("{context}: P{p} battlefield"),
            );
        }
    }
    for (name, state) in &expect.battlefield_state {
        let id = unique_permanent(game, name, &context);
        let object = game.state.object(id);
        if let Some(damage) = state.damage {
            assert_eq!(object.damage, damage, "{context}: {name} marked damage");
        }
        if let Some(power) = state.pump_power {
            assert_eq!(object.pump_power, power, "{context}: {name} pump power");
        }
        if let Some(toughness) = state.pump_toughness {
            assert_eq!(
                object.pump_toughness, toughness,
                "{context}: {name} pump toughness"
            );
        }
    }
    if let Some(outcome) = &expect.outcome {
        let Some(Outcome::Win { winner, .. }) = &game.state.outcome else {
            panic!(
                "{context}: expected a completed game, got {:?}",
                game.state.outcome
            );
        };
        assert_eq!(
            *winner,
            parse_player(&outcome.winner, &context),
            "{context}: unexpected winner"
        );
    }
}

fn assert_card_multiset(game: &Game, actual: &[ObjectId], expected: &[String], context: &str) {
    let mut actual_names: Vec<&str> = actual
        .iter()
        .map(|id| game.state.object(*id).card.name())
        .collect();
    let mut expected_names: Vec<&str> = expected.iter().map(String::as_str).collect();
    actual_names.sort_unstable();
    expected_names.sort_unstable();
    assert_eq!(actual_names, expected_names, "{context}");
}

/// Library assertions are order-sensitive on purpose: a shuffled or reordered
/// library is a real bug that a multiset comparison cannot see.
fn assert_card_order(game: &Game, actual: &[ObjectId], expected: &[String], context: &str) {
    let actual_names: Vec<&str> = actual
        .iter()
        .map(|id| game.state.object(*id).card.name())
        .collect();
    let expected_names: Vec<&str> = expected.iter().map(String::as_str).collect();
    assert_eq!(actual_names, expected_names, "{context} (order matters)");
}

fn opponent_of(game: &Game, actor: PlayerId, context: &str) -> PlayerId {
    let mut live = game.state.live_seats().filter(|seat| *seat != actor);
    let only = live
        .next()
        .unwrap_or_else(|| panic!("{context}: no live opponent"));
    assert!(
        live.next().is_none(),
        "{context}: two-seat helper used with multiple live opponents"
    );
    only
}

fn parse_player(name: &str, context: &str) -> PlayerId {
    match name {
        "P0" => P0,
        "P1" => P1,
        other => panic!("{context}: unknown player {other:?}"),
    }
}

fn parse_step(name: &str, context: &str) -> Step {
    STEP_ORDER
        .into_iter()
        .find(|step| step_name_of(*step) == normalize(name))
        .unwrap_or_else(|| panic!("{context}: unknown step {name:?}"))
}

fn step_name_of(step: Step) -> String {
    match step {
        Step::Untap => "untap",
        Step::Upkeep => "upkeep",
        Step::Draw => "draw",
        Step::PrecombatMain => "precombatmain",
        Step::BeginCombat => "begincombat",
        Step::DeclareAttackers => "declareattackers",
        Step::DeclareBlockers => "declareblockers",
        Step::CombatDamage => "combatdamage",
        Step::EndCombat => "endcombat",
        Step::PostcombatMain => "postcombatmain",
        Step::EndStep => "endstep",
        Step::Cleanup => "cleanup",
    }
    .to_string()
}

fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn priority_name(priority: Option<PlayerId>) -> &'static str {
    match priority {
        Some(P0) => "P0",
        Some(P1) => "P1",
        _ => "none",
    }
}

fn pending_name(pending: Option<&Pending>) -> &'static str {
    match pending {
        None => "none",
        Some(Pending::Mulligan { .. }) => "Mulligan",
        Some(Pending::BottomCards { .. }) => "BottomCards",
        Some(Pending::DeclareAttackers { .. }) => "DeclareAttackers",
        Some(Pending::DeclareBlockers { .. }) => "DeclareBlockers",
        Some(Pending::OrderCard { .. }) => "OrderCard",
        Some(Pending::Discard { count: 2, .. }) => "Discard(2)",
        Some(Pending::Discard { .. }) => "Discard",
        Some(Pending::ApnapSacrifice { .. }) => "ApnapSacrifice",
        Some(Pending::ApnapReveal { .. }) => "ApnapReveal",
        Some(Pending::CommanderReturn { .. }) => "CommanderReturn",
        Some(Pending::CommanderReplacement { .. }) => "CommanderReplacement",
        Some(Pending::CommanderDraw { .. }) => "CommanderDraw",
        Some(Pending::TriggerTarget { .. }) => "TriggerTarget",
        Some(Pending::TriggerChoice { .. }) => "TriggerChoice",
        Some(Pending::TriggerMode { .. }) => "TriggerMode",
        Some(Pending::TopCardChoice { .. }) => "TopCardChoice",
    }
}
