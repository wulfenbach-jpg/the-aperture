//! Command-zone practice constructor behavior. The first cycle checks only the
//! supported-commander acceptance because the command-zone fields are introduced
//! by the implementation that follows the RED run.

use aperture_core::{CardId, DeckList, Game, P0, P1};

fn decks() -> Vec<DeckList> {
    vec![
        DeckList::parse_practice("A", "40 Forest\n").expect("deck A"),
        DeckList::parse_practice("B", "40 Forest\n").expect("deck B"),
    ]
}

#[test]
fn a_supported_single_commander_per_seat_constructs_a_game() {
    let result = Game::new_command_zone(
        7,
        &decks(),
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    );
    assert!(
        result.is_ok(),
        "one supported commander per seat must construct: {:?}",
        result.err()
    );
}

#[test]
fn commanders_start_independently_in_the_command_zone() {
    let game = Game::new_command_zone(
        9,
        &decks(),
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect("supported setup");
    assert!(game.state.command_zone_practice);
    for seat in [P0, P1] {
        let player = game.state.player(seat);
        assert_eq!(player.command_zone.len(), 1);
        assert!(
            player
                .library
                .iter()
                .chain(player.hand.iter())
                .all(|id| game.state.object(*id).card != CardId::IsamaruHoundOfKonda),
            "the commander must not be dealt or shuffled into seat P{seat}'s library"
        );
        let registry = game
            .state
            .commanders
            .iter()
            .find(|c| c.owner == seat)
            .expect("registry entry per seat");
        assert_eq!(registry.card, CardId::IsamaruHoundOfKonda);
        assert_eq!(registry.casts_from_command_zone, 0);
        assert_eq!(registry.object, player.command_zone[0]);
        assert_eq!(
            game.state.object(registry.object).zone,
            aperture_core::Zone::Command
        );
        assert_eq!(
            game.state.object(registry.object).commander,
            Some(registry.id)
        );
    }
    assert_ne!(
        game.state.player(P0).command_zone,
        game.state.player(P1).command_zone
    );
}

#[test]
fn unsupported_commander_and_multi_commander_setups_are_refused() {
    let unsupported = Game::new_command_zone(
        1,
        &decks(),
        &[
            (P0, CardId::GrizzlyBears),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect_err("a non-commander card must not be accepted as a commander");
    assert!(matches!(
        unsupported,
        aperture_core::EngineError::UnsupportedCommander {
            card: CardId::GrizzlyBears
        }
    ));

    let multi = Game::new_command_zone(
        1,
        &decks(),
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P0, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect_err("two commanders for one seat must be refused");
    assert!(matches!(
        multi,
        aperture_core::EngineError::InvalidCommanderSetup { .. }
    ));

    let missing = Game::new_command_zone(1, &decks(), &[(P0, CardId::IsamaruHoundOfKonda)])
        .expect_err("every seat needs a commander in this profile");
    assert!(matches!(
        missing,
        aperture_core::EngineError::InvalidCommanderSetup { .. }
    ));
}

#[test]
fn ordinary_practice_games_refuse_a_commander_in_the_library() {
    let decks = vec![
        // Build the list directly: the ordinary text parser refuses Isamaru at
        // the same boundary, while this test separately proves Game::new never
        // accepts a forged/legacy ordinary list containing it.
        DeckList::new(
            "A",
            vec![(CardId::IsamaruHoundOfKonda, 1), (CardId::Forest, 39)],
        ),
        DeckList::parse_practice("B", "40 Forest\n").expect("deck B"),
    ];
    let err = Game::new(3, &decks)
        .expect_err("Isamaru is commander-only in this slice; the legend rule is not implemented");
    assert!(matches!(
        err,
        aperture_core::EngineError::UnsupportedCommander {
            card: CardId::IsamaruHoundOfKonda
        }
    ));
}

fn open_main(game: &mut Game) {
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.active = P0;
    game.state.turn.step = aperture_core::Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
}

fn add_permanent(
    game: &mut Game,
    p: aperture_core::PlayerId,
    card: CardId,
) -> aperture_core::ObjectId {
    let id = aperture_core::ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card,
            owner: p,
            controller: p,
            zone: aperture_core::Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
        },
    );
    id
}

fn command_zone_game() -> Game {
    let mut game = Game::new_command_zone(
        11,
        &decks(),
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect("setup");
    open_main(&mut game);
    game
}

#[test]
fn a_commander_is_castable_from_the_command_zone_and_counts_the_cast() {
    let mut game = command_zone_game();
    add_permanent(&mut game, P0, CardId::Plains);
    let commander = game.state.player(P0).command_zone[0];
    let legal = game.legal_actions();
    assert!(
        legal.iter().any(|action| matches!(
            &action.action,
            aperture_core::Action::CastSpell { card, .. } if *card == commander
        )),
        "the command-zone cast must be a legal action: {legal:?}"
    );
    let events = game
        .submit_action(
            P0,
            aperture_core::Action::CastSpell {
                card: commander,
                targets: Vec::new(),
            },
        )
        .expect("cast");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, aperture_core::Event::Cast { object, .. } if *object != commander)),
        "the commander spell must be a fresh object on the stack"
    );
    assert!(game.state.player(P0).command_zone.is_empty());
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 1);
    assert_eq!(
        game.state.object(game.state.commanders[0].object).zone,
        aperture_core::Zone::Stack
    );
}

