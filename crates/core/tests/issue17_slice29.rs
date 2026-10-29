//! Slice 29: Stuffed Bear — the pool's first permanent that is *not* a creature
//! until an ability animates it.
//!
//! The printed text is "{2}: This artifact becomes a 4/4 green Bear artifact
//! creature until end of turn." (MID 259, retrieved 2026-10-27, recorded in
//! `fixtures/starter-corpus/slice29-sourced.json`). One printed ability creates
//! three until-end-of-turn continuous effects — layer 4 adds the creature card
//! type and the Bear subtype in addition to the printed Artifact type
//! (CR 613.1d), layer 5 sets the colour to green (CR 613.1e), layer 7b sets base
//! power/toughness 4/4 (CR 613.4b) — and cleanup ends all three (CR 514.2).
//!
//! These tests exercise the animation through the engine's own offers and real
//! resolutions: target lists, combat, state-based actions, the death watchers
//! and a save round trip, not through an internal accessor.
mod common;

use aperture_core::ability::{self, Ability};
use aperture_core::catalog;
use aperture_core::layer::{Layer, SubtypeLine};
use aperture_core::*;
use common::*;

fn bear() -> CardId {
    catalog::by_name("Stuffed Bear").expect("Stuffed Bear must be supported")
}

fn pool(generic: u32) -> ManaPool {
    ManaPool {
        colorless: generic,
        ..ManaPool::default()
    }
}

fn drain(g: &mut Game) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..128 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return out;
        }
        let Some(player) = g.decision_owner() else {
            return out;
        };
        out.extend(submit_ok(g, player, Action::Pass));
    }
    panic!("the stack did not settle");
}

fn stats(g: &Game, id: ObjectId) -> (i32, i32) {
    let object = g.state.object(id);
    (object.power(), object.toughness())
}

fn is_creature(g: &Game, id: ObjectId) -> bool {
    let object = g.state.object(id);
    g.is_creature(object)
}

fn offered_targets(g: &Game, player: PlayerId, spec: TargetSpec) -> Vec<Target> {
    g.legal_targets(player, spec, ObjectId(u32::MAX))
}

/// Activate the printed animation and resolve it.
fn animate(g: &mut Game, player: PlayerId, source: ObjectId) -> Vec<Event> {
    let action = g
        .legal_actions()
        .into_iter()
        .find(|offered| {
            matches!(
                &offered.action,
                Action::ActivateAbility { source: s, .. } if *s == source
            )
        })
        .expect("the animation must be offered while {2} is payable")
        .action;
    let mut events = submit_ok(g, player, action);
    events.extend(drain(g));
    events
}

fn cast_bear(g: &mut Game, player: PlayerId) -> ObjectId {
    let id = hand_card(g, player, bear());
    let action = g
        .legal_actions()
        .into_iter()
        .find(|offered| matches!(&offered.action, Action::CastSpell { card, .. } if *card == id))
        .expect("Stuffed Bear must be castable")
        .action;
    submit_ok(g, player, action);
    let _ = drain(g);
    battlefield_card(g, player, bear())
}

/// Pass the whole table through the current turn and return the new active seat.
fn advance_turn(g: &mut Game) -> PlayerId {
    let start = g.state.turn.number;
    for _ in 0..800 {
        if g.state.turn.number != start {
            return g.state.turn.active;
        }
        assert!(!g.state.is_over(), "the game ended while advancing");
        let owner = g.decision_owner().expect("a seat decides while advancing");
        let action = auto_answer(g);
        let _ = g.submit_action(owner, action);
    }
    panic!("the turn did not advance");
}

fn bear_game(seed: u64) -> Game {
    let id = bear();
    fixture(seed)
        .battlefield(P0, &[id])
        .mana(P0, pool(2))
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build()
}

