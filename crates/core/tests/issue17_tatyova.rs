//! Issue 17 slice 3: Tatyova landfall and Jaddi Offshoot defender.

mod common;

use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn tatyova_game(seed: u64) -> Game {
    let deck = DeckList::parse_practice("landfall", "40 Forest\n").expect("deck");
    let mut game = Game::new_commander_combat(
        seed,
        &[deck.clone(), deck],
        &[
            (P0, card("Tatyova, Benthic Druid")),
            (P1, card("Tatyova, Benthic Druid")),
        ],
    )
    .expect("commander game");
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    game
}

fn add_permanent(game: &mut Game, controller: PlayerId, card: CardId) -> ObjectId {
    let id = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card,
            owner: controller,
            controller,
            zone: Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: Default::default(),
        },
    );
    id
}

fn land_in_hand(game: &mut Game, player: PlayerId) -> ObjectId {
    let id = game.state.players[player as usize]
        .library
        .pop()
        .expect("land");
    let object = game.state.objects.get_mut(&id).unwrap();
    object.card = CardId::Forest;
    object.zone = Zone::Hand;
    game.state.players[player as usize].hand.push(id);
    id
}

#[test]
fn catalog_def_and_catalog_all_agree_on_jaddi_defender() {
    let id = card("Jaddi Offshoot");
    let from_all = catalog::all()
        .iter()
        .find(|def| def.id == id)
        .expect("Jaddi is in the catalog");
    assert_eq!(
        from_all.defender,
        catalog::def(id).defender,
        "catalog::all and catalog::def must expose the same Canonical CardDef"
    );
    assert!(
        from_all.defender,
        "Jaddi has defender through every consumer"
    );
}

#[test]
fn sourced_tatyova_and_jaddi_match_their_printings() {
    let t = card("Tatyova, Benthic Druid").def();
    assert_eq!(t.cost, Some("{3}{G}{U}"));
    assert_eq!(t.type_line, "Legendary Creature — Merfolk Druid");
    assert_eq!((t.power, t.toughness), (Some(3), Some(3)));
    assert_eq!(
        t.oracle,
        "Landfall — Whenever a land you control enters, you gain 1 life and draw a card."
    );
    assert!(card("Tatyova, Benthic Druid").is_supported_commander());
    let j = card("Jaddi Offshoot").def();
    assert_eq!(j.cost, Some("{G}"));
    assert_eq!(j.type_line, "Creature — Plant");
    assert_eq!((j.power, j.toughness), (Some(0), Some(3)));
    assert!(j.oracle.contains("Defender"));
    assert!(
        j.oracle
            .contains("Landfall — Whenever a land you control enters, you gain 1 life.")
    );
    let err = DeckList::parse_practice("bad", "1 Tatyova, Benthic Druid\n39 Forest\n")
        .expect_err("commander-only");
    assert!(err.contains("commander-only"), "{err}");
}

