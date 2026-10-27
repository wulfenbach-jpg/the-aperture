//! Independent review-289 probe for slice 32. Out of tree; depends on the
//! candidate crate by path. Builds states directly (like the in-repo test
//! fixture) and drives the public API only.

use std::collections::BTreeMap;

use aperture_core::state::{Object, Player, Turn};
use aperture_core::*;

const P2: PlayerId = 2;
const P3: PlayerId = 3;

fn new_player(id: PlayerId, name: &str) -> Player {
    Player {
        id,
        name: name.to_string(),
        deck_name: format!("probe-{name}"),
        library: Vec::new(),
        hand: Vec::new(),
        graveyard: Vec::new(),
        exile: Vec::new(),
        command_zone: Vec::new(),
        life: 20,
        mana: ManaPool::default(),
        mulligans: 0,
        drew_from_empty: false,
        lost: false,
    }
}

fn fixture_n(seed: u64, seat_count: usize) -> Game {
    assert!((2..=4).contains(&seat_count));
    Game {
        state: GameState {
            seed,
            first_player: P0,
            rng: aperture_core::rng::Pcg32::new(seed),
            objects: BTreeMap::new(),
            commanders: Vec::new(),
            profile: Profile::Practice20,
            commander_damage: Vec::new(),
            retired_commanders: Vec::new(),
            prevent_all_combat_damage_turn: None,
            players: (0..seat_count as PlayerId)
                .map(|id| new_player(id, &format!("P{id}")))
                .collect(),
            stack: Vec::new(),
            turn: Turn {
                number: 1,
                active: P0,
                step: Step::Untap,
                priority: None,
                passed: 0,
                lands_played: 0,
                attackers: Vec::new(),
                blocked: Vec::new(),
                blockers: Vec::new(),
                combat_damage_done: false,
                defender: None,
            },
            pending: None,
            mulligan: None,
            resolving: None,
            next_object: 1,
            trigger_queue: Vec::new(),
            next_trigger_seq: 0,
            revealed: Vec::new(),
            creature_died_this_turn: false,
            nontoken_creature_deaths_this_turn: Vec::new(),
            activation_uses: BTreeMap::new(),
            skip_next_untap: Default::default(),
            effects: Vec::new(),
            shields: Vec::new(),
            regeneration_shields: Vec::new(),
            next_effect: 1,
            next_timestamp: 1,
            revision: 0,
            decision: 0,
            outcome: None,
            rules_revision: RULES_REVISION.to_string(),
        },
    }
}

fn add(game: &mut Game, p: PlayerId, zone: Zone, card: CardId) -> ObjectId {
    let id = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        id,
        Object {
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
        },
    );
    match zone {
        Zone::Hand => game.state.players[p as usize].hand.push(id),
        Zone::Library => game.state.players[p as usize].library.push(id),
        Zone::Graveyard => game.state.players[p as usize].graveyard.push(id),
        _ => {}
    }
    id
}

fn hand(game: &mut Game, p: PlayerId, cards: &[CardId]) -> Vec<ObjectId> {
    cards.iter().map(|c| add(game, p, Zone::Hand, *c)).collect()
}

fn battlefield(game: &mut Game, p: PlayerId, cards: &[CardId]) -> Vec<ObjectId> {
    cards
        .iter()
        .map(|c| add(game, p, Zone::Battlefield, *c))
        .collect()
}

fn library(game: &mut Game, p: PlayerId, cards: &[CardId]) {
    for c in cards {
        add(game, p, Zone::Library, *c);
    }
}

fn main_phase(game: &mut Game, active: PlayerId, step: Step) {
    game.state.turn.active = active;
    game.state.turn.step = step;
    game.state.turn.priority = Some(active);
    game.state.turn.passed = 0;
}

fn submit_ok(game: &mut Game, p: PlayerId, action: Action) -> Vec<Event> {
    match game.submit_action(p, action) {
        Ok(e) => e,
        Err(e) => panic!("expected P{p} to act, got {e:?}"),
    }
}

fn submit_illegal(game: &mut Game, p: PlayerId, action: Action, what: &str) {
    match game.submit_action(p, action) {
        Err(EngineError::Illegal { .. }) => println!("  REFUSED as expected: {what}"),
        Ok(_) => panic!("NOT REFUSED: {what}"),
        Err(other) => panic!("refused with wrong error ({what}): {other:?}"),
    }
}

