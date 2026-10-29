//! Slice 32: Kamahl's Summons (ONS 269) and the Surrak and Goreclaw printed
//! keyword repair, plus the committed regression for a mandatory target trigger.
//!
//! Kamahl's Summons is printed (ONS 269, retrieved 2026-10-27, recorded in
//! `fixtures/starter-corpus/slice32-sourced.json`):
//!
//! "Each player may reveal any number of creature cards from their hand. Then
//! each player creates a 2/2 green Bear creature token for each card they
//! revealed this way."
//!
//! The choice is a per-seat arbitrary subset of that seat's own creature cards
//! (CR 701.9a), made in APNAP order from the active player (CR 101.4, 608.2f).
//! Nothing is applied per answer: every seat's count is fixed by its own reveal
//! and the tokens are created together once the last seat has answered. Revealed
//! cards never leave their owner's hand.
mod common;

use aperture_core::ability;
use aperture_core::catalog;
use aperture_core::*;
use common::*;

/// The crate exports `P0`/`P1` only; slice 32 needs a third seat.
const P2: PlayerId = 2;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn kamahl() -> CardId {
    card("Kamahl's Summons")
}

fn pool(generic: u32, green: u32) -> ManaPool {
    ManaPool {
        colorless: generic,
        green,
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

fn bears(g: &Game, player: PlayerId) -> Vec<ObjectId> {
    g.state
        .battlefield(player)
        .into_iter()
        .filter(|object| object.card == CardId::BearToken)
        .map(|object| object.id)
        .collect()
}

fn zones(g: &Game, ids: &[ObjectId]) -> Vec<Zone> {
    ids.iter().map(|id| g.state.object(*id).zone).collect()
}

#[test]
fn kamahls_summons_is_sourced_and_supported() {
    let def = kamahl().def();
    assert_eq!(def.name, "Kamahl's Summons");
    assert_eq!(def.cost, Some("{3}{G}"));
    assert_eq!(def.type_line, "Sorcery");
    assert!(def.is_sorcery() && !def.is_creature());
    assert_eq!(
        def.oracle,
        "Each player may reveal any number of creature cards from their hand. Then each player creates a 2/2 green Bear creature token for each card they revealed this way."
    );
    let spec = def.spell_spec().expect("a sorcery has a spell spec");
    assert_eq!(spec.target, TargetSpec::None);
    assert!(!spec.instant, "a sorcery is cast at sorcery timing");
    assert_eq!(
        spec.effects,
        &[Effect::EachPlayerMayRevealCreatureCardsForBears]
    );
    assert!(
        ability::abilities(kamahl()).is_empty(),
        "no printed activated or triggered ability"
    );
}

#[test]
fn every_seat_reveals_in_apnap_order_and_creates_that_many_bears() {
    let mut g = fixture_n(3201, 3)
        .hand(P0, &[kamahl(), CardId::EliteVanguard])
        .hand(P1, &[CardId::GrizzlyBears, CardId::WillowElf])
        .hand(P2, &[CardId::Forest])
        .mana(P0, pool(3, 1))
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .library(P2, &[CardId::Forest; 6])
        .main(P0, Step::PrecombatMain)
        .build();
    let spell = hand_card(&g, P0, kamahl());
    let p0_creature = hand_card(&g, P0, CardId::EliteVanguard);
    let p1_creature_a = hand_card(&g, P1, CardId::GrizzlyBears);
    let p1_creature_b = hand_card(&g, P1, CardId::WillowElf);
    let p2_land = hand_card(&g, P2, CardId::Forest);

    let cast = g
        .legal_actions()
        .into_iter()
        .find(|legal| matches!(&legal.action, Action::CastSpell { card, .. } if *card == spell))
        .expect("the sorcery is castable with {3}{G}")
        .action;
    submit_ok(&mut g, P0, cast);
    let mut log = drain(&mut g);
    assert!(
        matches!(
            g.state.pending,
            Some(Pending::ApnapReveal { player: P0, .. })
        ),
        "the active player answers first: {:?}",
        g.state.pending
    );

    // Refusals while the first seat's reveal is pending.
    let mut too_many = vec![p0_creature];
    too_many.push(p0_creature);
    for (label, cards) in [
        ("a duplicate card", too_many),
        ("a land card", vec![p2_land]),
        ("an opponent's creature card", vec![p1_creature_a]),
    ] {
        match submit_err(&mut g, P0, Action::RevealCreatureCards { cards }) {
            EngineError::Illegal { .. } => {}
            other => panic!("{label} was not refused: {other:?}"),
        }
        assert!(
            matches!(
                g.state.pending,
                Some(Pending::ApnapReveal { player: P0, .. })
            ),
            "a refused reveal leaves the pending choice untouched"
        );
    }

    // Offers: "reveal nothing" and "reveal every creature card in hand".
    let offers: Vec<Action> = g.legal_actions().into_iter().map(|a| a.action).collect();
    assert!(offers.contains(&Action::RevealCreatureCards { cards: Vec::new() }));
    assert!(offers.contains(&Action::RevealCreatureCards {
        cards: vec![p0_creature]
    }));

    log.extend(submit_ok(
        &mut g,
        P0,
        Action::RevealCreatureCards {
            cards: vec![p0_creature],
        },
    ));
    assert!(
        matches!(
            g.state.pending,
            Some(Pending::ApnapReveal { player: P1, .. })
        ),
        "P1 answers second: {:?}",
        g.state.pending
    );
    assert!(
        bears(&g, P0).is_empty(),
        "no token is created while seats still owe an answer"
    );
    log.extend(submit_ok(
        &mut g,
        P1,
        Action::RevealCreatureCards {
            cards: vec![p1_creature_a, p1_creature_b],
        },
    ));
    assert!(
        matches!(
            g.state.pending,
            Some(Pending::ApnapReveal { player: P2, .. })
        ),
        "P2 answers last: {:?}",
        g.state.pending
    );
    log.extend(submit_ok(
        &mut g,
        P2,
        Action::RevealCreatureCards { cards: Vec::new() },
    ));
    log.extend(drain(&mut g));

    assert_eq!(bears(&g, P0).len(), 1, "P0 revealed one creature card");
    assert_eq!(bears(&g, P1).len(), 2, "P1 revealed two");
    assert!(bears(&g, P2).is_empty(), "P2 revealed none");
    for id in bears(&g, P1) {
        let object = g.state.object(id);
        assert_eq!((object.power(), object.toughness()), (2, 2));
        assert!(g.is_creature(object), "the token is a creature");
        assert_eq!(object.card.def().colors, &[ManaColor::Green]);
        assert!(
            object.card.def().type_line.contains("Bear"),
            "the token is a Bear"
        );
    }
    assert_eq!(
        zones(&g, &[p0_creature, p1_creature_a, p1_creature_b]),
        vec![Zone::Hand, Zone::Hand, Zone::Hand],
        "revealing a card does not move it (CR 701.9a)"
    );
    let order: Vec<PlayerId> = log
        .iter()
        .filter_map(|event| match event {
            Event::CardsRevealed { player, .. } => Some(*player),
            _ => None,
        })
        .collect();
    assert_eq!(order, vec![P0, P1, P2], "answers are public in APNAP order");
    let counts: Vec<u8> = log
        .iter()
        .filter_map(|event| match event {
            Event::CardsRevealed { count, .. } => Some(*count),
            _ => None,
        })
        .collect();
    assert_eq!(counts, vec![1, 2, 0]);
    assert!(
        g.state.pending.is_none(),
        "the spell finishes after the last answer"
    );
    // Nothing else can answer a reveal that is no longer pending.
    match submit_err(
        &mut g,
        P0,
        Action::RevealCreatureCards { cards: Vec::new() },
    ) {
        EngineError::Illegal { .. } => {}
        other => panic!("a reveal outside its pending choice was not refused: {other:?}"),
    }
}

#[test]
fn a_mandatory_target_trigger_never_offers_or_accepts_declining() {
    let mut g = fixture(3202)
        .hand(P0, &[CardId::LittleBear])
        .battlefield(P0, &[CardId::GrizzlyBears, CardId::EliteVanguard])
        .mana(P0, pool(3, 1))
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .main(P0, Step::PrecombatMain)
        .build();
    let little_bear = hand_card(&g, P0, CardId::LittleBear);
    let other = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let cast = g
        .legal_actions()
        .into_iter()
        .find(
            |legal| matches!(&legal.action, Action::CastSpell { card, .. } if *card == little_bear),
        )
        .expect("Little Bear is castable with {2}{G}")
        .action;
    submit_ok(&mut g, P0, cast);
    let log = drain(&mut g);
    assert!(
        matches!(
            g.state.pending,
            Some(Pending::TriggerTarget { player: P0, .. })
        ),
        "the mandatory enters trigger waits for its target: {:?} (log {:?})",
        g.state.pending,
        log
    );
    let offers: Vec<Action> = g.legal_actions().into_iter().map(|a| a.action).collect();
    assert!(
        offers.contains(&Action::ChooseTriggerTarget {
            target: Target::Permanent(other)
        }),
        "the other creature is a legal target"
    );
    assert!(
        !offers.contains(&Action::DeclineTriggerTarget),
        "a mandatory target is never offered as declinable (CR 115.6)"
    );
    match submit_err(&mut g, P0, Action::DeclineTriggerTarget) {
        EngineError::Illegal { .. } => {}
        other => panic!("declining a mandatory target was not refused: {other:?}"),
    }
    assert!(
        matches!(
            g.state.pending,
            Some(Pending::TriggerTarget { player: P0, .. })
        ),
        "the refusal leaves the trigger waiting for its target"
    );
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(other),
        },
    );
    drain(&mut g);
    assert!(g.state.pending.is_none(), "the trigger resolves normally");
}