#[test]
fn tatyova_landfall_fires_only_for_its_controller_and_gains_then_draws() {
    let mut game = tatyova_game(51);
    add_permanent(&mut game, P0, card("Tatyova, Benthic Druid"));
    // P1 plays a land: P0's Tatyova must not trigger.
    let p1_land = land_in_hand(&mut game, P1);
    game.state.turn.active = P1;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P1);
    game.state.turn.lands_played = 0;
    let before_life = game.state.player(P0).life;
    let before_hand = game.state.player(P0).hand.len();
    submit_ok(&mut game, P1, Action::PlayLand { card: p1_land });
    for _ in 0..4 {
        if game.state.trigger_queue.is_empty() && game.state.stack.is_empty() {
            break;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert_eq!(
        game.state.player(P0).life,
        before_life,
        "opponent land does not trigger"
    );
    assert_eq!(game.state.player(P0).hand.len(), before_hand);

    // P0 plays a land: +1 life then one draw, fully resolved.
    let p0_land = land_in_hand(&mut game, P0);
    game.state.turn.active = P0;
    game.state.turn.priority = Some(P0);
    game.state.turn.lands_played = 0;
    let before_hand = game.state.player(P0).hand.len();
    let events = submit_ok(&mut game, P0, Action::PlayLand { card: p0_land });
    let mut all = events;
    for _ in 0..8 {
        if game.state.trigger_queue.is_empty() && game.state.stack.is_empty() {
            break;
        }
        let player = game.decision_owner().expect("priority");
        all.extend(submit_ok(&mut game, player, Action::Pass));
    }
    assert_eq!(game.state.player(P0).life, before_life + 1);
    assert_eq!(
        game.state.player(P0).hand.len(),
        before_hand,
        "land leaves hand, one draw returns it"
    );
    let life_at = all
        .iter()
        .position(|e| matches!(e, Event::LifeChange { player, delta: 1, .. } if *player == P0));
    let draw_at = all
        .iter()
        .position(|e| matches!(e, Event::Draw { player, .. } if *player == P0));
    assert!(
        life_at.is_some() && draw_at.is_some(),
        "both effects happened: {all:?}"
    );
    assert!(life_at < draw_at, "gain life resolves before draw: {all:?}");
}

#[test]
fn a_landfall_trigger_still_resolves_after_its_source_leaves() {
    let mut game = tatyova_game(54);
    add_permanent(&mut game, P0, card("Tatyova, Benthic Druid"));
    for _ in 0..2 {
        add_permanent(&mut game, P0, CardId::Island);
    }
    let unsummon = game.state.players[P0 as usize].library.pop().unwrap();
    game.state.objects.get_mut(&unsummon).unwrap().card = CardId::Unsummon;
    game.state.objects.get_mut(&unsummon).unwrap().zone = Zone::Hand;
    game.state.players[P0 as usize].hand.push(unsummon);
    let land = land_in_hand(&mut game, P0);
    let life = game.state.player(P0).life;
    let mut events = submit_ok(&mut game, P0, Action::PlayLand { card: land });
    let tatyova = game
        .state
        .battlefield(P0)
        .into_iter()
        .find(|object| object.card == card("Tatyova, Benthic Druid"))
        .expect("Tatyova on battlefield")
        .id;
    events.extend(submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: unsummon,
            targets: vec![Target::Permanent(tatyova)],
        },
    ));
    for _ in 0..8 {
        if game.state.stack.is_empty() && game.state.trigger_queue.is_empty() {
            break;
        }
        let player = game.decision_owner().expect("priority");
        events.extend(submit_ok(&mut game, player, Action::Pass));
    }
    assert_eq!(
        game.state.player(P0).life,
        life + 1,
        "the queued trigger still gains life"
    );
    assert!(
        game.state.players.iter().any(|p| p
            .hand
            .iter()
            .any(|id| game.state.object(*id).card == card("Tatyova, Benthic Druid"))),
        "the source was returned to hand through a real spell"
    );
    let life_at = events.iter().position(|e| {
        matches!(
            e,
            Event::LifeChange {
                player: P0,
                delta: 1,
                ..
            }
        )
    });
    let draw_at = events
        .iter()
        .position(|e| matches!(e, Event::Draw { player: P0, .. }));
    let resolved_at = events.iter().position(|e| matches!(e, Event::AbilityResolved { card: resolved, .. } if *resolved == card("Tatyova, Benthic Druid")));
    assert!(
        life_at < draw_at && draw_at < resolved_at,
        "life then draw then resolved: {events:?}"
    );
    assert!(
        !events[life_at.unwrap()..=resolved_at.unwrap()]
            .iter()
            .any(|e| matches!(e, Event::Priority { .. })),
        "no priority between gain-life, draw and AbilityResolved"
    );
}

#[test]
fn jaddi_landfall_gains_life_and_defender_blocks_attacks() {
    let mut game = fixture(52)
        .battlefield(P0, &[card("Jaddi Offshoot")])
        .hand(P0, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let jaddi = battlefield_card(&game, P0, card("Jaddi Offshoot"));
    assert!(
        !game.legal_attackers(P0).contains(&jaddi),
        "defender cannot attack"
    );
    let life = game.state.player(P0).life;
    let land = hand_card(&game, P0, CardId::Forest);
    let mut events = submit_ok(&mut game, P0, Action::PlayLand { card: land });
    assert!(
        !game.state.stack.is_empty(),
        "landfall trigger staged: events={events:?}"
    );
    for _ in 0..6 {
        if game.state.trigger_queue.is_empty() && game.state.stack.is_empty() {
            break;
        }
        let player = game.decision_owner().expect("priority");
        events.extend(submit_ok(&mut game, player, Action::Pass));
    }
    assert_eq!(game.state.player(P0).life, life + 1);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::LifeChange {
            player: P0,
            delta: 1,
            ..
        }
    )));

    // Forged direct declaration must be refused with unchanged state.
    let mut game = fixture(53)
        .battlefield(P0, &[card("Jaddi Offshoot")])
        .main(P0, Step::PrecombatMain)
        .build();
    let jaddi = battlefield_card(&game, P0, card("Jaddi Offshoot"));
    game.state.turn.active = P0;
    game.state.turn.step = Step::DeclareAttackers;
    game.state.turn.priority = None;
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let before = aperture_core::digest::digest(&game.state);
    let err = submit_err(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![jaddi],
            defender: Some(P1),
        },
    );
    assert!(err.to_string().to_lowercase().contains("defender"), "{err}");
    assert_eq!(
        aperture_core::digest::digest(&game.state),
        before,
        "refusal leaves the canonical state unchanged"
    );
    assert!(game.state.turn.attackers.is_empty());
}
