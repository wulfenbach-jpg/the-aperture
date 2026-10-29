//! Slice 64 behaviour: the grouped Krenko batch that shares real primitives.
//!
//! Goblin Sharpshooter (standing untap-step restriction + direct untap on a
//! creature death + a ping), Krenko, Tin Street Kingpin (tokens scaled to its own
//! power after its attack counter), Hobgoblin Bandit Lord (the shared Goblin
//! anthem plus damage equal to the Goblins that entered under its controller this
//! turn) and Goblin Matron (an optional subtype-filtered library search).
mod common;
use aperture_core::ability::{Ability, TriggerEvent, abilities};
use aperture_core::*;
use common::*;

// The fixture helper builds a two-seat table (P0, P1); every damage target in
// these tests is P1, the only opponent.
const OPPONENT: PlayerId = P1;

fn red(generic: u32, red: u32) -> ManaPool {
    ManaPool {
        red,
        colorless: generic,
        ..ManaPool::default()
    }
}

fn goblin_tokens(g: &Game, p: PlayerId) -> Vec<ObjectId> {
    g.state
        .battlefield(p)
        .into_iter()
        .filter(|o| o.card == CardId::GoblinToken)
        .map(|o| o.id)
        .collect()
}

fn reached_untap_step(game: &Game, turn: u32) -> bool {
    // The untap step itself gives no priority, so "reached P0's untap step" is
    // observed just after it: the turn is P0's, the step has advanced past
    // Untap, and a priority window exists. (Same shape as slice 19's helper.)
    game.state.turn.number == turn
        && game.state.turn.active == P0
        && game.state.turn.step != Step::Untap
}

/// "{T}: this creature deals 1 damage to any target" on Sharpshooter, which is
/// its second declared ability (the dies-trigger is the first).
fn ping_player(g: &mut Game, shooter: ObjectId) -> Result<Vec<Event>, EngineError> {
    let p = g.decision_owner().expect("priority");
    let target = (0..g.state.seat_count())
        .map(|seat| seat as PlayerId)
        .find(|seat| *seat != p)
        .expect("an opponent seat");
    g.submit_action(
        p,
        Action::ActivateAbility {
            source: shooter,
            ability: 1,
            targets: vec![Target::Player(target)],
            mode: 0,
        },
    )
}

fn activate_bandit(
    game: &mut Game,
    lord: ObjectId,
    target: Target,
) -> Result<Vec<Event>, EngineError> {
    let p = game.decision_owner().expect("priority");
    game.submit_action(
        p,
        Action::ActivateAbility {
            source: lord,
            ability: 0,
            targets: vec![target],
            mode: 0,
        },
    )
}

#[test]
fn sharpshooter_never_untaps_in_its_controllers_untap_step() {
    let mut game = fixture(6401)
        .battlefield(P0, &[CardId::GoblinSharpshooter, CardId::GrizzlyBears])
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let shooter = battlefield_card(&game, P0, CardId::GoblinSharpshooter);
    let bystander = battlefield_card(&game, P0, CardId::GrizzlyBears);

    // Tap it by actually using the ability, so the tapped state comes from the
    // engine rather than being forced into the fixture. The ability goes on the
    // stack, so the damage is real only once it resolves.
    let mut log = ping_player(&mut game, shooter).expect("the ping is legal");
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        4000,
    );
    assert!(
        log.iter()
            .any(|e| matches!(e, Event::Damage { amount: 1, .. })),
        "the ping deals 1 damage, got {log:#?}"
    );
    assert!(game.state.object(shooter).tapped);

    let mut log = Vec::new();
    run_until(&mut game, &mut log, |g| reached_untap_step(g, 3), 8000);
    assert_eq!(game.state.turn.number, 3, "the driver reached turn 3");
    assert!(
        game.state.object(shooter).tapped,
        "Goblin Sharpshooter must not untap during its controller's untap step"
    );
    assert!(
        !game.state.object(bystander).tapped,
        "an ordinary creature untaps in the same step, so the skip is not a global freeze"
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description }
                if description.contains("doesn't untap during its controller's untap step")
        )),
        "the standing restriction is recorded as a rules event"
    );

    // Standing, not one-shot: the next untap step skips it again, which a one-shot
    // marker (the Spectral Bears shape) would not do.
    run_until(&mut game, &mut log, |g| reached_untap_step(g, 5), 8000);
    assert_eq!(game.state.turn.number, 5, "the driver reached turn 5");
    assert!(
        game.state.object(shooter).tapped,
        "the restriction reapplies every turn; it is not a one-shot marker"
    );
    assert!(
        game.state.skip_next_untap.is_empty(),
        "no one-shot skip marker was consumed for the standing restriction"
    );
}

