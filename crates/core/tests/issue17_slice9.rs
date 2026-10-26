//! Issue 17 slice 9: widened mana pool and count-scaled red rituals.

mod common;

use aperture_core::catalog;
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

fn add_permanent(game: &mut Game, controller: PlayerId, id: CardId, tapped: bool) -> ObjectId {
    let oid = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        oid,
        aperture_core::state::Object {
            id: oid,
            card: id,
            owner: controller,
            controller,
            zone: Zone::Battlefield,
            commander: None,
            tapped,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: Default::default(),
        },
    );
    oid
}

fn resolve_all(game: &mut Game) {
    for _ in 0..16 {
        if drained(game) {
            return;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(game, player, Action::Pass);
    }
    assert!(drained(game));
}

#[test]
fn mana_pool_adds_and_pays_above_255_without_truncation() {
    let mut pool = ManaPool::default();
    pool.add(ManaColor::Red, 255);
    pool.add(ManaColor::Red, 45);
    assert_eq!(pool.red, 300);
    assert_eq!(pool.total(), 300);
    let mut game = fixture(101)
        .hand(P0, &[CardId::LightningBolt])
        .main(P0, Step::PrecombatMain)
        .build();
    game.state.players[P0 as usize]
        .mana
        .add(ManaColor::Red, 255);
    game.state.players[P0 as usize].mana.add(ManaColor::Red, 45);
    assert_eq!(game.state.player(P0).mana.red, 300);
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Player(P1)],
        },
    );
    resolve_all(&mut game);
    assert_eq!(
        game.state.player(P0).mana.red,
        299,
        "one red pip paid exactly"
    );
    assert_eq!(game.state.player(P1).life, 17);
    let json = serde_json::to_string(&game.state).expect("state json");
    let restored: GameState = serde_json::from_str(&json).expect("state parses");
    assert_eq!(restored.player(P0).mana.red, 299);
}

#[test]
fn brightstone_counts_all_goblins_including_opponents_and_above_255() {
    let mut game = fixture(102)
        .hand(P0, &[card("Brightstone Ritual")])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    for _ in 0..150 {
        add_permanent(&mut game, P0, CardId::GoblinInstigator, false);
        add_permanent(&mut game, P1, CardId::GoblinInstigator, false);
    }
    let spell = hand_card(&game, P0, card("Brightstone Ritual"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    resolve_all(&mut game);
    assert_eq!(game.state.player(P0).mana.red, 300);
}

#[test]
fn mana_geyser_counts_only_opponent_tapped_lands() {
    let mut game = fixture(103)
        .hand(P0, &[card("Mana Geyser")])
        .battlefield(P0, &[CardId::Mountain; 5])
        .main(P0, Step::PrecombatMain)
        .build();
    for _ in 0..4 {
        add_permanent(&mut game, P1, CardId::Island, true);
        add_permanent(&mut game, P1, CardId::Island, false);
    }
    add_permanent(&mut game, P0, CardId::Mountain, true);
    // Pay {3}{R}{R}; after casting, the spell resolves against 4 tapped opponents' lands.
    for _ in 0..2 {
        let land = game
            .state
            .battlefield(P0)
            .into_iter()
            .find(|o| o.card == CardId::Mountain && !o.tapped)
            .unwrap()
            .id;
        submit_ok(&mut game, P0, Action::ActivateManaAbility { land });
    }
    let spell = hand_card(&game, P0, card("Mana Geyser"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    resolve_all(&mut game);
    assert_eq!(game.state.player(P0).mana.red, 4);
}

#[test]
fn rite_of_flame_counts_base_and_all_graveyard_copies() {
    let mut game = fixture(104)
        .hand(P0, &[card("Rite of Flame")])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    // Two Rite of Flame cards already in graveyards before resolution.
    let first = add_permanent(&mut game, P0, card("Rite of Flame"), false);
    let second = add_permanent(&mut game, P1, card("Rite of Flame"), false);
    game.state.players[P0 as usize]
        .graveyard
        .extend([first, second]);
    for id in game.state.players[P0 as usize].graveyard.clone() {
        game.state.objects.get_mut(&id).unwrap().zone = Zone::Graveyard;
    }
    let spell = hand_card(&game, P0, card("Rite of Flame"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    resolve_all(&mut game);
    // 2 base + 1 for each of two graveyard copies = 4.
    assert_eq!(game.state.player(P0).mana.red, 4);
}