fn submit_refused(game: &mut Game, p: PlayerId, action: Action, what: &str) {
    match game.submit_action(p, action) {
        Err(err) => println!("  REFUSED as expected ({err:?}): {what}"),
        Ok(_) => panic!("NOT REFUSED: {what}"),
    }
}

fn drain_to_pending(game: &mut Game, log: &mut Vec<Event>) {
    for _ in 0..256 {
        if game.state.pending.is_some() || game.state.stack.is_empty() {
            return;
        }
        let Some(p) = game.decision_owner() else {
            return;
        };
        log.extend(submit_ok(game, p, Action::Pass));
    }
    panic!("stack did not settle");
}

fn auto_answer(game: &Game) -> Action {
    if let Some(pending) = &game.state.pending {
        return match pending {
            Pending::Mulligan { .. } => Action::Mulligan { keep: true },
            Pending::DeclareAttackers { .. } => Action::DeclareAttackers {
                attackers: Vec::new(),
                defender: None,
            },
            Pending::DeclareBlockers { .. } => Action::DeclareBlockers { blocks: Vec::new() },
            Pending::ApnapReveal { .. } => Action::RevealCreatureCards { cards: Vec::new() },
            _ => game
                .legal_actions()
                .into_iter()
                .map(|l| l.action)
                .find(|a| !matches!(a, Action::Pass))
                .unwrap_or(Action::Pass),
        };
    }
    Action::Pass
}

fn run_until(game: &mut Game, pred: impl Fn(&Game) -> bool, cap: usize) -> bool {
    for _ in 0..cap {
        if pred(game) {
            return true;
        }
        if game.state.is_over() {
            return false;
        }
        let Some(owner) = game.decision_owner() else {
            return false;
        };
        let action = auto_answer(game);
        if let Err(e) = game.submit_action(owner, action) {
            panic!("driver rejected: {e:?}");
        }
    }
    false
}

fn bears(game: &Game, p: PlayerId) -> Vec<ObjectId> {
    game.state
        .battlefield(p)
        .into_iter()
        .filter(|o| o.card == CardId::BearToken)
        .map(|o| o.id)
        .collect()
}

fn hand_ids(game: &Game, p: PlayerId) -> Vec<ObjectId> {
    game.state.player(p).hand.clone()
}

fn cast_action(game: &Game, card: ObjectId) -> Action {
    game.legal_actions()
        .into_iter()
        .map(|l| l.action)
        .find(|a| matches!(a, Action::CastSpell { card: c, .. } if *c == card))
        .unwrap_or_else(|| panic!("no cast offer for {card}"))
}

fn pending_reveal(game: &Game) -> (PlayerId, Vec<PlayerId>) {
    match &game.state.pending {
        Some(Pending::ApnapReveal { player, order, .. }) => (*player, order.clone()),
        other => panic!("expected ApnapReveal pending, got {other:?}"),
    }
}