#[test]
fn sharpshooter_untaps_on_an_opponents_creature_dying_and_can_ping_again() {
    let mut game = fixture(6402)
        .battlefield(P0, &[CardId::GoblinSharpshooter])
        .battlefield(P1, &[CardId::ElvishVisionary])
        .hand(P0, &[CardId::LightningStrike])
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(P0, red(2, 1))
        .main(P0, Step::PrecombatMain)
        .build();
    let shooter = battlefield_card(&game, P0, CardId::GoblinSharpshooter);
    let mut ping_log = ping_player(&mut game, shooter).expect("the ping is legal");
    run_until(
        &mut game,
        &mut ping_log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        4000,
    );
    assert!(
        game.state.object(shooter).tapped,
        "the ping taps the Sharpshooter"
    );

    let victim = battlefield_card(&game, P1, CardId::ElvishVisionary);
    let strike = hand_card(&game, P0, CardId::LightningStrike);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Permanent(victim)],
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| {
            g.state
                .try_object(victim)
                .is_some_and(|o| o.zone == Zone::Graveyard)
                && g.state.stack.is_empty()
                && g.state.pending.is_none()
        },
        8000,
    );
    assert!(
        game.state
            .try_object(victim)
            .is_some_and(|o| o.zone == Zone::Graveyard),
        "the victim actually died"
    );
    assert!(
        !game.state.object(shooter).tapped,
        "a creature dying untaps Goblin Sharpshooter, whoever controlled it"
    );
    // And the untapped Sharpshooter is usable again in the same turn.
    ping_player(&mut game, shooter).expect("the untapped Sharpshooter pings again");
}

#[test]
fn krenko_creates_tokens_equal_to_his_power_after_the_attack_counter() {
    let mut game = fixture(6403)
        .battlefield(P0, &[CardId::KrenkoTinStreetKingpin])
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::DeclareAttackers)
        .build();
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let krenko = battlefield_card(&game, P0, CardId::KrenkoTinStreetKingpin);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![krenko],
            defender: Some(P1),
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        game.state
            .object(krenko)
            .counters
            .get(counters::CounterKind::PlusOnePlusOne),
        1,
        "the attack trigger puts one +1/+1 counter on Krenko"
    );
    assert_eq!(
        game.state.object(krenko).power(),
        2,
        "the counter is already part of Krenko's power"
    );
    assert_eq!(
        goblin_tokens(&game, P0).len(),
        2,
        "the token count is Krenko's power *after* the counter, not his printed 1"
    );
}

#[test]
fn krenko_token_count_reads_an_anthem_pumped_power() {
    // With one Goblin Chieftain, Krenko is a 2/3 that becomes 3/4 after its own
    // counter, so the same trigger creates three tokens. This separates reading
    // *effective* power from reading the printed number.
    let mut game = fixture(6404)
        .battlefield(
            P0,
            &[CardId::KrenkoTinStreetKingpin, CardId::GoblinChieftain],
        )
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::DeclareAttackers)
        .build();
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let krenko = battlefield_card(&game, P0, CardId::KrenkoTinStreetKingpin);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![krenko],
            defender: Some(P1),
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(game.state.object(krenko).power(), 3);
    assert_eq!(
        goblin_tokens(&game, P0).len(),
        3,
        "effective power (printed + anthem + counter) decides the token count"
    );
}