#[test]
fn commander_tax_is_two_per_previous_command_zone_cast() {
    let mut game = command_zone_game();
    game.state.commanders[0].casts_from_command_zone = 1;
    add_permanent(&mut game, P0, CardId::Plains);
    add_permanent(&mut game, P0, CardId::Plains);
    let commander = game.state.player(P0).command_zone[0];
    assert!(
        !game.legal_actions().iter().any(|action| matches!(
            &action.action,
            aperture_core::Action::CastSpell { card, .. } if *card == commander
        )),
        "one prior command-zone cast makes Isamaru cost {{W}} + {{2}}, so two Plains are insufficient"
    );
    let err = game
        .submit_action(
            P0,
            aperture_core::Action::CastSpell {
                card: commander,
                targets: Vec::new(),
            },
        )
        .expect_err("the tax makes this cast unpayable");
    assert!(matches!(err, aperture_core::EngineError::CannotPay { .. }));

    add_permanent(&mut game, P0, CardId::Plains);
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: commander,
            targets: Vec::new(),
        },
    )
    .expect("three Plains pay {W} + {2}");
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 2);
    assert!(
        game.state.player(P0).mana.is_empty()
            && game
                .state
                .battlefield(P0)
                .iter()
                .all(|object| object.tapped),
        "the tax is actually paid, not merely checked"
    );
}

#[test]
fn a_dying_commander_gets_one_owner_return_choice_and_declining_is_final() {
    let mut game = command_zone_game();
    add_permanent(&mut game, P0, CardId::Plains);
    let commander = game.state.player(P0).command_zone[0];
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: commander,
            targets: Vec::new(),
        },
    )
    .expect("cast");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass to opponent");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("resolve commander");
    assert_eq!(
        game.state.object(game.state.commanders[0].object).zone,
        aperture_core::Zone::Battlefield
    );

    // Lethal marked damage, then the next SBA check.
    let battlefield_id = game.state.commanders[0].object;
    game.state.objects.get_mut(&battlefield_id).unwrap().damage = 2;
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass into the SBA check");
    let pending = game
        .state
        .pending
        .as_ref()
        .expect("the commander's owner must get the CR 903.9a choice");
    assert_eq!(pending.player(), P0);
    let grave_id = game.state.commanders[0].object;
    assert_ne!(
        grave_id, battlefield_id,
        "a zone change re-keys the commander"
    );
    assert_eq!(
        game.state.object(grave_id).zone,
        aperture_core::Zone::Graveyard
    );
    assert!(game.legal_actions().iter().any(|legal| matches!(
        legal.action,
        aperture_core::Action::AnswerCommanderReturn { .. }
    )));

    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderReturn {
            to_command_zone: false,
        },
    )
    .expect("decline");
    assert_eq!(
        game.state.object(grave_id).zone,
        aperture_core::Zone::Graveyard,
        "a refusal leaves the commander in the graveyard"
    );
    assert!(
        game.state.pending.is_none(),
        "a declined return is not offered again"
    );
    assert!(!game.state.commanders[0].return_eligible);
}

fn add_to_hand(
    game: &mut Game,
    p: aperture_core::PlayerId,
    card: CardId,
) -> aperture_core::ObjectId {
    let id = aperture_core::ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card,
            owner: p,
            controller: p,
            zone: aperture_core::Zone::Hand,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
        },
    );
    game.state.players[p as usize].hand.push(id);
    id
}

#[test]
fn repeat_command_zone_casts_stack_tax_and_a_countered_cast_still_counts() {
    let mut game = command_zone_game();
    add_permanent(&mut game, P0, CardId::Plains);
    let commander = game.state.player(P0).command_zone[0];
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: commander,
            targets: Vec::new(),
        },
    )
    .expect("first cast");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("resolve");
    let battlefield = game.state.commanders[0].object;
    game.state.objects.get_mut(&battlefield).unwrap().damage = 2;
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("sba");
    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    )
    .expect("return to command zone");

    open_main(&mut game);
    // Second command-zone cast: {W} + {2} = three mana.
    add_permanent(&mut game, P0, CardId::Plains);
    add_permanent(&mut game, P0, CardId::Plains);
    add_permanent(&mut game, P0, CardId::Plains);
    let commander = game.state.player(P0).command_zone[0];
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: commander,
            targets: Vec::new(),
        },
    )
    .expect("second cast pays the tax");
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 2);
    let stack_commander = game.state.commanders[0].object;

    // Countering the cast does not undo the cast (CR 903.8): count stays 2.
    add_to_hand(&mut game, P1, CardId::Counterspell);
    add_permanent(&mut game, P1, CardId::Island);
    add_permanent(&mut game, P1, CardId::Island);
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(
        P1,
        aperture_core::Action::CastSpell {
            card: hand_card_id(&game, P1, CardId::Counterspell),
            targets: vec![aperture_core::Target::Spell(stack_commander)],
        },
    )
    .expect("counter the commander");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("resolve counter");
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 2);
    assert!(
        matches!(
            game.state.pending,
            Some(aperture_core::Pending::CommanderReturn { .. })
        ),
        "expected return choice, got {:?}; commander object {:?} zone {:?}",
        game.state.pending,
        game.state.commanders[0].object,
        game.state.object(game.state.commanders[0].object).zone
    );
}

fn hand_card_id(game: &Game, p: aperture_core::PlayerId, card: CardId) -> aperture_core::ObjectId {
    game.state
        .player(p)
        .hand
        .iter()
        .copied()
        .find(|id| game.state.object(*id).card == card)
        .expect("card in hand")
}

#[test]
fn an_unpayable_or_unauthorized_command_zone_cast_does_not_increment_or_mutate() {
    let mut game = command_zone_game();
    let commander = game.state.player(P0).command_zone[0];
    let revision = game.state.revision;
    let decision = game.state.decision;
    let err = game
        .submit_action(
            P0,
            aperture_core::Action::CastSpell {
                card: commander,
                targets: Vec::new(),
            },
        )
        .expect_err("no mana");
    assert!(matches!(err, aperture_core::EngineError::CannotPay { .. }));
    assert_eq!(game.state.revision, revision);
    assert_eq!(game.state.decision, decision);
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 0);
    assert_eq!(game.state.player(P0).command_zone, vec![commander]);

    add_permanent(&mut game, P0, CardId::Plains);
    let err = game
        .submit_action(
            P1,
            aperture_core::Action::CastSpell {
                card: commander,
                targets: Vec::new(),
            },
        )
        .expect_err("P1 does not own that commander");
    assert!(matches!(
        err,
        aperture_core::EngineError::NotYourDecision { .. }
    ));
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 0);

    let stale = aperture_core::Submission {
        player: P0,
        revision: game.state.revision + 1,
        decision: game.state.decision,
        action: aperture_core::Action::CastSpell {
            card: commander,
            targets: Vec::new(),
        },
    };
    assert!(matches!(
        game.submit(&stale),
        Err(aperture_core::EngineError::StaleRevision { .. })
    ));
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 0);
}