fn test_apnap_four_seats_from_active_player() {
    println!("\n[1] Kamahl's Summons: four seats, active = P2 (not P0)");
    let mut g = fixture_n(9101, 4);
    main_phase(&mut g, P2, Step::PrecombatMain);
    // Pre-existing refusal: no reveal pending at all.
    let stray = add(&mut g, P0, Zone::Battlefield, CardId::GrizzlyBears);
    submit_illegal(
        &mut g,
        P2,
        Action::RevealCreatureCards { cards: Vec::new() },
        "reveal with no pending choice",
    );
    let _ = stray;

    library(&mut g, P0, &[CardId::Forest; 6]);
    library(&mut g, P1, &[CardId::Forest; 6]);
    library(&mut g, P2, &[CardId::Forest; 6]);
    library(&mut g, P3, &[CardId::Forest; 6]);

    let p2_hand = hand(
        &mut g,
        P2,
        &[CardId::KamahlsSummons, CardId::EliteVanguard, CardId::Forest],
    );
    let spell = p2_hand[0];
    let p2_creature = p2_hand[1];
    let p2_land = p2_hand[2];
    let p3_hand = hand(&mut g, P3, &[CardId::GrizzlyBears, CardId::WillowElf]);
    let _p0_hand = hand(&mut g, P0, &[CardId::Forest]);
    let p1_hand = hand(&mut g, P1, &[CardId::RuneclawBear, CardId::CentaurCourser]);

    g.state.players[P2 as usize].mana = ManaPool {
        colorless: 3,
        green: 1,
        ..ManaPool::default()
    };

    println!(
        "  hands before cast: P2 {} P3 {} P0 {} P1 {}",
        hand_ids(&g, P2).len(),
        hand_ids(&g, P3).len(),
        hand_ids(&g, P0).len(),
        hand_ids(&g, P1).len()
    );
    let mut log = Vec::new();
    let cast = cast_action(&g, spell);
    log.extend(submit_ok(&mut g, P2, cast));
    drain_to_pending(&mut g, &mut log);

    let (first, order) = pending_reveal(&g);
    println!("  first seat asked: P{first}; order: {order:?}");
    assert_eq!(first, P2, "active player answers first");
    assert_eq!(order, vec![P2, P3, P0, P1], "APNAP from the active player");

    let offers: Vec<Action> = g.legal_actions().into_iter().map(|l| l.action).collect();
    println!("  offers while P2 owes an answer: {offers:?}");
    assert!(offers.contains(&Action::RevealCreatureCards { cards: Vec::new() }));

    // Refusals, all while P2's reveal is pending.
    submit_illegal(
        &mut g,
        P2,
        Action::RevealCreatureCards {
            cards: vec![p2_creature, p2_creature],
        },
        "the same card named twice",
    );
    submit_illegal(
        &mut g,
        P2,
        Action::RevealCreatureCards {
            cards: vec![p2_land],
        },
        "a non-creature card from the chooser's own hand",
    );
    submit_illegal(
        &mut g,
        P2,
        Action::RevealCreatureCards {
            cards: vec![p3_hand[0]],
        },
        "a creature card from another seat's hand",
    );
    submit_refused(
        &mut g,
        P0,
        Action::RevealCreatureCards { cards: Vec::new() },
        "another seat answering a reveal it does not owe",
    );
    assert_eq!(pending_reveal(&g).0, P2, "refusals leave the choice intact");

    // P2 reveals exactly one of its one creature card.
    log.extend(submit_ok(
        &mut g,
        P2,
        Action::RevealCreatureCards {
            cards: vec![p2_creature],
        },
    ));
    assert_eq!(pending_reveal(&g).0, P3);
    assert!(
        bears(&g, P2).is_empty(),
        "no Bear exists before the last answer"
    );

    // P3 reveals both.
    log.extend(submit_ok(
        &mut g,
        P3,
        Action::RevealCreatureCards {
            cards: vec![p3_hand[0], p3_hand[1]],
        },
    ));
    assert_eq!(pending_reveal(&g).0, P0);

    // P0 declines (legal: "may reveal ANY number").
    log.extend(submit_ok(
        &mut g,
        P0,
        Action::RevealCreatureCards { cards: Vec::new() },
    ));
    println!("  P0's empty reveal accepted (declining is legal)");
    assert_eq!(pending_reveal(&g).0, P1);

    // P1 reveals an arbitrary subset: one of its two creature cards.
    log.extend(submit_ok(
        &mut g,
        P1,
        Action::RevealCreatureCards {
            cards: vec![p1_hand[0]],
        },
    ));
    // settle the rest of the resolution (spell finishes, priority returns)
    drain_to_pending(&mut g, &mut log);
    assert!(g.state.pending.is_none(), "queue finished: {:?}", g.state.pending);

    let counts = [P0, P1, P2, P3].map(|p| bears(&g, p).len());
    println!("  Bear tokens by seat P0..P3: {counts:?}");
    assert_eq!(bears(&g, P2).len(), 1, "P2 revealed 1");
    assert_eq!(bears(&g, P3).len(), 2, "P3 revealed 2");
    assert!(bears(&g, P0).is_empty(), "P0 declined");
    assert_eq!(bears(&g, P1).len(), 1, "P1 revealed 1 of its 2 creatures");
    for id in bears(&g, P3).iter().chain(bears(&g, P2).iter()) {
        let o = g.state.object(*id);
        assert_eq!((o.power(), o.toughness()), (2, 2));
        assert_eq!(o.card.def().colors, &[ManaColor::Green]);
        assert!(o.card.def().type_line.contains("Bear"));
    }

    // (iii) revealed cards stay in hand; nothing left any hand.
    for id in [p2_creature, p3_hand[0], p3_hand[1], p1_hand[0]] {
        assert_eq!(
            g.state.object(id).zone,
            Zone::Hand,
            "revealed card {id} must stay in hand"
        );
    }
    println!(
        "  hands after: P2 {} P3 {} P0 {} P1 {} (spell + {})",
        hand_ids(&g, P2).len(),
        hand_ids(&g, P3).len(),
        hand_ids(&g, P0).len(),
        hand_ids(&g, P1).len(),
        "no card moved"
    );
    assert!(hand_ids(&g, P3).contains(&p3_hand[0]));
    assert!(hand_ids(&g, P1).contains(&p1_hand[1]), "unrevealed card still in hand");
    assert!(!hand_ids(&g, P2).contains(&spell), "the cast spell left the hand");

    let order_events: Vec<(PlayerId, u8)> = log
        .iter()
        .filter_map(|e| match e {
            Event::CardsRevealed { player, count, .. } => Some((*player, *count)),
            _ => None,
        })
        .collect();
    println!("  CardsRevealed events: {order_events:?}");
    assert_eq!(order_events, vec![(P2, 1u8), (P3, 2), (P0, 0), (P1, 1)]);

    // (iv) any reveal after the choice is gone is refused.
    let owner = g.decision_owner().expect("priority after resolution");
    submit_illegal(
        &mut g,
        owner,
        Action::RevealCreatureCards { cards: Vec::new() },
        "reveal after the queue resolved",
    );
    println!("  PASS: four-seat APNAP reveal, per-seat counts, cards stayed in hand");
}

