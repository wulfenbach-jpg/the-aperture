//! Slice 67 correctness: Siege-Gang Lieutenant complete printed text.
mod common;
use aperture_core::*;
use common::*;
const P2: PlayerId = 2;
const P3: PlayerId = 3;

fn add_object(g: &mut Game, p: PlayerId, card: CardId, zone: Zone) -> ObjectId {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card,
            owner: p,
            controller: p,
            zone,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: aperture_core::counters::Counters::default(),
            paired_with: None,
        },
    );
    match zone {
        Zone::Hand => g.state.players[p as usize].hand.push(id),
        Zone::Battlefield => {}
        Zone::Graveyard => g.state.players[p as usize].graveyard.push(id),
        _ => {}
    }
    id
}
fn commander_game(seed: u64, commander_on_battlefield: bool, commander_seat: PlayerId) -> Game {
    let deck = DeckList::parse_practice("A", "40 Mountain\n").unwrap();
    let mut g = Game::new_commander_combat(
        seed,
        &[deck.clone(), deck.clone(), deck.clone(), deck],
        &[
            (P0, CardId::KrenkoMobBoss),
            (P1, CardId::KrenkoMobBoss),
            (P2, CardId::KrenkoMobBoss),
            (P3, CardId::KrenkoMobBoss),
        ],
    )
    .unwrap();
    g.state.mulligan = None;
    g.state.pending = None;
    let commander = g
        .state
        .commanders
        .iter()
        .find(|c| c.owner == commander_seat)
        .unwrap()
        .object;
    if commander_on_battlefield {
        g.state.players[commander_seat as usize]
            .command_zone
            .retain(|id| *id != commander);
        let object = g.state.objects.get_mut(&commander).unwrap();
        object.zone = Zone::Battlefield;
        object.controller = commander_seat;
        object.summoning_sick = false;
    }
    g.state.turn.active = P0;
    g.state.turn.number = 3;
    g.state.turn.step = Step::PrecombatMain;
    g.state.turn.priority = Some(P0);
    g
}