#[test]
fn a_commander_cast_from_hand_does_not_add_tax() {
    let mut game = command_zone_game();
    // Model a refused CR 903.9b return: the commander is in hand, not the
    // command zone. Casting from hand is ordinary and untaxed.
    let id = game.state.commanders[0].object;
    game.state.players[P0 as usize].command_zone.clear();
    game.state.objects.get_mut(&id).unwrap().zone = aperture_core::Zone::Hand;
    game.state.players[P0 as usize].hand.push(id);
    add_permanent(&mut game, P0, CardId::Plains);
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: id,
            targets: Vec::new(),
        },
    )
    .expect("hand cast");
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 0);
    assert_eq!(
        game.state.object(game.state.commanders[0].object).zone,
        aperture_core::Zone::Stack
    );
}

fn put_commander_on_battlefield(game: &mut Game) -> aperture_core::ObjectId {
    add_permanent(game, P0, CardId::Plains);
    let commander = game.state.player(P0).command_zone[0];
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: commander,
            targets: Vec::new(),
        },
    )
    .expect("cast commander");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("resolve");
    game.state.commanders[0].object
}

#[test]
fn unsummon_suspends_before_the_move_and_declining_proceeds_to_hand() {
    let mut game = command_zone_game();
    let battlefield = put_commander_on_battlefield(&mut game);
    add_permanent(&mut game, P0, CardId::Island);
    add_to_hand(&mut game, P0, CardId::Unsummon);
    let unsummon = hand_card_id(&game, P0, CardId::Unsummon);
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: unsummon,
            targets: vec![aperture_core::Target::Permanent(battlefield)],
        },
    )
    .expect("cast Unsummon");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("resolve Unsummon");

    let pending = game
        .state
        .pending
        .as_ref()
        .expect("the owner must choose the CR 903.9b replacement before the move");
    assert_eq!(pending.player(), P0);
    assert!(
        matches!(pending, aperture_core::Pending::CommanderReplacement { destination: aperture_core::Zone::Hand, source, .. } if *source == unsummon),
        "the choice must carry the exact source: {pending:?}"
    );
    assert_eq!(
        game.state.object(battlefield).zone,
        aperture_core::Zone::Battlefield,
        "the commander must not move before the owner answers"
    );
    assert_eq!(game.state.commanders[0].object, battlefield);
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 1);

    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderReplacement {
            to_command_zone: false,
        },
    )
    .expect("decline");
    let hand_object = game.state.commanders[0].object;
    assert_ne!(hand_object, battlefield);
    assert_eq!(
        game.state.object(hand_object).zone,
        aperture_core::Zone::Hand
    );
    assert!(
        game.state.player(P0).hand.contains(&hand_object),
        "the refused replacement proceeds to hand exactly once"
    );
    assert!(game.state.pending.is_none());
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 1);
}

#[test]
fn swords_exiles_then_offers_return_with_controller_life_gain() {
    let mut game = command_zone_game();
    put_commander_on_battlefield(&mut game);
    add_permanent(&mut game, P0, CardId::Plains);
    add_to_hand(&mut game, P0, CardId::SwordsToPlowshares);
    let swords = hand_card_id(&game, P0, CardId::SwordsToPlowshares);
    let battlefield = game.state.commanders[0].object;
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: swords,
            targets: vec![aperture_core::Target::Permanent(battlefield)],
        },
    )
    .expect("cast Swords");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("resolve Swords");
    assert_eq!(game.state.player(P0).life, 22, "life gain after exile");
    let exiled = game.state.commanders[0].object;
    assert_eq!(game.state.object(exiled).zone, aperture_core::Zone::Exile);
    assert!(matches!(
        game.state.pending,
        Some(aperture_core::Pending::CommanderReturn { .. })
    ));
    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    )
    .expect("return");
    let returned = game.state.commanders[0].object;
    assert_eq!(
        game.state.object(returned).zone,
        aperture_core::Zone::Command
    );
    assert_eq!(game.state.player(P0).command_zone, vec![returned]);
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 1);
}

#[test]
fn time_ebb_suspends_before_the_library_move_and_declining_puts_it_on_top() {
    let mut game = command_zone_game();
    put_commander_on_battlefield(&mut game);
    add_permanent(&mut game, P0, CardId::Island);
    add_permanent(&mut game, P0, CardId::Island);
    add_permanent(&mut game, P0, CardId::Island);
    add_to_hand(&mut game, P0, CardId::TimeEbb);
    let ebb = hand_card_id(&game, P0, CardId::TimeEbb);
    let battlefield = game.state.commanders[0].object;
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: ebb,
            targets: vec![aperture_core::Target::Permanent(battlefield)],
        },
    )
    .expect("cast Time Ebb");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("resolve");
    assert!(matches!(
        game.state.pending,
        Some(aperture_core::Pending::CommanderReplacement {
            destination: aperture_core::Zone::Library,
            ..
        })
    ));
    assert_eq!(
        game.state.object(battlefield).zone,
        aperture_core::Zone::Battlefield
    );
    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderReplacement {
            to_command_zone: false,
        },
    )
    .expect("decline");
    let top = game.state.commanders[0].object;
    assert_eq!(game.state.object(top).zone, aperture_core::Zone::Library);
    assert_eq!(game.state.player(P0).library.first(), Some(&top));
}

fn put_commander_on_top_of_library(game: &mut Game) {
    put_commander_on_battlefield(game);
    add_permanent(game, P0, CardId::Island);
    add_permanent(game, P0, CardId::Island);
    add_permanent(game, P0, CardId::Island);
    add_to_hand(game, P0, CardId::TimeEbb);
    let ebb = hand_card_id(game, P0, CardId::TimeEbb);
    let battlefield = game.state.commanders[0].object;
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: ebb,
            targets: vec![aperture_core::Target::Permanent(battlefield)],
        },
    )
    .expect("cast Time Ebb");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("resolve");
    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderReplacement {
            to_command_zone: false,
        },
    )
    .expect("decline library replacement");
}

