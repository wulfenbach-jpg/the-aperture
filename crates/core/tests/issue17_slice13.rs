//! Issue 17 slice 13: landfall counters and Beast/Pest token creation.

mod common;

use aperture_core::catalog;
use aperture_core::counters::CounterKind;
use aperture_core::*;
use common::*;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn drained(game: &Game) -> bool {
    game.state.pending.is_none()
        && game.state.resolving.is_none()
        && game.state.stack.is_empty()
        && game.state.trigger_queue.is_empty()
}

fn resolve_all(game: &mut Game) {
    for _ in 0..32 {
        if drained(game) {
            return;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(game, player, Action::Pass);
    }
    assert!(drained(game));
}

fn token_of<'a>(
    game: &'a Game,
    controller: PlayerId,
    name: &str,
) -> Vec<&'a aperture_core::state::Object> {
    game.state
        .battlefield(controller)
        .into_iter()
        .filter(|o| o.name() == name)
        .collect()
}

#[test]
fn sourced_slice13_cards_match_their_printings() {
    let bird = card("Sazh's Chocobo").def();
    assert_eq!(bird.cost, Some("{G}"));
    assert_eq!(bird.type_line, "Creature — Bird");
    assert_eq!((bird.power, bird.toughness), (Some(0), Some(1)));
    assert_eq!(
        bird.oracle,
        "Landfall — Whenever a land you control enters, put a +1/+1 counter on this creature."
    );
    let baloths = card("Rampaging Baloths").def();
    assert_eq!(baloths.cost, Some("{4}{G}{G}"));
    assert_eq!((baloths.power, baloths.toughness), (Some(6), Some(6)));
    assert!(baloths.has_trample());
    assert_eq!(
        baloths.oracle,
        "Trample\nLandfall — Whenever a land you control enters, create a 4/4 green Beast creature token."
    );
    let professor = card("Professor of Zoomancy").def();
    assert_eq!(professor.cost, Some("{3}{G}"));
    assert_eq!((professor.power, professor.toughness), (Some(4), Some(3)));
    assert!(professor.oracle.contains("1/1 black and green Pest"));
    assert!(catalog::by_name("Beast Token").is_none());
    assert!(catalog::by_name("Pest Token").is_none());
}

#[test]
fn sazh_chocobo_landfall_adds_a_real_counter_only_for_its_controller() {
    let mut game = fixture(170)
        .hand(P0, &[card("Sazh's Chocobo")])
        .battlefield(P0, &[CardId::Forest, CardId::Forest])
        .hand(P0, &[CardId::Forest])
        .library(P0, &[CardId::Forest; 2])
        .library(P1, &[CardId::Forest; 1])
        .main(P0, Step::PrecombatMain)
        .build();
    let bird_hand = hand_card(&game, P0, card("Sazh's Chocobo"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bird_hand,
            targets: Vec::new(),
        },
    );
    resolve_all(&mut game);
    let bird = game
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == card("Sazh's Chocobo"))
        .unwrap()
        .id;
    let land = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: land });
    resolve_all(&mut game);
    assert_eq!(
        game.state
            .object(bird)
            .counters
            .get(CounterKind::PlusOnePlusOne),
        1
    );
    assert_eq!(game.state.object(bird).power(), 1);
    assert_eq!(game.state.object(bird).toughness(), 2);

    // Repeated own landfall still adds; an opponent's land does not.
    game.state.turn.lands_played = 0;
    let land2 = game.state.players[P0 as usize].library.pop().unwrap();
    game.state.objects.get_mut(&land2).unwrap().zone = Zone::Hand;
    game.state.players[P0 as usize].hand.push(land2);
    submit_ok(&mut game, P0, Action::PlayLand { card: land2 });
    resolve_all(&mut game);
    assert_eq!(
        game.state
            .object(bird)
            .counters
            .get(CounterKind::PlusOnePlusOne),
        2
    );

    let opp_land = game.state.players[P1 as usize].library.pop().unwrap();
    game.state.objects.get_mut(&opp_land).unwrap().card = CardId::Forest;
    game.state.objects.get_mut(&opp_land).unwrap().zone = Zone::Hand;
    game.state.players[P1 as usize].hand.push(opp_land);
    game.state.turn.active = P1;
    game.state.turn.priority = Some(P1);
    game.state.turn.lands_played = 0;
    let count = game
        .state
        .object(bird)
        .counters
        .get(CounterKind::PlusOnePlusOne);
    submit_ok(&mut game, P1, Action::PlayLand { card: opp_land });
    resolve_all(&mut game);
    assert_eq!(
        game.state
            .object(bird)
            .counters
            .get(CounterKind::PlusOnePlusOne),
        count
    );
}

#[test]
fn rampaging_baloths_creates_beast_token_and_survives_source_left() {
    let mut game = fixture(171)
        .hand(P0, &[card("Rampaging Baloths"), CardId::Unsummon])
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Island,
            ],
        )
        .hand(P0, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let baloths_hand = hand_card(&game, P0, card("Rampaging Baloths"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: baloths_hand,
            targets: Vec::new(),
        },
    );
    resolve_all(&mut game);
    let baloths = game
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == card("Rampaging Baloths"))
        .unwrap()
        .id;
    assert!(game.state.object(baloths).has_trample());
    let land = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: land });
    // Bounce the source before its landfall trigger resolves.
    let unsummon = hand_card(&game, P0, CardId::Unsummon);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: unsummon,
            targets: vec![Target::Permanent(baloths)],
        },
    );
    resolve_all(&mut game);
    let beasts = token_of(&game, P0, "Beast Token");
    assert_eq!(beasts.len(), 1, "queued landfall token still created");
    assert_eq!((beasts[0].power(), beasts[0].toughness()), (4, 4));
    assert_eq!(beasts[0].card.def().colors, &[ManaColor::Green]);
    assert!(beasts[0].card.def().type_line.contains("Beast"));
}

#[test]
fn professor_pest_token_dies_gaining_life_once_after_full_drain() {
    let mut game = fixture(172)
        .hand(P0, &[card("Professor of Zoomancy"), CardId::Shock])
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Mountain,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let professor = hand_card(&game, P0, card("Professor of Zoomancy"));
    let life = game.state.player(P0).life;
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: professor,
            targets: Vec::new(),
        },
    );
    resolve_all(&mut game);
    let pests = token_of(&game, P0, "Pest Token");
    assert_eq!(pests.len(), 1);
    assert_eq!((pests[0].power(), pests[0].toughness()), (1, 1));
    assert_eq!(
        pests[0].card.def().colors,
        &[ManaColor::Black, ManaColor::Green]
    );
    let pest = pests[0].id;
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Permanent(pest)],
        },
    );
    resolve_all(&mut game);
    for _ in 0..4 {
        if game.state.try_object(pest).is_none() {
            break;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert!(
        game.state.try_object(pest).is_none(),
        "token ceases after death: {:?}",
        game.state.try_object(pest)
    );
    assert_eq!(
        game.state.player(P0).life,
        life + 1,
        "Pest's own dies trigger once"
    );
    assert_eq!(game.state.player(P1).life, 20, "no opponent drain");
}
