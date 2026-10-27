//! Slice 21: Dragon-Scarred Bear regeneration.
mod common;
use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}
fn drain(g: &mut Game) {
    for _ in 0..96 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(g, p, Action::Pass);
    }
    panic!("did not settle");
}
fn cast(g: &mut Game, id: ObjectId, targets: Vec<Target>) {
    submit_ok(g, P0, Action::CastSpell { card: id, targets });
    drain(g);
}
fn activate_shield(g: &mut Game, bear: ObjectId) {
    let action = g
        .legal_actions()
        .into_iter()
        .find(|a| {
            matches!(
                a.action, Action::ActivateAbility { source, .. } if source == bear
            )
        })
        .expect("offered paid ability")
        .action;
    submit_ok(g, P0, action);
    drain(g);
}
fn add(g: &mut Game, c: CardId) -> ObjectId {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card: c,
            owner: P0,
            controller: P0,
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
fn add_hand(g: &mut Game, c: CardId) -> ObjectId {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card: c,
            owner: P0,
            controller: P0,
            zone: Zone::Hand,
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
    g.state.players[P0 as usize].hand.push(id);
    id
}
fn shield_game(seed: u64, extra_power: &[CardId], forests: usize) -> (Game, ObjectId) {
    let mut board = vec![CardId::DragonScarredBear];
    board.extend_from_slice(extra_power);
    for _ in 0..forests {
        board.push(CardId::Forest);
    }
    let g = fixture(seed)
        .battlefield(P0, &board)
        .library(P0, &[CardId::GrizzlyBears; 6])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&g, P0, CardId::DragonScarredBear);
    (g, bear)
}
fn shock(g: &mut Game, target: ObjectId) {
    add(g, CardId::Mountain);
    let s = add_hand(g, CardId::Shock);
    cast(g, s, vec![Target::Permanent(target)]);
}

#[test]
fn source_and_offered_cast_activation_and_formidable_refusal() {
    let def = card("Dragon-Scarred Bear").def();
    assert_eq!(def.cost, Some("{2}{G}"));
    assert_eq!(
        def.oracle,
        "Formidable — {1}{G}: Regenerate this creature. Activate only if creatures you control have total power 8 or greater."
    );
    let mut g = fixture(2101)
        .hand(P0, &[card("Dragon-Scarred Bear")])
        .battlefield(P0, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let spell = hand_card(&g, P0, card("Dragon-Scarred Bear"));
    let offered = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::CastSpell { card, .. } if card == spell))
        .expect("offered cast")
        .action;
    submit_ok(&mut g, P0, offered);
    drain(&mut g);
    let bear = battlefield_card(&g, P0, card("Dragon-Scarred Bear"));
    for _ in 0..3 {
        add(&mut g, CardId::Forest);
    }
    add(&mut g, CardId::GrizzlyBears);
    add(&mut g, CardId::RuneclawBear); // 3 + 2 + 2 = 7, below Formidable
    assert!(
        !g.legal_actions()
            .iter()
            .any(|a| matches!(a.action, Action::ActivateAbility { source, .. } if source == bear))
    );
    let before = g.state.clone();
    let err = g
        .submit_action(
            P0,
            Action::ActivateAbility {
                source: bear,
                ability: 0,
                mode: 0,
                targets: Vec::new(),
            },
        )
        .expect_err("below 8 must be refused");
    assert!(format!("{err:?}").contains("Formidable"), "{err:?}");
    assert_eq!(g.state, before);
    add(&mut g, CardId::CentaurCourser); // total 10
    activate_shield(&mut g, bear);
    assert_eq!(g.state.regeneration_shields.len(), 1);
    assert!(!g.state.object(bear).tapped, "shield creation does not tap");
    assert_eq!(g.state.object(bear).damage, 0);
}

#[test]
fn lethal_hit_consumes_shield_taps_and_prevents_death() {
    let (mut g, bear) = shield_game(
        2102,
        &[
            CardId::GrizzlyBears,
            CardId::GrizzlyBears,
            CardId::RuneclawBear,
        ],
        12,
    );
    activate_shield(&mut g, bear);
    shock(&mut g, bear);
    assert_eq!(g.state.object(bear).zone, Zone::Battlefield);
    assert!(g.state.object(bear).tapped);
    assert_eq!(g.state.object(bear).damage, 0);
    assert!(g.state.regeneration_shields.is_empty());
    shock(&mut g, bear);
    assert_eq!(
        g.state.object(bear).zone,
        Zone::Graveyard,
        "a second hit without a shield destroys it"
    );
}