#[test]
fn draw_step_suspends_before_a_commander_moves_from_library_to_hand() {
    let mut game = command_zone_game();
    put_commander_on_top_of_library(&mut game);
    let top = game.state.player(P0).library[0];
    assert_eq!(top, game.state.commanders[0].object);
    game.state.turn.number = 2;
    game.state.turn.step = aperture_core::Step::Upkeep;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass upkeep");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("enter draw step");
    assert!(
        matches!(
            game.state.pending,
            Some(aperture_core::Pending::CommanderDraw {
                resume: aperture_core::state::DrawResume::Step,
                ..
            })
        ),
        "the draw step must suspend before the move: {:?}",
        game.state.pending
    );
    assert_eq!(game.state.player(P0).library[0], top);
    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderDraw {
            to_command_zone: true,
        },
    )
    .expect("accept");
    let command = game.state.commanders[0].object;
    assert_ne!(command, top);
    assert_eq!(
        game.state.object(command).zone,
        aperture_core::Zone::Command
    );
    assert!(game.state.player(P0).hand.iter().all(|id| *id != command));
    assert_eq!(game.state.turn.step, aperture_core::Step::Draw);
    assert_eq!(game.state.turn.priority, Some(P0));
}

#[test]
fn divination_draw_two_continues_after_a_commander_draw_replacement() {
    let mut game = command_zone_game();
    put_commander_on_top_of_library(&mut game);
    open_main(&mut game);
    let top = game.state.player(P0).library[0];
    assert_eq!(top, game.state.commanders[0].object);
    let second = game.state.player(P0).library[1];
    add_permanent(&mut game, P0, CardId::Island);
    add_permanent(&mut game, P0, CardId::Island);
    add_permanent(&mut game, P0, CardId::Island);
    add_to_hand(&mut game, P0, CardId::Divination);
    let divination = hand_card_id(&game, P0, CardId::Divination);
    let hand_before = game.state.player(P0).hand.len();
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: divination,
            targets: Vec::new(),
        },
    )
    .expect("cast Divination");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("resolve Divination");
    assert!(matches!(
        game.state.pending,
        Some(aperture_core::Pending::CommanderDraw { remaining: 1, .. })
    ));
    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderDraw {
            to_command_zone: true,
        },
    )
    .expect("accept replacement");
    let command = game.state.commanders[0].object;
    assert_eq!(
        game.state.object(command).zone,
        aperture_core::Zone::Command
    );
    assert!(
        game.state.player(P0).hand.contains(&second),
        "the second Divination draw must still happen exactly once"
    );
    assert_eq!(
        game.state.player(P0).hand.len(),
        hand_before - 1 + 1,
        "Divination is cast, commander replaced, second card drawn once"
    );
    assert_eq!(
        game.state.object(divination).zone,
        aperture_core::Zone::Graveyard
    );
    assert!(game.state.pending.is_none());
}