fn test_decline_everywhere() {
    println!("\n[2] Kamahl's Summons: every seat declines (3 seats)");
    let mut g = fixture_n(9102, 3);
    for p in [P0, P1, P2] {
        library(&mut g, p, &[CardId::Forest; 6]);
    }
    let h = hand(
        &mut g,
        P0,
        &[CardId::KamahlsSummons, CardId::GrizzlyBears, CardId::Forest],
    );
    let spell = h[0];
    let creature = h[1];
    hand(&mut g, P1, &[CardId::CentaurCourser]);
    hand(&mut g, P2, &[CardId::RuneclawBear]);
    g.state.players[P0 as usize].mana = ManaPool {
        colorless: 3,
        green: 1,
        ..ManaPool::default()
    };
    main_phase(&mut g, P0, Step::PrecombatMain);
    let mut log = Vec::new();
    let cast = cast_action(&g, spell);
    log.extend(submit_ok(&mut g, P0, cast));
    drain_to_pending(&mut g, &mut log);
    for expected in [P0, P1, P2] {
        let (p, _o) = pending_reveal(&g);
        assert_eq!(p, expected);
        log.extend(submit_ok(
            &mut g,
            p,
            Action::RevealCreatureCards { cards: Vec::new() },
        ));
    }
    assert!(g.state.pending.is_none());
    let total: usize = [P0, P1, P2].iter().map(|p| bears(&g, *p).len()).sum();
    println!("  tokens after everyone declined: {total}");
    assert_eq!(total, 0);
    assert_eq!(g.state.object(creature).zone, Zone::Hand);
    println!("  PASS: declining (empty reveal) is legal for every seat");
}

fn test_surrak_trample() {
    println!("\n[3] Surrak and Goreclaw: printed Trample and real trample damage");
    let surrak = catalog::by_name("Surrak and Goreclaw").expect("Surrak supported");
    let def = surrak.def();
    println!(
        "  has_trample={} oracle_first_line={:?}",
        def.has_trample(),
        def.oracle.lines().next()
    );
    assert!(def.has_trample());
    assert_eq!(def.oracle.lines().next(), Some("Trample"));

    let mut g = fixture_n(9103, 2);
    let s = battlefield(&mut g, P0, &[surrak])[0];
    let blocker = battlefield(&mut g, P1, &[CardId::EliteVanguard])[0];
    library(&mut g, P0, &[CardId::Forest; 8]);
    library(&mut g, P1, &[CardId::Forest; 8]);
    main_phase(&mut g, P0, Step::PrecombatMain);
    g.state.players[P0 as usize].mana = ManaPool::default();
    let start_life = g.state.player(P1).life;

    assert!(run_until(
        &mut g,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![s],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut g,
        |g| g.state.turn.step == Step::DeclareBlockers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut g,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                attacker: s,
                blocker,
            }],
        },
    );
    for _ in 0..64 {
        if g.state.player(P1).life < start_life {
            break;
        }
        assert!(!g.state.is_over(), "game ended before damage");
        let Some(owner) = g.decision_owner() else { break };
        submit_ok(&mut g, owner, Action::Pass);
    }
    let dealt = start_life - g.state.player(P1).life;
    println!(
        "  blocker zone={:?}, damage to player={}, attacker damage marked={}",
        g.state.object(blocker).zone,
        dealt,
        g.state.object(s).damage
    );
    assert_eq!(g.state.object(blocker).zone, Zone::Graveyard);
    assert_eq!(dealt, 5, "6/5: 1 lethal to the 2/1, 5 tramples over");
    println!("  PASS: trample assigns lethal then tramples over");
}