#[test]
fn the_catalog_string_is_the_recorded_printing_and_the_ability_is_typed() {
    let def = bear().def();
    assert_eq!(def.name, "Stuffed Bear");
    assert_eq!(def.cost, Some("{2}"));
    assert_eq!(def.type_line, "Artifact");
    assert_eq!((def.power, def.toughness), (None, None));
    assert_eq!(
        def.oracle,
        "{2}: This artifact becomes a 4/4 green Bear artifact creature until end of turn."
    );
    assert!(def.is_artifact() && !def.is_creature(), "printed Artifact");
    let abilities = ability::abilities(bear());
    assert_eq!(abilities.len(), 1, "one printed ability");
    let activated = abilities[0]
        .activated()
        .expect("the printed ability is activated");
    assert_eq!(activated.text, def.oracle);
    assert_eq!(activated.cost.mana, Some("{2}"));
    assert!(!activated.cost.tap && !activated.cost.sacrifice_source);
    assert!(
        !activated.once_per_turn,
        "the card prints no once-per-turn limit"
    );
    assert_eq!(
        activated.effects,
        &[
            Effect::AddSourceCreatureType(SubtypeLine::Bear),
            Effect::SetSourceColor(ManaColor::Green),
            Effect::SetSourceBasePowerToughness(4, 4),
        ],
        "printed order: creature type, colour, base power/toughness"
    );
    assert_eq!(
        (
            Layer::Type.label(),
            Layer::Color.label(),
            Layer::Setting.label()
        ),
        ("4", "5", "7b"),
        "the three layers the printing spans"
    );
    assert!(
        ability::can_become_creature(bear()),
        "the entry rule has to know this permanent can animate itself"
    );
    assert!(!ability::can_become_creature(CardId::GrizzlyBears));
}

#[test]
fn it_is_a_plain_artifact_until_the_printed_ability_animates_it() {
    let mut game = bear_game(2901);
    let id = battlefield_card(&game, P0, bear());

    assert!(!is_creature(&game, id), "an Artifact is not a creature");
    assert!(!game.has_color(id, ManaColor::Green), "not green yet");
    assert!(
        game.has_color(id, ManaColor::Colorless),
        "a colourless artifact"
    );
    assert!(
        !offered_targets(&game, P0, TargetSpec::Creature).contains(&Target::Permanent(id)),
        "\"target creature\" must not offer a noncreature artifact"
    );
    assert!(
        !offered_targets(&game, P0, TargetSpec::Bear).contains(&Target::Permanent(id)),
        "an unanimated Stuffed Bear is not a Bear"
    );

    let events = animate(&mut game, P0, id);
    let layers: Vec<String> = events
        .iter()
        .filter_map(|event| match event {
            Event::ContinuousEffectStarted { layer, .. } => Some(layer.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        layers,
        vec!["4".to_string(), "5".to_string(), "7b".to_string()],
        "one printed sentence, three layers"
    );

    assert!(is_creature(&game, id), "it is an artifact creature now");
    assert!(
        game.state.object(id).card.def().is_artifact(),
        "still an artifact"
    );
    assert_eq!(stats(&game, id), (4, 4), "layer 7b base 4/4");
    assert!(game.has_color(id, ManaColor::Green), "layer 5 sets green");
    assert!(!game.has_color(id, ManaColor::Colorless));
    assert!(
        offered_targets(&game, P0, TargetSpec::Creature).contains(&Target::Permanent(id)),
        "the animated artifact is a legal creature target"
    );
    assert!(
        offered_targets(&game, P0, TargetSpec::Bear).contains(&Target::Permanent(id)),
        "layer 4 adds the Bear subtype the engine's Bear filters read"
    );
    assert!(
        !offered_targets(&game, P0, TargetSpec::HumanYouControl).contains(&Target::Permanent(id)),
        "it is a Bear, not a Human"
    );
    let described: Vec<String> = game
        .state
        .effects
        .iter()
        .map(|effect| effect.describe())
        .collect();
    assert_eq!(
        described,
        vec![
            "becomes a creature with type Bear until end of turn (CR 613.1d type-changing effects)"
                .to_string(),
            "becomes green until end of turn (CR 613.1e colour-changing effects)".to_string(),
            "base power/toughness 4/4 until end of turn (CR 613.4b base power/toughness setting)"
                .to_string(),
        ],
        "one continuous effect per layer, each naming its own layer"
    );
}

#[test]
fn the_animation_expires_at_cleanup_and_the_artifact_reverts() {
    let mut game = bear_game(2902);
    let id = battlefield_card(&game, P0, bear());
    animate(&mut game, P0, id);
    assert!(is_creature(&game, id));

    assert_eq!(advance_turn(&mut game), P1, "the turn ended");
    assert!(game.state.effects.is_empty(), "CR 514.2: effects ended");
    assert!(
        !is_creature(&game, id),
        "the artifact is not a creature again"
    );
    assert_eq!(stats(&game, id), (0, 0), "no printed power/toughness");
    assert!(!game.has_color(id, ManaColor::Green), "green ended");
    assert!(
        !offered_targets(&game, P0, TargetSpec::Bear).contains(&Target::Permanent(id)),
        "the added Bear subtype ended too"
    );
    assert!(
        !offered_targets(&game, P0, TargetSpec::Creature).contains(&Target::Permanent(id)),
        "and it can no longer be targeted as a creature"
    );
}

#[test]
fn an_animated_bear_dies_as_a_creature() {
    // Zulaport Cutthroat prints "whenever this creature or another creature you
    // control dies, each opponent loses 1 life and you gain 1 life", so its drain
    // is proof that the engine recorded the animated artifact's death as a
    // *creature* death (CR 700.4), not as an artifact leaving the battlefield.
    let mut game = fixture(2903)
        .battlefield(P0, &[bear(), CardId::ZulaportCutthroat])
        .mana(P0, pool(2))
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build();
    let id = battlefield_card(&game, P0, bear());
    animate(&mut game, P0, id);
    assert!(is_creature(&game, id));

    // Four damage kills a 4/4; the death is a state-based action (CR 704.5g).
    object_mut(&mut game.state, id).damage = 4;
    submit_ok(&mut game, P0, Action::Pass);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P0).life == 41,
        64
    ));

    assert!(
        game.state.player(P0).graveyard.iter().any(|id| game
            .state
            .try_object(*id)
            .is_some_and(|object| object.card == bear())),
        "the animated artifact died"
    );
    assert_eq!(
        game.state.player(P0).life,
        41,
        "the creature-death watcher drained"
    );
    assert_eq!(
        game.state.player(P1).life,
        39,
        "and its opponent lost 1 life"
    );
}