#[test]
fn bandit_lord_anthem_and_damage_reads_only_this_turns_goblin_entries() {
    let mut game = fixture(6405)
        .battlefield(
            P0,
            &[
                CardId::HobgoblinBanditLord,
                CardId::GoblinSharpshooter,
                CardId::ElvishVisionary,
            ],
        )
        .hand(P0, &[CardId::GoblinInstigator])
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(P0, red(6, 3))
        .main(P0, Step::PrecombatMain)
        .build();
    let lord = battlefield_card(&game, P0, CardId::HobgoblinBanditLord);
    let other_goblin = battlefield_card(&game, P0, CardId::GoblinSharpshooter);
    let elf = battlefield_card(&game, P0, CardId::ElvishVisionary);

    // The shared layer 7c anthem: other Goblins get +1/+1; a non-Goblin and the
    // lord itself do not. No Goblin has entered this turn, so the activation
    // deals no damage at all (CR 120.4a) rather than a "0 damage" event.
    let log = activate_bandit(&mut game, lord, Target::Player(OPPONENT)).expect("legal");
    assert!(
        !log.iter().any(|e| matches!(e, Event::Damage { .. })),
        "with no Goblin entries this turn the activation deals no damage: {log:#?}"
    );
    assert!(
        game.state.try_object(lord).is_some_and(|o| o.tapped),
        "the {{R}}, {{T}} cost tapped the Bandit Lord even though it dealt nothing"
    );

    // Resolving the activation passes priority around, which is when the additive
    // printed statics are recomputed (the same point slice 63 observed the Goblin
    // King anthem at).
    let mut log = Vec::new();
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.decision_owner() == Some(P0),
        4000,
    );
    assert_eq!(game.state.object(other_goblin).power(), 2);
    assert_eq!(game.state.object(elf).power(), 1);
    assert_eq!(game.state.object(lord).power(), 2);

    // Cast Goblin Instigator: it enters (one Goblin) and creates a Goblin token
    // (a second Goblin), so this turn's history is 2.
    let instigator = hand_card(&game, P0, CardId::GoblinInstigator);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: instigator,
            targets: Vec::new(),
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        game.state
            .goblins_entered_this_turn
            .get(usize::from(P0))
            .copied(),
        Some(2),
        "the Instigator and its token are two Goblin entries this turn"
    );

    // Untap and fire again: the damage is exactly those two entries, not the four
    // Goblins now on the battlefield.
    game.state.objects.get_mut(&lord).unwrap().tapped = false;
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.decision_owner() == Some(P0),
        4000,
    );
    let before = game.state.players[OPPONENT as usize].life;
    let mut activation = activate_bandit(&mut game, lord, Target::Player(OPPONENT)).expect("legal");
    run_until(
        &mut game,
        &mut activation,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        4000,
    );
    assert!(
        activation
            .iter()
            .any(|e| matches!(e, Event::Damage { amount: 2, .. })),
        "damage equals this turn's Goblin entries (2), not the board's Goblin count: \
         {activation:#?}"
    );
    assert_eq!(game.state.players[OPPONENT as usize].life, before - 2);
}

#[test]
fn bandit_lord_entry_history_is_cleared_when_the_turn_advances() {
    let mut game = fixture(6406)
        .battlefield(P0, &[CardId::HobgoblinBanditLord])
        .hand(P0, &[CardId::GoblinInstigator])
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(P0, red(2, 1))
        .main(P0, Step::PrecombatMain)
        .build();
    let instigator = hand_card(&game, P0, CardId::GoblinInstigator);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: instigator,
            targets: Vec::new(),
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        game.state
            .goblins_entered_this_turn
            .get(usize::from(P0))
            .copied(),
        Some(2),
        "two Goblin entries were recorded this turn"
    );
    run_until(&mut game, &mut log, |g| g.state.turn.number >= 3, 12000);
    assert!(game.state.turn.number >= 3);
    assert!(
        game.state.goblins_entered_this_turn.is_empty(),
        "the entry history is cleared when the turn advances, got {:?}",
        game.state.goblins_entered_this_turn
    );
}