#[test]
fn multiple_shields_are_consumed_one_per_destruction() {
    let (mut g, bear) = shield_game(
        2103,
        &[
            CardId::GrizzlyBears,
            CardId::GrizzlyBears,
            CardId::RuneclawBear,
        ],
        14,
    );
    activate_shield(&mut g, bear);
    activate_shield(&mut g, bear);
    assert_eq!(g.state.regeneration_shields.len(), 2);
    shock(&mut g, bear);
    assert_eq!(g.state.object(bear).zone, Zone::Battlefield);
    assert_eq!(g.state.regeneration_shields.len(), 1);
    shock(&mut g, bear);
    assert_eq!(g.state.object(bear).zone, Zone::Battlefield);
    assert!(g.state.regeneration_shields.is_empty());
    shock(&mut g, bear);
    assert_eq!(g.state.object(bear).zone, Zone::Graveyard);
}

#[test]
fn exile_and_bounce_are_not_destruction_and_do_not_transfer_shields() {
    let (mut g, bear) = shield_game(
        2104,
        &[
            CardId::GrizzlyBears,
            CardId::GrizzlyBears,
            CardId::RuneclawBear,
        ],
        16,
    );
    activate_shield(&mut g, bear);
    add(&mut g, CardId::Plains);
    let swords = add_hand(&mut g, CardId::SwordsToPlowshares);
    cast(&mut g, swords, vec![Target::Permanent(bear)]);
    assert_eq!(
        g.state.object(bear).zone,
        Zone::Exile,
        "exile is not replaced"
    );
    assert_eq!(
        g.state.regeneration_shields.len(),
        1,
        "shield stays keyed to the old object"
    );

    let (mut g, bear) = shield_game(
        2105,
        &[
            CardId::GrizzlyBears,
            CardId::GrizzlyBears,
            CardId::RuneclawBear,
        ],
        16,
    );
    activate_shield(&mut g, bear);
    add(&mut g, CardId::Island);
    let unsummon = add_hand(&mut g, CardId::Unsummon);
    cast(&mut g, unsummon, vec![Target::Permanent(bear)]);
    assert_eq!(g.state.object(bear).zone, Zone::Hand);
    // A fresh object (as after a zone change) is not protected by the old
    // shield keyed to the previous object id.
    let new_bear = add(&mut g, CardId::DragonScarredBear);
    assert_ne!(new_bear, bear);
    shock(&mut g, new_bear);
    assert_eq!(
        g.state.object(new_bear).zone,
        Zone::Graveyard,
        "shield cannot transfer to a new object"
    );
}

#[test]
fn save_reload_with_live_shield_has_parity() {
    let (mut g, bear) = shield_game(
        2107,
        &[
            CardId::GrizzlyBears,
            CardId::GrizzlyBears,
            CardId::RuneclawBear,
        ],
        12,
    );
    activate_shield(&mut g, bear);
    assert_eq!(g.state.regeneration_shields.len(), 1);
    let decks = vec![
        DeckList::parse_practice("A", "40 Forest\n").unwrap(),
        DeckList::parse_practice("B", "40 Forest\n").unwrap(),
    ];
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        decks,
        AiSettings::default(),
    )
    .unwrap();
    let save = SessionSave::capture(&g, replay, Vec::new(), true);
    let loaded = SessionSave::from_json(&save.to_json()).expect("save with live shield loads");
    let mut loaded = Game::from_state(loaded.game).unwrap();
    assert_eq!(loaded.digest(), g.digest());
    shock(&mut g, bear);
    shock(&mut loaded, bear);
    assert_eq!(loaded.state, g.state);
    assert_eq!(loaded.digest(), g.digest());
}

#[test]
fn regeneration_removes_a_blocked_attacker_from_combat_without_player_damage() {
    let (mut g, bear) = shield_game(
        2108,
        &[
            CardId::GrizzlyBears,
            CardId::GrizzlyBears,
            CardId::RuneclawBear,
        ],
        12,
    );
    let blocker = add(&mut g, CardId::GrizzlyBears);
    g.state.objects.get_mut(&blocker).unwrap().controller = P1;
    g.state.objects.get_mut(&blocker).unwrap().owner = P1;
    activate_shield(&mut g, bear);
    let life = g.state.player(P1).life;
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| { g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some() },
        64
    ));
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| { g.state.turn.step == Step::DeclareBlockers && g.state.pending.is_some() },
        64
    ));
    submit_ok(
        &mut g,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker,
                attacker: bear,
            }],
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| { g.state.object(blocker).zone == Zone::Graveyard },
        64
    ));
    assert_eq!(g.state.object(bear).zone, Zone::Battlefield);
    assert!(g.state.object(bear).tapped);
    assert!(
        g.state.turn.attackers.is_empty(),
        "attacker removed from combat"
    );
    assert!(g.state.turn.blockers.is_empty(), "blocker relation removed");
    assert_eq!(
        g.state.player(P1).life,
        life,
        "blocked attacker did not hit the player"
    );
}
