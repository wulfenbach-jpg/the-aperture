//! Issue 17 slice 2: Talrand, Sky Summoner — spell-cast trigger, Drake tokens,
//! and functional flying block legality.

mod common;

use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn talrand_game(seed: u64) -> Game {
    let deck = DeckList::parse_practice("talrand", "40 Island\n").expect("deck");
    let mut game = Game::new_commander_combat(
        seed,
        &[deck.clone(), deck],
        &[
            (
                P0,
                catalog::by_name("Talrand, Sky Summoner").expect("Talrand"),
            ),
            (
                P1,
                catalog::by_name("Talrand, Sky Summoner").expect("Talrand"),
            ),
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

fn put_talrand_on_battlefield(game: &mut Game, player: PlayerId) -> ObjectId {
    let talrand = catalog::by_name("Talrand, Sky Summoner").expect("Talrand");
    let id = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card: talrand,
            owner: player,
            controller: player,
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

fn drakes(game: &Game, controller: PlayerId) -> Vec<&aperture_core::state::Object> {
    game.state
        .battlefield(controller)
        .into_iter()
        .filter(|object| object.card == CardId::DrakeToken)
        .collect()
}

#[test]
fn talrand_and_drake_are_sourced_and_the_drake_is_not_importable() {
    let def = catalog::by_name("Talrand, Sky Summoner")
        .expect("Talrand is in the pool")
        .def();
    assert_eq!(def.cost, Some("{2}{U}{U}"));
    assert_eq!(def.type_line, "Legendary Creature — Merfolk Wizard");
    assert_eq!((def.power, def.toughness), (Some(2), Some(2)));
    assert_eq!(
        def.oracle,
        "Whenever you cast an instant or sorcery spell, create a 2/2 blue Drake creature token with flying."
    );
    let drake = CardId::DrakeToken.def();
    assert_eq!(drake.type_line, "Token Creature — Drake");
    assert_eq!((drake.power, drake.toughness), (Some(2), Some(2)));
    assert_eq!(drake.colors, &[ManaColor::Blue]);
    assert!(drake.has_flying());
    assert!(catalog::by_name("Drake Token").is_none());
    assert!(catalog::by_name("Drake").is_none());
    let err = DeckList::parse_practice("bad", "1 Drake Token\n39 Island\n").unwrap_err();
    assert!(err.contains("not in the supported pool"));
}

#[test]
fn casting_an_instant_stages_the_trigger_above_it_and_creates_a_drake() {
    let mut game = talrand_game(31);
    put_talrand_on_battlefield(&mut game, P0);
    for _ in 0..3 {
        let island = game.state.players[P0 as usize]
            .library
            .pop()
            .expect("island");
        game.state.objects.get_mut(&island).unwrap().zone = Zone::Battlefield;
        game.state.players[P0 as usize].hand.push(island);
    }
    let opt = game.state.players[P0 as usize].library.pop().expect("opt");
    game.state.objects.get_mut(&opt).unwrap().card = CardId::Opt;
    game.state.objects.get_mut(&opt).unwrap().zone = Zone::Hand;
    game.state.players[P0 as usize].hand.push(opt);
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: Vec::new(),
        },
    );
    assert!(
        drakes(&game, P0).is_empty(),
        "no token before the trigger resolves"
    );
    assert_eq!(game.state.stack.len(), 2, "spell + trigger");
    assert!(
        matches!(
            game.state.stack.last().unwrap().entry,
            StackEntry::Triggered { .. }
        ),
        "the trigger is above the spell"
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(drakes(&game, P0).len(), 1);
    assert_eq!(game.state.stack.len(), 1, "the spell is still on the stack");
    let drake = drakes(&game, P0)[0];
    assert_eq!(drake.name(), "Drake Token", "CR 111.4 generated-token name");
    assert!(drake.has_flying());
    assert_eq!((drake.power(), drake.toughness()), (2, 2));
}

#[test]
fn the_trigger_still_resolves_after_the_spell_is_countered() {
    let mut game = talrand_game(35);
    put_talrand_on_battlefield(&mut game, P0);
    for _ in 0..3 {
        let island = game.state.players[P0 as usize].library.pop().unwrap();
        game.state.objects.get_mut(&island).unwrap().card = CardId::Island;
        game.state.objects.get_mut(&island).unwrap().zone = Zone::Battlefield;
        game.state.objects.get_mut(&island).unwrap().summoning_sick = false;
        game.state.players[P0 as usize].hand.push(island);
    }
    for _ in 0..2 {
        let island = game.state.players[P1 as usize].library.pop().unwrap();
        game.state.objects.get_mut(&island).unwrap().card = CardId::Island;
        game.state.objects.get_mut(&island).unwrap().zone = Zone::Battlefield;
        game.state.objects.get_mut(&island).unwrap().summoning_sick = false;
        game.state.players[P1 as usize].hand.push(island);
    }
    let opt = game.state.players[P0 as usize].library.pop().unwrap();
    game.state.objects.get_mut(&opt).unwrap().card = CardId::Opt;
    game.state.objects.get_mut(&opt).unwrap().zone = Zone::Hand;
    game.state.players[P0 as usize].hand.push(opt);
    let counter = game.state.players[P1 as usize].library.pop().unwrap();
    game.state.objects.get_mut(&counter).unwrap().card = CardId::Counterspell;
    game.state.objects.get_mut(&counter).unwrap().zone = Zone::Hand;
    game.state.players[P1 as usize].hand.push(counter);
    game.state.turn.priority = Some(P0);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: Vec::new(),
        },
    );
    let opt_on_stack = game.state.stack[0].object;
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: counter,
            targets: vec![Target::Spell(opt_on_stack)],
        },
    );
    for _ in 0..8 {
        if game.state.stack.is_empty() {
            break;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert_eq!(
        drakes(&game, P0).len(),
        1,
        "the trigger resolved above the spell"
    );
    assert!(
        game.state
            .players
            .iter()
            .any(|player| player
                .graveyard
                .iter()
                .any(|id| game.state.object(*id).card == CardId::Opt)),
        "the countered spell is in the graveyard"
    );
}

#[test]
fn an_opponents_spell_does_not_trigger_talrand() {
    let mut game = talrand_game(32);
    put_talrand_on_battlefield(&mut game, P0);
    game.state.turn.active = P1;
    game.state.turn.priority = Some(P1);
    game.state.turn.step = Step::PrecombatMain;
    for _ in 0..2 {
        let mountain = game.state.players[P1 as usize].library.pop().expect("card");
        game.state.objects.get_mut(&mountain).unwrap().card = CardId::Mountain;
        game.state.objects.get_mut(&mountain).unwrap().zone = Zone::Battlefield;
        game.state
            .objects
            .get_mut(&mountain)
            .unwrap()
            .summoning_sick = false;
    }
    let shock = game.state.players[P1 as usize].library.pop().expect("card");
    game.state.objects.get_mut(&shock).unwrap().card = CardId::Shock;
    game.state.objects.get_mut(&shock).unwrap().zone = Zone::Hand;
    game.state.players[P1 as usize].hand.push(shock);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P0)],
        },
    );
    for _ in 0..4 {
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert!(drakes(&game, P0).is_empty());
}

#[test]
fn nonflyers_cannot_block_flyers_and_the_refusal_preserves_state() {
    let mut game = fixture(33)
        .battlefield(P0, &[CardId::DrakeToken])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let drake = battlefield_card(&game, P0, CardId::DrakeToken);
    let bear = battlefield_card(&game, P1, CardId::GrizzlyBears);
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
            attackers: vec![drake],
            defender: Some(P1),
        },
    );
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::DeclareBlockers { .. })),
        64
    ));
    let before = game.state.revision;
    let err = submit_err(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: bear,
                attacker: drake,
            }],
        },
    );
    assert!(err.to_string().to_lowercase().contains("fly"), "{err}");
    assert_eq!(game.state.revision, before, "refusal changes no state");
    assert!(game.state.turn.blockers.is_empty());
}

#[test]
fn flyers_can_block_either_flyers_or_nonflyers() {
    let mut game = fixture(34)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::DrakeToken])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let drake = battlefield_card(&game, P1, CardId::DrakeToken);
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
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::DeclareBlockers { .. })),
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: drake,
                attacker: bear,
            }],
        },
    );
    assert_eq!(game.state.turn.blockers.len(), 1);
}