#[test]
fn siege_gang_lieutenant_cast_trigger_tokens_haste_and_activation() {
    let mut g = commander_game(6701, true, P0);
    add_object(&mut g, P0, CardId::Mountain, Zone::Battlefield);
    add_object(&mut g, P0, CardId::Mountain, Zone::Battlefield);
    add_object(&mut g, P0, CardId::Mountain, Zone::Battlefield);
    add_object(&mut g, P0, CardId::Mountain, Zone::Battlefield);
    let card = add_object(&mut g, P0, CardId::SiegeGangLieutenant, Zone::Hand);
    let log = submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    assert!(log.iter().any(|e| matches!(
        e,
        Event::Cast {
            card: CardId::SiegeGangLieutenant,
            ..
        }
    )));
    for _ in 0..10 {
        if g.state.battlefield(P0).iter().any(|o| o.id == card) {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(
        g.state.battlefield(P0).iter().any(|o| o.id == card),
        "cast resolved"
    );
    // Enter BeginCombat through a real step transition.
    g.state.turn.step = Step::PrecombatMain;
    g.state.turn.priority = Some(P0);
    submit_ok(&mut g, P0, Action::Pass);
    for _ in 0..20 {
        if g.state
            .battlefield(P0)
            .iter()
            .filter(|o| o.card == CardId::GoblinToken)
            .count()
            == 2
        {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    let tokens: Vec<ObjectId> = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::GoblinToken)
        .map(|o| o.id)
        .collect();
    assert_eq!(tokens.len(), 2, "Lieutenant creates exactly two tokens");
    for id in &tokens {
        assert_eq!(g.state.object(*id).power(), 1);
        assert_eq!(g.state.object(*id).toughness(), 1);
        assert!(
            g.legal_attackers(P0).contains(id),
            "tokens gain haste this turn"
        );
    }
    // Activated ability: {2}, sacrifice a Goblin, deal 1 to any target.
    add_object(&mut g, P0, CardId::Mountain, Zone::Battlefield);
    add_object(&mut g, P0, CardId::Mountain, Zone::Battlefield);
    let before = g.state.player(P1).life;
    let token = tokens[0];
    let action = Action::ActivateCostedAbility {
        source: card,
        ability: 1,
        mode: 0,
        cost_cards: vec![token],
        targets: vec![Target::Player(P1)],
    };
    assert!(
        g.legal_actions().iter().any(|x| x.action == action),
        "activation offered"
    );
    let mut log = submit_ok(&mut g, P0, action);
    for _ in 0..30 {
        if g.state.stack.is_empty() && g.state.pending.is_none() {
            break;
        }
        let p = g.decision_owner().unwrap();
        log.extend(submit_ok(&mut g, p, Action::Pass));
    }
    assert!(
        g.state
            .try_object(token)
            .is_none_or(|o| o.zone != Zone::Battlefield),
        "the sacrificed token left the battlefield"
    );
    assert!(g.state.player(P1).life < before, "one damage dealt");
}

#[test]
fn lieutenant_requires_your_commander_at_trigger_and_is_absent_without_one() {
    let mut g = commander_game(6702, false, P0);
    add_object(&mut g, P0, CardId::SiegeGangLieutenant, Zone::Battlefield);
    g.state.turn.step = Step::PrecombatMain;
    g.state.turn.priority = Some(P0);
    submit_ok(&mut g, P0, Action::Pass);
    for _ in 0..15 {
        if g.state.turn.step == Step::BeginCombat && g.state.pending.is_none() {
            break;
        }
        let Some(p) = g.decision_owner() else { break };
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(
        g.state
            .battlefield(P0)
            .iter()
            .filter(|o| o.card == CardId::GoblinToken)
            .count(),
        0,
        "no own commander means no Lieutenant trigger"
    );

    let mut opponent = commander_game(6703, true, P1);
    add_object(
        &mut opponent,
        P0,
        CardId::SiegeGangLieutenant,
        Zone::Battlefield,
    );
    opponent.state.turn.step = Step::PrecombatMain;
    opponent.state.turn.priority = Some(P0);
    submit_ok(&mut opponent, P0, Action::Pass);
    for _ in 0..15 {
        if opponent.state.turn.step == Step::BeginCombat && opponent.state.pending.is_none() {
            break;
        }
        let Some(p) = opponent.decision_owner() else {
            break;
        };
        submit_ok(&mut opponent, p, Action::Pass);
    }
    assert_eq!(
        opponent
            .state
            .battlefield(P0)
            .iter()
            .filter(|o| o.card == CardId::GoblinToken)
            .count(),
        0,
        "an opponent's commander does not satisfy Lieutenant"
    );
}

#[test]
fn lieutenant_trigger_still_resolves_after_its_source_leaves() {
    let mut g = commander_game(6704, true, P0);
    let lieutenant = add_object(&mut g, P0, CardId::SiegeGangLieutenant, Zone::Battlefield);
    g.state.turn.step = Step::PrecombatMain;
    g.state.turn.priority = Some(P0);
    submit_ok(&mut g, P0, Action::Pass);
    for _ in 0..12 {
        if g.state
            .stack
            .iter()
            .any(|item| item.card == CardId::SiegeGangLieutenant)
        {
            break;
        }
        let Some(p) = g.decision_owner() else { break };
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(
        g.state
            .stack
            .iter()
            .any(|item| item.card == CardId::SiegeGangLieutenant)
    );
    if let Some(object) = g.state.objects.get_mut(&lieutenant) {
        object.zone = Zone::Graveyard;
    }
    g.state.players[P0 as usize].graveyard.push(lieutenant);
    for _ in 0..12 {
        if g.state
            .battlefield(P0)
            .iter()
            .filter(|o| o.card == CardId::GoblinToken)
            .count()
            == 2
        {
            break;
        }
        let Some(p) = g.decision_owner() else { break };
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(
        g.state
            .battlefield(P0)
            .iter()
            .filter(|o| o.card == CardId::GoblinToken)
            .count(),
        2,
        "the queued Lieutenant trigger resolves with source LKI"
    );
}