#[test]
fn goblin_matron_searches_only_goblins_reveals_and_shuffles() {
    let mut game = fixture(6407)
        .hand(P0, &[CardId::GoblinMatron])
        .library(
            P0,
            &[
                CardId::GrizzlyBears,
                CardId::GoblinWarchief,
                CardId::Mountain,
                CardId::Forest,
                CardId::GoblinChieftain,
            ],
        )
        .library(P1, &[CardId::Forest; 10])
        .mana(P0, red(2, 1))
        .main(P0, Step::PrecombatMain)
        .build();
    let matron = hand_card(&game, P0, CardId::GoblinMatron);
    let library_before: Vec<ObjectId> = game.state.player(P0).library.clone();
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: matron,
            targets: Vec::new(),
        },
    );
    // The "you may" is answered on the stack, before anything is searched.
    let mut log = Vec::new();
    run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::TriggerChoice { .. })),
        4000,
    );
    assert!(
        matches!(game.state.pending, Some(Pending::TriggerChoice { .. })),
        "the optional search is a real choice"
    );
    let owner = game.decision_owner().expect("priority");
    submit_ok(
        &mut game,
        owner,
        Action::AnswerTriggerChoice { use_ability: true },
    );
    run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::SearchLibrary { .. })),
        4000,
    );
    let Some(Pending::SearchLibrary { eligible, mode, .. }) = &game.state.pending else {
        panic!("the accepted search creates a library-search choice");
    };
    assert_eq!(
        *mode,
        LibrarySearchMode::GoblinCardToHand,
        "the search is a Goblin search"
    );
    let eligible_names = names(&game, eligible);
    assert_eq!(
        eligible_names.len(),
        2,
        "exactly the two Goblin cards are searchable, got {eligible_names:?}"
    );
    assert!(eligible_names.contains(&"Goblin Warchief"));
    assert!(eligible_names.contains(&"Goblin Chieftain"));
    assert!(
        !eligible_names.contains(&"Grizzly Bears") && !eligible_names.contains(&"Mountain"),
        "a non-Goblin card is never offered"
    );

    let pick = *eligible
        .iter()
        .find(|id| game.state.object(**id).card == CardId::GoblinWarchief)
        .expect("Warchief is eligible");
    let mut pick_log = submit_ok(&mut game, owner, Action::SearchLibraryPick { card: pick });
    if let Some(Pending::SearchLibrary { .. }) = game.state.pending {
        pick_log.extend(submit_ok(&mut game, owner, Action::SearchLibraryDone));
    }
    let hand: Vec<&'static str> = game
        .state
        .player(P0)
        .hand
        .iter()
        .map(|id| game.state.object(*id).card.name())
        .collect();
    assert!(
        hand.contains(&"Goblin Warchief"),
        "the searched Goblin goes to hand, got {hand:?}"
    );
    assert!(
        pick_log.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description }
                if description.contains("reveals Goblin Warchief")
        )),
        "the searched card is revealed to the table"
    );
    assert!(
        pick_log.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description }
                if description.contains("shuffles their library after searching")
        )),
        "a search ends with a shuffle"
    );
    let library_after: Vec<ObjectId> = game.state.player(P0).library.clone();
    assert_eq!(
        library_after.len(),
        library_before.len() - 1,
        "the found card left the library"
    );
    assert_ne!(
        library_after,
        library_before
            .iter()
            .copied()
            .filter(|id| *id != pick)
            .collect::<Vec<_>>(),
        "the shuffle actually reordered what remained"
    );
}

#[test]
fn goblin_matron_declining_the_may_search_changes_nothing() {
    let mut game = fixture(6408)
        .hand(P0, &[CardId::GoblinMatron])
        .library(
            P0,
            &[
                CardId::GrizzlyBears,
                CardId::GoblinWarchief,
                CardId::Mountain,
            ],
        )
        .library(P1, &[CardId::Forest; 10])
        .mana(P0, red(2, 1))
        .main(P0, Step::PrecombatMain)
        .build();
    let matron = hand_card(&game, P0, CardId::GoblinMatron);
    let library_before: Vec<ObjectId> = game.state.player(P0).library.clone();
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: matron,
            targets: Vec::new(),
        },
    );
    let mut log = Vec::new();
    run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::TriggerChoice { .. })),
        4000,
    );
    let owner = game.decision_owner().expect("priority");
    submit_ok(
        &mut game,
        owner,
        Action::AnswerTriggerChoice { use_ability: false },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        4000,
    );
    assert!(
        !log.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description }
                if description.contains("shuffles their library")
        )),
        "declining the \"may\" search must not shuffle anything"
    );
    assert_eq!(
        game.state.player(P0).library,
        library_before,
        "declining leaves the library exactly as it was"
    );
}