#[test]
fn command_zone_replay_and_save_round_trip_the_explicit_setup() {
    use aperture_core::{AiSettings, CommanderSetup, Replay, SessionSave};
    let decks = decks();
    let first = Game::new_command_zone(
        21,
        &decks,
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect("setup")
    .state
    .first_player;
    let mut replay = Replay::new_command_zone(
        21,
        first,
        decks.clone(),
        vec![
            CommanderSetup {
                seat: P0,
                card: CardId::IsamaruHoundOfKonda,
            },
            CommanderSetup {
                seat: P1,
                card: CardId::IsamaruHoundOfKonda,
            },
        ],
        AiSettings::default(),
    )
    .expect("replay");
    let (game, _) = replay.run().expect("rebuild command-zone game");
    replay.final_digest = game.digest();
    let (rebuilt, _) = replay.run().expect("replay with digest");
    assert_eq!(rebuilt.digest(), game.digest());
    assert_eq!(rebuilt.state.commanders.len(), 2);
    assert!(rebuilt.state.command_zone_practice);

    let save = SessionSave::capture(&game, replay.clone(), Vec::new(), false);
    let json = save.to_json();
    let loaded = SessionSave::from_json(&json).expect("save round trip");
    assert_eq!(loaded.game.commanders, game.state.commanders);
    assert_eq!(loaded.replay.commanders.len(), 2);

    let mut tampered = save.clone();
    tampered.replay.commanders.clear();
    let err = SessionSave::from_json(&tampered.to_json())
        .expect_err("missing commander setup must be rejected");
    assert!(err.to_string().contains("setup") || err.to_string().contains("seat"));
}

#[test]
fn a_departing_commander_owner_is_purged_without_touching_other_seats() {
    let decks3 = vec![
        DeckList::parse_practice("A", "40 Forest\n").expect("A"),
        DeckList::parse_practice("B", "40 Forest\n").expect("B"),
        DeckList::parse_practice("C", "40 Forest\n").expect("C"),
    ];
    let mut game = Game::new_command_zone(
        31,
        &decks3,
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
            (2, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect("3-seat command-zone setup");
    let p0_commander = game
        .state
        .commanders
        .iter()
        .find(|c| c.owner == P0)
        .unwrap()
        .object;
    let p2_commander = game
        .state
        .commanders
        .iter()
        .find(|c| c.owner == 2)
        .unwrap()
        .object;
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.step = aperture_core::Step::PrecombatMain;
    game.state.turn.active = P0;
    game.state.turn.priority = Some(P0);
    game.state.players[P1 as usize].life = 0;
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("SBA removes P1");
    assert!(game.state.player(P1).lost);
    assert!(game.state.player(P1).command_zone.is_empty());
    assert!(game.state.commanders.iter().all(|c| c.owner != P1));
    assert_eq!(
        game.state
            .commanders
            .iter()
            .find(|c| c.owner == P0)
            .unwrap()
            .object,
        p0_commander
    );
    assert_eq!(
        game.state
            .commanders
            .iter()
            .find(|c| c.owner == 2)
            .unwrap()
            .object,
        p2_commander
    );
    game.state
        .validate()
        .expect("departed commander state stays valid");
}

fn three_seat_command_zone_with_graveyard_commanders() -> Game {
    let decks3 = vec![
        DeckList::parse_practice("A", "40 Forest\n").expect("A"),
        DeckList::parse_practice("B", "40 Forest\n").expect("B"),
        DeckList::parse_practice("C", "40 Forest\n").expect("C"),
    ];
    let mut game = Game::new_command_zone(
        41,
        &decks3,
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
            (2, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect("setup");
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.active = P1;
    game.state.turn.step = aperture_core::Step::PrecombatMain;
    game.state.turn.priority = Some(P1);
    game.state.turn.passed = 0;
    for seat in [P0, P1, 2u8] {
        let registry = game
            .state
            .commanders
            .iter_mut()
            .find(|registry| registry.owner == seat)
            .expect("registry");
        let id = registry.object;
        registry.return_eligible = true;
        game.state.players[seat as usize]
            .command_zone
            .retain(|c| *c != id);
        game.state.players[seat as usize].graveyard.push(id);
        game.state.objects.get_mut(&id).unwrap().zone = aperture_core::Zone::Graveyard;
    }
    game
}

#[test]
fn simultaneous_commander_returns_wait_for_every_owner_then_apply_apnap_batch() {
    let mut game = three_seat_command_zone_with_graveyard_commanders();
    let before: Vec<(aperture_core::CommanderId, aperture_core::ObjectId)> = game
        .state
        .commanders
        .iter()
        .map(|registry| (registry.id, registry.object))
        .collect();
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("SBA");
    // Active is P1, so APNAP is P1, P2, P0 even though P0 has the lowest seat.
    let pending = game.state.pending.as_ref().expect("return queue");
    match pending {
        aperture_core::Pending::CommanderReturn { order, cursor, .. } => {
            assert_eq!(cursor, &0);
            assert_eq!(order[0].player, P1);
            assert_eq!(
                order.iter().map(|entry| entry.player).collect::<Vec<_>>(),
                vec![P1, 2, P0]
            );
        }
        other => panic!("unexpected pending {other:?}"),
    }
    let p1_grave = game
        .state
        .commanders
        .iter()
        .find(|registry| registry.owner == P1)
        .unwrap()
        .object;
    game.submit_action(
        P1,
        aperture_core::Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    )
    .expect("P1 accepts");
    assert_eq!(
        game.state.try_object(p1_grave).map(|object| object.zone),
        Some(aperture_core::Zone::Graveyard),
        "the first accepted commander must remain in the graveyard until the batch finishes"
    );
    let pending = game.state.pending.as_ref().expect("P2 is next");
    assert_eq!(pending.player(), 2);

    let p2_grave = game
        .state
        .commanders
        .iter()
        .find(|registry| registry.owner == 2)
        .unwrap()
        .object;
    game.submit_action(
        2,
        aperture_core::Action::AnswerCommanderReturn {
            to_command_zone: false,
        },
    )
    .expect("P2 refuses");
    assert_eq!(
        game.state.try_object(p1_grave).map(|object| object.zone),
        Some(aperture_core::Zone::Graveyard),
        "a refusal by another owner must not apply the earlier acceptance"
    );
    assert_eq!(
        game.state.object(p2_grave).zone,
        aperture_core::Zone::Graveyard
    );

    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    )
    .expect("P0 accepts");
    assert!(game.state.pending.is_none());
    for (commander, old_object) in before {
        let registry = game
            .state
            .commanders
            .iter()
            .find(|registry| registry.id == commander)
            .unwrap();
        if registry.owner == 2 {
            assert_eq!(registry.object, old_object);
            assert_eq!(
                game.state.object(old_object).zone,
                aperture_core::Zone::Graveyard
            );
            assert!(!registry.return_eligible);
        } else {
            assert_ne!(registry.object, old_object, "accepted moves re-key");
            assert_eq!(
                game.state.object(registry.object).zone,
                aperture_core::Zone::Command
            );
        }
        assert_eq!(registry.casts_from_command_zone, 0);
    }
    game.state.validate().expect("batch state valid");
}

fn queued_return_game() -> Game {
    let mut game = three_seat_command_zone_with_graveyard_commanders();
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("SBA");
    assert!(matches!(
        game.state.pending,
        Some(aperture_core::Pending::CommanderReturn { .. })
    ));
    game
}

#[test]
fn malformed_commander_return_queues_are_rejected_not_repaired() {
    let game = queued_return_game();
    let good = match game.state.pending.clone().unwrap() {
        aperture_core::Pending::CommanderReturn {
            order,
            cursor,
            answers,
        } => (order, cursor, answers),
        _ => unreachable!(),
    };

    let mut reordered = game.state.clone();
    if let Some(aperture_core::Pending::CommanderReturn { order, .. }) = &mut reordered.pending {
        order.swap(1, 2);
    }
    assert!(
        reordered.validate().is_err(),
        "reordered APNAP queue must be rejected"
    );

    let mut duplicated = game.state.clone();
    if let Some(aperture_core::Pending::CommanderReturn { order, .. }) = &mut duplicated.pending {
        order[1] = order[0].clone();
    }
    assert!(
        duplicated.validate().is_err(),
        "duplicate commander must be rejected"
    );

    let mut omitted = game.state.clone();
    if let Some(aperture_core::Pending::CommanderReturn { order, .. }) = &mut omitted.pending {
        order.pop();
    }
    assert!(
        omitted.validate().is_err(),
        "omitted armed commander must be rejected"
    );

    let mut stale = game.state.clone();
    if let Some(aperture_core::Pending::CommanderReturn { order, .. }) = &mut stale.pending {
        order[0].object = aperture_core::ObjectId(999_999);
    }
    assert!(
        stale.validate().is_err(),
        "stale current object must be rejected"
    );

    let mut bad_cursor = game.state.clone();
    if let Some(aperture_core::Pending::CommanderReturn {
        cursor, answers, ..
    }) = &mut bad_cursor.pending
    {
        *cursor = 1;
        answers.clear();
    }
    assert!(
        bad_cursor.validate().is_err(),
        "cursor/answers mismatch must be rejected"
    );

    let mut canonical = game.state.clone();
    if let Some(aperture_core::Pending::CommanderReturn {
        order,
        cursor,
        answers,
    }) = &mut canonical.pending
    {
        assert_eq!(*order, good.0);
        assert_eq!(*cursor, good.1);
        assert_eq!(*answers, good.2);
    }
    canonical.validate().expect("canonical queue remains valid");
}

#[test]
fn commander_zone_transitions_reset_controller_and_object_state() {
    let mut game = command_zone_game();
    let battlefield = put_commander_on_battlefield(&mut game);
    {
        let object = game.state.objects.get_mut(&battlefield).unwrap();
        object.controller = P1;
        object.tapped = true;
        object.pump_power = 4;
        object.pump_toughness = 4;
        object.plus_one_counters = 3;
        object.damage = 6;
    }
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("SBA");
    let grave = game.state.commanders[0].object;
    assert_ne!(grave, battlefield);
    let object = game.state.object(grave);
    assert_eq!(object.zone, aperture_core::Zone::Graveyard);
    assert_eq!(
        object.controller, P0,
        "owner is authoritative outside the battlefield"
    );
    assert!(!object.tapped);
    assert_eq!(object.damage, 0);
    assert_eq!(object.pump_power, 0);
    assert_eq!(object.pump_toughness, 0);
    assert_eq!(object.plus_one_counters, 0);
    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    )
    .expect("return");
    let command = game.state.commanders[0].object;
    let object = game.state.object(command);
    assert_eq!(object.zone, aperture_core::Zone::Command);
    assert_eq!(object.controller, P0);
    assert_eq!(object.plus_one_counters, 0);
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 1);
}

#[test]
fn malformed_commander_return_bounds_are_typed_not_panics() {
    let game = queued_return_game();
    for (cursor, order) in [
        (0usize, Vec::new()),
        (
            3usize,
            match game.state.pending.clone().unwrap() {
                aperture_core::Pending::CommanderReturn { order, .. } => order,
                _ => unreachable!(),
            },
        ),
    ] {
        let mut malformed = game.state.clone();
        if let Some(aperture_core::Pending::CommanderReturn {
            order: pending_order,
            cursor: pending_cursor,
            answers,
        }) = &mut malformed.pending
        {
            *pending_order = order;
            *pending_cursor = cursor;
            answers.clear();
        }
        let result = std::panic::catch_unwind(|| malformed.validate());
        assert!(
            result.is_ok(),
            "validate must not panic on a malformed return queue"
        );
        assert!(
            result.unwrap().is_err(),
            "malformed return queue must be rejected"
        );
    }
}

fn replacement_pending_game() -> Game {
    let mut game = command_zone_game();
    put_commander_on_battlefield(&mut game);
    add_permanent(&mut game, P0, CardId::Island);
    add_to_hand(&mut game, P0, CardId::Unsummon);
    let unsummon = hand_card_id(&game, P0, CardId::Unsummon);
    let battlefield = game.state.commanders[0].object;
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: unsummon,
            targets: vec![aperture_core::Target::Permanent(battlefield)],
        },
    )
    .expect("cast");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("resolve");
    assert!(matches!(
        game.state.pending,
        Some(aperture_core::Pending::CommanderReplacement { .. })
    ));
    game
}