#[test]
fn it_cannot_attack_on_the_turn_it_entered_but_attacks_for_four_on_a_later_turn() {
    let mut game = fixture(2904)
        .hand(P0, &[bear()])
        // A non-sick creature keeps the announce-attackers step alive; without
        // one the engine correctly skips the declaration entirely.
        .battlefield(P0, &[CardId::GrizzlyBears])
        .mana(P0, pool(8))
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build();
    let id = cast_bear(&mut game, P0);
    let grizzly = battlefield_card(&game, P0, CardId::GrizzlyBears);
    assert!(
        game.state.object(id).summoning_sick,
        "CR 302.6 records the restriction on arrival even though it is not a creature"
    );
    animate(&mut game, P0, id);
    assert!(is_creature(&game, id));

    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    let refused = game.submit_action(
        P0,
        Action::DeclareAttackers {
            attackers: vec![id],
            defender: Some(P1),
        },
    );
    assert!(
        refused.is_err(),
        "a permanent animated on the turn it arrived is summoning sick"
    );
    assert!(
        !game
            .legal_actions()
            .into_iter()
            .any(|offered| matches!(&offered.action, Action::DeclareAttackers { attackers, .. } if attackers.contains(&id))),
        "the engine does not even offer the sick animated artifact as an attacker"
    );

    // Finish this turn and come back around to P0's next turn.
    assert_eq!(advance_turn(&mut game), P1);
    assert!(!is_creature(&game, id), "the animation ended with the turn");
    assert_eq!(advance_turn(&mut game), P0);
    assert!(
        !game.state.object(id).summoning_sick,
        "the untap step cleared it"
    );

    game.state.players[P0 as usize].mana = pool(2);
    animate(&mut game, P0, id);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![id, grizzly],
            defender: Some(P1),
        },
    );
    let mut combat = Vec::new();
    assert!(run_until(
        &mut game,
        &mut combat,
        |g| g.state.player(P1).life == 34,
        64
    ));
    assert_eq!(
        game.state.player(P1).life,
        34,
        "the animated 4/4 plus the 2/2 dealt six combat damage"
    );
    assert_eq!(game.state.player(P0).life, 40, "and took nothing back");
}