fn test_mandatory_target() {
    println!("\n[4] mandatory target trigger: declining neither offered nor legal");
    let mut g = fixture_n(9104, 2);
    let h = hand(&mut g, P0, &[CardId::LittleBear]);
    let little = h[0];
    let other = battlefield(&mut g, P0, &[CardId::GrizzlyBears, CardId::EliteVanguard])[0];
    library(&mut g, P0, &[CardId::Forest; 8]);
    library(&mut g, P1, &[CardId::Forest; 8]);
    g.state.players[P0 as usize].mana = ManaPool {
        colorless: 2,
        green: 1,
        ..ManaPool::default()
    };
    main_phase(&mut g, P0, Step::PrecombatMain);
    let cast = cast_action(&g, little);
    submit_ok(&mut g, P0, cast);
    // resolve until the enters trigger asks for a target
    for i in 0..64 {
        println!(
            "  iter {i}: step={:?} pending={:?} stack={} owner={:?}",
            g.state.turn.step,
            g.state.pending,
            g.state.stack.len(),
            g.decision_owner()
        );
        if matches!(g.state.pending, Some(Pending::TriggerTarget { .. })) {
            break;
        }
        let Some(owner) = g.decision_owner() else { break };
        let answer = auto_answer(&g);
        submit_ok(&mut g, owner, answer);
    }
    match &g.state.pending {
        Some(Pending::TriggerTarget { player, .. }) => println!("  trigger waits for P{player}"),
        other => panic!("expected a mandatory target trigger, got {other:?}"),
    }
    let offers: Vec<Action> = g.legal_actions().into_iter().map(|l| l.action).collect();
    println!("  offers: {offers:?}");
    assert!(
        offers.contains(&Action::ChooseTriggerTarget {
            target: Target::Permanent(other)
        }),
        "the other creature is a legal target"
    );
    assert!(
        !offers.contains(&Action::DeclineTriggerTarget),
        "DeclineTriggerTarget must not be offered"
    );
    submit_illegal(&mut g, P0, Action::DeclineTriggerTarget, "declining a mandatory target");
    assert!(matches!(
        g.state.pending,
        Some(Pending::TriggerTarget { .. })
    ));
    println!("  PASS: not offered, and refused as EngineError::Illegal");
}

fn test_catalog_shape() {
    println!("\n[5] catalog shape");
    println!("  pool_size = {}", catalog::pool_size());
    assert_eq!(catalog::pool_size(), 148);
    let k = catalog::by_name("Kamahl's Summons").expect("Kamahl in pool");
    let d = k.def();
    println!("  Kamahl: {:?} {:?} effects={:?}", d.cost, d.type_line, d.spell_spec().map(|s| s.effects));
    assert_eq!(d.cost, Some("{3}{G}"));
    assert_eq!(d.type_line, "Sorcery");
    assert_eq!(
        d.spell_spec().unwrap().effects,
        &[Effect::EachPlayerMayRevealCreatureCardsForBears]
    );
    for name in ["Runeclaw Bear", "Centaur Courser", "Willow Elf", "Elite Vanguard"] {
        assert!(catalog::by_name(name).is_some(), "{name} supported");
    }
    println!("  PASS: catalog exposes Kamahl's Summons with the reveal effect");
}

fn main() {
    test_catalog_shape();
    test_apnap_four_seats_from_active_player();
    test_decline_everywhere();
    test_surrak_trample();
    test_mandatory_target();
    println!("\nALL PROBES PASSED");
}