#[test]
fn forged_replacement_source_and_cursor_are_rejected() {
    let game = replacement_pending_game();
    for mutate in 0..4 {
        let mut malformed = game.state.clone();
        if let Some(aperture_core::Pending::CommanderReplacement {
            source,
            cursor,
            object,
            destination,
            ..
        }) = &mut malformed.pending
        {
            match mutate {
                0 => *source = aperture_core::ObjectId(999_999),
                1 => *cursor = 99,
                2 => *object = aperture_core::ObjectId(999_998),
                _ => *destination = aperture_core::Zone::Command,
            }
        }
        let result = std::panic::catch_unwind(|| malformed.validate());
        assert!(result.is_ok(), "validate must not panic");
        assert!(
            result.unwrap().is_err(),
            "forged replacement case {mutate} must be rejected"
        );
    }
    game.state
        .validate()
        .expect("canonical replacement pending is valid");
}

#[test]
fn commander_view_redacts_hidden_zones_for_every_viewer() {
    let mut game = command_zone_game();
    let p1_view = game.view_for(P1, &[]);
    let view = p1_view.commanders.iter().find(|c| c.owner == P0).unwrap();
    assert_eq!(view.zone, Some(aperture_core::Zone::Command));
    assert_eq!(view.tax, 0);
    assert!(!p1_view.opponents[0].command_zone.is_empty());

    // Decline Unsummon: the commander is now in P0's hidden hand.
    put_commander_on_battlefield(&mut game);
    add_permanent(&mut game, P0, CardId::Island);
    add_to_hand(&mut game, P0, CardId::Unsummon);
    let unsummon = hand_card_id(&game, P0, CardId::Unsummon);
    let battlefield = game.state.commanders[0].object;
    game.submit_action(
        P0,
        aperture_core::Action::CastSpell {
            card: unsummon,
            targets: vec![aperture_core::Target::Permanent(battlefield)],
        },
    )
    .expect("cast");
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("resolve");
    game.submit_action(
        P0,
        aperture_core::Action::AnswerCommanderReplacement {
            to_command_zone: false,
        },
    )
    .expect("decline");
    let hand_object = game.state.commanders[0].object;
    for viewer in [P0, P1] {
        let view = game.view_for(viewer, &[]);
        let commander = view.commanders.iter().find(|c| c.owner == P0).unwrap();
        assert_eq!(
            commander.zone, None,
            "a commander in hand must be redacted for every viewer"
        );
        assert!(
            view.own_command_zone
                .iter()
                .all(|card| card.id != hand_object),
            "hidden hand object id must not leak through the command zone"
        );
        if viewer == P0 {
            assert!(view.own_hand.iter().any(|card| card.id == hand_object));
        }
    }
}