#[test]
fn surrak_and_goreclaw_prints_its_own_trample() {
    let def = card("Surrak and Goreclaw").def();
    assert_eq!(
        def.oracle.lines().next(),
        Some("Trample"),
        "the printed keyword is the first line"
    );
    assert!(
        def.has_trample(),
        "an included card's own printed Trample must be modelled"
    );

    let mut g = fixture(3203)
        .battlefield(P0, &[card("Surrak and Goreclaw")])
        .battlefield(P1, &[CardId::EliteVanguard])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let surrak = battlefield_card(&g, P0, card("Surrak and Goreclaw"));
    let blocker = battlefield_card(&g, P1, CardId::EliteVanguard);
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![surrak],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareBlockers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut g,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                attacker: surrak,
                blocker,
            }],
        },
    );
    let life_before = g.state.player(P1).life;
    for _ in 0..64 {
        if g.state.player(P1).life < life_before {
            break;
        }
        assert!(!g.state.is_over(), "the game ended before combat damage");
        let Some(owner) = g.decision_owner() else {
            break;
        };
        submit_ok(&mut g, owner, Action::Pass);
    }
    assert_eq!(
        g.state.object(blocker).zone,
        Zone::Graveyard,
        "lethal damage to the 2/1 blocker"
    );
    assert_eq!(
        g.state.player(P1).life,
        life_before - 5,
        "6/5 trample assigns 1 lethal damage to the 2/1 blocker and 5 tramples over (CR 702.19b)"
    );
}