#[test]
fn matron_search_refuses_a_forged_non_goblin_pick() {
    let mut game = fixture(6409)
        .hand(P0, &[CardId::GoblinMatron])
        .library(
            P0,
            &[
                CardId::GrizzlyBears,
                CardId::GoblinWarchief,
                CardId::Mountain,
            ],
        )
        .library(P1, &[CardId::Forest; 10])
        .mana(P0, red(2, 1))
        .main(P0, Step::PrecombatMain)
        .build();
    let matron = hand_card(&game, P0, CardId::GoblinMatron);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: matron,
            targets: Vec::new(),
        },
    );
    let mut log = Vec::new();
    run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::TriggerChoice { .. })),
        4000,
    );
    let owner = game.decision_owner().expect("priority");
    submit_ok(
        &mut game,
        owner,
        Action::AnswerTriggerChoice { use_ability: true },
    );
    run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::SearchLibrary { .. })),
        4000,
    );
    let bear = game
        .state
        .player(P0)
        .library
        .iter()
        .copied()
        .find(|id| game.state.object(*id).card == CardId::GrizzlyBears)
        .expect("a Bear is in the library");
    let err = submit_err(&mut game, owner, Action::SearchLibraryPick { card: bear });
    assert!(
        matches!(
            err,
            EngineError::Illegal { .. } | EngineError::InvalidState { .. }
        ),
        "a non-Goblin pick is refused, got {err:?}"
    );
    assert_eq!(
        game.state.player(P0).library.len(),
        3,
        "the refused pick moved nothing"
    );
}

#[test]
fn slice64_cards_are_playable_and_the_state_round_trips_through_a_save() {
    let mut game = fixture(6410)
        .battlefield(
            P0,
            &[
                CardId::GoblinSharpshooter,
                CardId::KrenkoTinStreetKingpin,
                CardId::HobgoblinBanditLord,
                CardId::GoblinMatron,
            ],
        )
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(P0, red(0, 1))
        .main(P0, Step::PrecombatMain)
        .build();
    let lord = battlefield_card(&game, P0, CardId::HobgoblinBanditLord);
    activate_bandit(&mut game, lord, Target::Player(OPPONENT)).expect("legal activation");
    let before = game.state.clone();
    before
        .validate()
        .expect("the state after a slice-64 activation is valid");
    let json = serde_json::to_string(&before).expect("state serializes");
    let loaded: GameState = serde_json::from_str(&json).expect("state deserializes");
    assert_eq!(loaded, before, "the saved state round-trips exactly");
}

#[test]
fn slice64_cards_declare_exactly_the_abilities_their_printings_have() {
    for (card, triggered, activated) in [
        (CardId::GoblinSharpshooter, 1usize, 1usize),
        (CardId::KrenkoTinStreetKingpin, 1, 0),
        (CardId::HobgoblinBanditLord, 0, 1),
        (CardId::GoblinMatron, 1, 0),
    ] {
        let list = abilities(card);
        let t = list
            .iter()
            .filter(|a| matches!(a, Ability::Triggered(_)))
            .count();
        let a = list
            .iter()
            .filter(|a| matches!(a, Ability::Activated(_)))
            .count();
        assert_eq!((t, a), (triggered, activated), "{}", card.name());
    }
    assert!(
        abilities(CardId::GoblinSharpshooter).iter().any(|ability| {
            matches!(
                ability,
                Ability::Triggered(triggered) if triggered.event == TriggerEvent::CreatureDies
            )
        }),
        "the untap is a creature-dies trigger"
    );
}