#[test]
fn a_declined_commander_draw_emits_one_real_draw_event() {
    let mut game = command_zone_game();
    put_commander_on_top_of_library(&mut game);
    game.state.turn.number = 2;
    game.state.turn.step = aperture_core::Step::Upkeep;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("pass");
    game.submit_action(P1, aperture_core::Action::Pass)
        .expect("draw step");
    let events = game
        .submit_action(
            P0,
            aperture_core::Action::AnswerCommanderDraw {
                to_command_zone: false,
            },
        )
        .expect("decline");
    let hand_object = game.state.commanders[0].object;
    let draws: Vec<_> = events
        .iter()
        .filter(|event| matches!(event, aperture_core::Event::Draw { object, .. } if *object == hand_object))
        .collect();
    assert_eq!(
        draws.len(),
        1,
        "a refused replacement is a real draw: {events:?}"
    );
    assert_eq!(
        game.state.object(hand_object).zone,
        aperture_core::Zone::Hand
    );
}

#[test]
fn terminal_sba_wins_immediately_without_an_irrelevant_return_choice() {
    let decks3 = vec![
        DeckList::parse_practice("A", "40 Forest\n").expect("A"),
        DeckList::parse_practice("B", "40 Forest\n").expect("B"),
        DeckList::parse_practice("C", "40 Forest\n").expect("C"),
    ];
    let mut game = Game::new_command_zone(
        51,
        &decks3,
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
            (2, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect("setup");
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.active = P0;
    game.state.turn.step = aperture_core::Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    game.state.players[P1 as usize].life = 0;
    game.state.players[2].life = 0;
    let commander = game
        .state
        .commanders
        .iter()
        .find(|registry| registry.owner == P0)
        .unwrap()
        .object;
    game.state.players[P0 as usize]
        .command_zone
        .retain(|c| *c != commander);
    let object = game.state.objects.get_mut(&commander).unwrap();
    object.zone = aperture_core::Zone::Battlefield;
    object.damage = 2;
    game.submit_action(P0, aperture_core::Action::Pass)
        .expect("SBA");
    assert!(
        game.state.outcome.is_some(),
        "the last live seat wins immediately"
    );
    assert!(
        game.state.pending.is_none(),
        "an irrelevant commander return choice must not defer GameOver"
    );
}

#[test]
fn opt_and_consider_commander_draws_validate_after_the_first_instruction() {
    use aperture_core::{Action, DrawResume, Pending};
    for card in [CardId::Opt, CardId::Consider] {
        for accept in [false, true] {
            let mut game = command_zone_game();
            put_commander_on_top_of_library(&mut game);
            add_permanent(&mut game, P0, CardId::Island);
            let spell = add_to_hand(&mut game, P0, card);
            game.submit_action(
                P0,
                Action::CastSpell {
                    card: spell,
                    targets: vec![],
                },
            )
            .unwrap();
            game.submit_action(P0, Action::Pass).unwrap();
            game.submit_action(P1, Action::Pass).unwrap();
            game.submit_action(P0, Action::OrderCard { to_top: true })
                .unwrap();
            assert!(matches!(
                game.state.pending,
                Some(Pending::CommanderDraw {
                    remaining: 0,
                    resume: DrawResume::Effect { cursor: 2, .. },
                    ..
                })
            ));
            game.state
                .validate()
                .expect("instruction index is not the number of consumed draws");
            game.submit_action(
                P0,
                Action::AnswerCommanderDraw {
                    to_command_zone: accept,
                },
            )
            .unwrap();
            assert!(game.state.pending.is_none());
            assert!(game.state.stack.is_empty());
            game.state.validate().unwrap();
        }
    }
}

#[test]
fn injected_departure_retains_the_unanswered_commander_return_tail() {
    use aperture_core::{Action, Pending};
    let mut game = three_seat_command_zone_with_graveyard_commanders();
    game.state.turn.active = P0;
    game.state.turn.priority = Some(P0);
    game.submit_action(P0, Action::Pass).unwrap();
    // Explicit sandbox fixture: supported effects cannot change life mid-choice.
    game.state.players[P1 as usize].life = 0;
    game.submit_action(
        P0,
        Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    )
    .unwrap();
    game.state
        .validate()
        .expect("unanswered entries survive a departure");
    assert_eq!(game.decision_owner(), Some(2));
    assert!(matches!(
        game.state.pending,
        Some(Pending::CommanderReturn { cursor: 1, .. })
    ));
    game.submit_action(
        2,
        Action::AnswerCommanderReturn {
            to_command_zone: false,
        },
    )
    .unwrap();
    game.state.validate().unwrap();
    assert!(game.state.pending.is_none());
}

#[test]
fn final_commander_return_answer_restores_priority_without_advancing_the_step() {
    use aperture_core::{Action, Pending};
    let mut game = three_seat_command_zone_with_graveyard_commanders();
    let step = game.state.turn.step;
    game.submit_action(P1, Action::Pass).unwrap();
    while matches!(game.state.pending, Some(Pending::CommanderReturn { .. })) {
        let owner = game.decision_owner().unwrap();
        game.submit_action(
            owner,
            Action::AnswerCommanderReturn {
                to_command_zone: false,
            },
        )
        .unwrap();
    }
    assert_eq!(
        game.state.turn.step, step,
        "SBA choices do not pass priority or end the phase"
    );
    assert_eq!(game.state.turn.priority, Some(P1));
}

#[test]
fn all_lost_command_zone_save_preserves_its_original_profile() {
    use aperture_core::{Action, SessionSave};
    let save = valid_command_zone_save();
    let mut game = save.game();
    open_main(&mut game);
    game.state.players[0].life = 0;
    game.state.players[1].life = 0;
    let log = game.submit_action(P0, Action::Pass).unwrap();
    game.state.validate().unwrap();
    assert!(game.state.is_over());
    let terminal = SessionSave::capture(&game, save.replay, log, true);
    SessionSave::from_json(&terminal.to_json())
        .expect("all-lost does not change the configured profile");
}

fn sandbox_save_for_pending(game: &Game) -> aperture_core::SessionSave {
    use aperture_core::{AiSettings, CommanderSetup, Replay, SessionSave};
    let replay = Replay::new_command_zone(
        game.state.seed,
        game.state.first_player,
        decks(),
        vec![
            CommanderSetup {
                seat: P0,
                card: CardId::IsamaruHoundOfKonda,
            },
            CommanderSetup {
                seat: P1,
                card: CardId::IsamaruHoundOfKonda,
            },
        ],
        AiSettings::default(),
    )
    .unwrap();
    SessionSave::capture(game, replay, vec![], true)
}

#[test]
fn replacement_save_is_bound_to_the_producer_destination_and_target() {
    use aperture_core::{Pending, SessionSave, Zone};
    let game = replacement_pending_game();
    SessionSave::from_json(&sandbox_save_for_pending(&game).to_json()).unwrap();
    for mutate in 0..2 {
        let mut forged = Game {
            state: game.state.clone(),
        };
        let other = forged.state.commanders[1].clone();
        if let Some(Pending::CommanderReplacement {
            destination,
            player,
            commander,
            object,
            from_zone,
            ..
        }) = &mut forged.state.pending
        {
            if mutate == 0 {
                *destination = Zone::Library;
            } else {
                *player = other.owner;
                *commander = other.id;
                *object = other.object;
                *from_zone = Zone::Command;
            }
        }
        let text = sandbox_save_for_pending(&forged).to_json();
        assert!(
            SessionSave::from_json(&text).is_err(),
            "producer-forged case {mutate} accepted despite a valid digest"
        );
    }
}

#[test]
fn a_draw_step_save_cannot_claim_additional_effect_draws() {
    use aperture_core::{Action, Pending, SessionSave, Step};
    let mut game = command_zone_game();
    put_commander_on_top_of_library(&mut game);
    game.state.turn.number = 2;
    game.state.turn.step = Step::Upkeep;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    game.submit_action(P0, Action::Pass).unwrap();
    game.submit_action(P1, Action::Pass).unwrap();
    SessionSave::from_json(&sandbox_save_for_pending(&game).to_json()).unwrap();
    if let Some(Pending::CommanderDraw { remaining, .. }) = &mut game.state.pending {
        *remaining = 255;
    } else {
        panic!("expected commander draw step");
    }
    assert!(SessionSave::from_json(&sandbox_save_for_pending(&game).to_json()).is_err());
}

fn valid_command_zone_save() -> aperture_core::SessionSave {
    use aperture_core::{AiSettings, CommanderSetup, Replay, SessionSave};
    let decks = decks();
    let game = Game::new_command_zone(
        61,
        &decks,
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect("setup");
    let replay = Replay::new_command_zone(
        61,
        game.state.first_player,
        decks,
        vec![
            CommanderSetup {
                seat: P0,
                card: CardId::IsamaruHoundOfKonda,
            },
            CommanderSetup {
                seat: P1,
                card: CardId::IsamaruHoundOfKonda,
            },
        ],
        AiSettings::default(),
    )
    .expect("replay");
    SessionSave::capture(&game, replay, Vec::new(), true)
}

fn reseal_mutated_save(mut value: serde_json::Value) -> String {
    let state: aperture_core::GameState =
        serde_json::from_value(value["game"].clone()).expect("state");
    value["digest"] = serde_json::json!(aperture_core::digest::digest(&state));
    value.to_string()
}

#[test]
fn sandbox_save_container_and_identity_corruption_is_rejected() {
    let save = valid_command_zone_save();
    let canonical = serde_json::to_value(&save).expect("json");

    let mut duplicate_container = canonical.clone();
    let command = duplicate_container["game"]["players"][0]["command_zone"][0].clone();
    duplicate_container["game"]["players"][0]["command_zone"]
        .as_array_mut()
        .unwrap()
        .push(command);
    let err =
        aperture_core::SessionSave::from_json(&reseal_mutated_save(duplicate_container.clone()))
            .expect_err("duplicate command-zone container entry must be rejected");
    eprintln!("duplicate error: {err}");
    assert!(err.to_string().contains("invalid") || err.to_string().contains("state"));

    let mut missing_container = canonical.clone();
    missing_container["game"]["players"][0]["command_zone"] = serde_json::json!([]);
    aperture_core::SessionSave::from_json(&reseal_mutated_save(missing_container.clone()))
        .expect_err("missing command-zone container entry must be rejected");

    let mut key_mismatch = canonical.clone();
    key_mismatch["game"]["objects"]["1"]["id"] = serde_json::json!(999_999);
    aperture_core::SessionSave::from_json(&reseal_mutated_save(key_mismatch.clone()))
        .expect_err("object id versus map key mismatch must be rejected");

    let mut reused_allocator = canonical.clone();
    reused_allocator["game"]["next_object"] = serde_json::json!(1);
    aperture_core::SessionSave::from_json(&reseal_mutated_save(reused_allocator.clone()))
        .expect_err("reused next_object allocator must be rejected");

    aperture_core::SessionSave::from_json(&reseal_mutated_save(canonical.clone()))
        .expect("canonical save still valid");
}

#[test]
fn ordinary_practice_parser_refuses_the_commander_only_card_with_reason() {
    let err = DeckList::parse_practice("bad", "1 Isamaru, Hound of Konda\n39 Forest\n")
        .expect_err("Isamaru must not parse as an ordinary practice library card");
    let text = err.to_lowercase();
    assert!(
        text.contains("commander-only") || text.contains("commander only"),
        "{err}"
    );
    assert!(text.contains("legend"), "{err}");
}