#[test]
fn a_second_activation_restacks_the_same_layers_without_stacking_the_stats() {
    // The card prints no once-per-turn limit, so activating twice is legal. A
    // base *setter* does not add: the second layer 7b effect re-sets 4/4 rather
    // than making it 8/8 (CR 613.4b), and the type and colour effects stay
    // single results as well.
    let mut game = bear_game(2905);
    let id = battlefield_card(&game, P0, bear());
    game.state.players[P0 as usize].mana = pool(4);
    animate(&mut game, P0, id);
    animate(&mut game, P0, id);

    assert_eq!(
        game.state.effects.len(),
        6,
        "two resolutions, three layers each"
    );
    assert!(is_creature(&game, id));
    assert_eq!(stats(&game, id), (4, 4), "a set is not an addition");
    assert!(game.has_color(id, ManaColor::Green));
}

#[test]
fn a_save_and_reload_preserves_the_animation_and_its_layers() {
    let mut game = bear_game(2906);
    let id = battlefield_card(&game, P0, bear());
    animate(&mut game, P0, id);
    // The animation is state, not a cache: a state round trip keeps the three
    // continuous effects and therefore the creature type, colour and P/T.
    let text = serde_json::to_string(&game.state).expect("the state serializes");
    let reloaded_state: GameState = serde_json::from_str(&text).expect("the state reloads");
    assert_eq!(
        reloaded_state, game.state,
        "the animation is recorded state, not a cache that a reload loses"
    );
    let reloaded = Game {
        state: reloaded_state,
    };
    assert!(
        is_creature(&reloaded, id),
        "the reloaded artifact is still animated"
    );
    assert_eq!(stats(&reloaded, id), (4, 4));
    assert!(reloaded.has_color(id, ManaColor::Green));
}

#[test]
fn the_colour_query_starts_from_the_printed_colour_and_layer_five_replaces_it() {
    // The printed baseline: a black creature answers black through its cost
    // (CR 202.2a, Festering Goblin), a colourless artifact answers no colour.
    let mut game = fixture(2907)
        .battlefield(P0, &[bear(), CardId::FesteringGoblin])
        .mana(P0, pool(2))
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear_id = battlefield_card(&game, P0, bear());
    let goblin = battlefield_card(&game, P0, CardId::FesteringGoblin);
    assert!(game.has_color(goblin, ManaColor::Black), "printed black");
    assert!(!game.has_color(goblin, ManaColor::Green));
    assert!(game.has_color(bear_id, ManaColor::Colorless));

    animate(&mut game, P0, bear_id);
    assert!(
        game.has_color(bear_id, ManaColor::Green),
        "layer 5 set green"
    );
    assert!(
        !game.has_color(bear_id, ManaColor::Colorless),
        "a set colour replaces colourless"
    );
    assert!(
        !game.has_color(bear_id, ManaColor::Black),
        "and green is not black"
    );
    assert!(game.has_color(goblin, ManaColor::Black), "unaffected");
}

#[test]
fn the_ability_offer_requires_two_mana_and_the_card_is_not_a_creature_spell() {
    let mut game = fixture(2908)
        .battlefield(P0, &[bear()])
        .mana(P0, pool(1))
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let id = battlefield_card(&game, P0, bear());
    assert!(
        !game.legal_actions().into_iter().any(|offered| matches!(
            offered.action,
            Action::ActivateAbility { source, .. } if source == id
        )),
        "{{2}} cannot be paid from one mana"
    );
    game.state.players[P0 as usize].mana = pool(2);
    assert!(game.legal_actions().into_iter().any(|offered| matches!(
        offered.action,
        Action::ActivateAbility { source, .. } if source == id
    )));
    // The printed card is an artifact spell: it is not a creature entering, so
    // "another creature enters" watchers must not see it.
    assert!(!bear().def().is_creature());
    assert!(matches!(
        bear().def().spell_spec(),
        Some(spec) if spec.effects.is_empty()
    ));
    assert_eq!(
        Ability::text(&ability::abilities(bear())[0]),
        bear().def().oracle
    );
}
