//! Slice 65 behaviour: the first demanded "Sacrifice a Goblin" card plus the
//! review-364 functional repair.
//!
//! Covers two things that must not regress independently:
//!
//! * R364-1 — Krenko, Tin Street Kingpin's token count reads its LAST KNOWN
//!   power when the source died while its own attack trigger was on the stack
//!   (CR 608.2h), instead of falling back to zero.
//! * Siege-Gang Commander — a real "enters with three Goblin tokens" trigger and
//!   a `{1}{R}, Sacrifice a Goblin: deals 2 damage to any target` activated
//!   ability whose cost is a chosen creature the activator controls.
mod common;
use aperture_core::*;
use common::*;

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

/// Every legal offer for the Commander's activated ability (index 1).
fn commander_offers(game: &Game) -> Vec<LegalAction> {
    let commander = battlefield_card(game, P0, CardId::SiegeGangCommander);
    game.legal_actions()
        .into_iter()
        .filter(|entry| {
            matches!(
                entry.action,
                Action::ActivateCostedAbility { source, ability: 1, .. } if source == commander
            )
        })
        .collect()
}

/// R364-1. Krenko attacks, the opponent kills it with Lightning Strike while the
/// attack trigger waits on the stack, and the trigger still uses Krenko's last
/// known power (1) rather than creating nothing.
#[test]
fn krenko_tokens_use_last_known_power_when_it_dies_before_the_trigger_resolves() {
    // Control: the same attack with no interference makes two tokens.
    let mut control = fixture(6501)
        .battlefield(P0, &[CardId::KrenkoTinStreetKingpin])
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::DeclareAttackers)
        .build();
    control.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let krenko = battlefield_card(&control, P0, CardId::KrenkoTinStreetKingpin);
    let mut log = submit_ok(
        &mut control,
        P0,
        Action::DeclareAttackers {
            attackers: vec![krenko],
            defender: Some(OPPONENT),
        },
    );
    run_until(
        &mut control,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        goblin_tokens(&control, P0).len(),
        2,
        "undisturbed: Krenko is 2/2 after its own counter, so two tokens"
    );

    // The defect: kill Krenko while the trigger is on the stack.
    let mut game = fixture(6502)
        .battlefield(P0, &[CardId::KrenkoTinStreetKingpin])
        .hand(P1, &[CardId::LightningStrike])
        .mana(P1, red(1, 1))
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
            defender: Some(OPPONENT),
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| !g.state.stack.is_empty() && g.decision_owner() == Some(P0),
        4000,
    );
    // Priority passes to the defending seat, which kills the source.
    submit_ok(&mut game, P0, Action::Pass);
    run_until(
        &mut game,
        &mut log,
        |g| g.decision_owner() == Some(OPPONENT),
        4000,
    );
    let strike = hand_card(&game, OPPONENT, CardId::LightningStrike);
    let mut strike_log = submit_ok(
        &mut game,
        OPPONENT,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Permanent(krenko)],
        },
    );
    run_until(
        &mut game,
        &mut strike_log,
        |g| {
            g.state.try_object(krenko).is_none()
                && g.state.stack.is_empty()
                && g.state.pending.is_none()
        },
        8000,
    );
    assert_eq!(
        game.state.object(krenko).zone,
        Zone::Graveyard,
        "Lightning Strike really killed Krenko before its trigger resolved"
    );
    assert_eq!(
        goblin_tokens(&game, P0).len(),
        1,
        "CR 608.2h: the trigger uses Krenko's last known power 1, so one token"
    );
}

#[test]
fn siege_gang_commander_enters_with_three_goblin_tokens() {
    let mut game = fixture(6503)
        .hand(P0, &[CardId::SiegeGangCommander])
        .mana(P0, red(3, 2))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let card = hand_card(&game, P0, CardId::SiegeGangCommander);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    let commander = battlefield_card(&game, P0, CardId::SiegeGangCommander);
    assert_eq!(game.state.object(commander).power(), 2);
    assert_eq!(
        goblin_tokens(&game, P0).len(),
        3,
        "the enters trigger creates exactly three Goblin tokens"
    );
    assert!(
        game.has_subtype(game.state.object(commander), "Goblin"),
        "the Commander prints the Goblin subtype, so it can sacrifice itself"
    );
}

#[test]
fn sacrifice_cost_offers_one_action_per_legal_goblin_and_every_offer_validates() {
    let mut game = fixture(6504)
        .battlefield(
            P0,
            &[
                CardId::SiegeGangCommander,
                CardId::GoblinToken,
                CardId::GrizzlyBears,
            ],
        )
        .battlefield(P1, &[CardId::GoblinToken])
        .mana(P0, red(1, 1))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let offers = commander_offers(&game);
    assert!(
        !offers.is_empty(),
        "the ability is offered with a Goblin out"
    );

    let mut named: Vec<ObjectId> = Vec::new();
    for entry in &offers {
        assert!(
            game.validate(P0, &entry.action).is_ok(),
            "every offer is legal: {:?}",
            entry.label
        );
        if let Action::ActivateCostedAbility { cost_cards, .. } = &entry.action {
            assert_eq!(cost_cards.len(), 1, "exactly one Goblin is the cost");
            named.push(cost_cards[0]);
        }
    }
    named.sort_by_key(|id| id.0);
    named.dedup();
    let commander = battlefield_card(&game, P0, CardId::SiegeGangCommander);
    let own_goblin = battlefield_card(&game, P0, CardId::GoblinToken);
    assert_eq!(
        named,
        {
            let mut v = vec![commander, own_goblin];
            v.sort_by_key(|id| id.0);
            v
        },
        "one offer per Goblin P0 controls — and never the opponent's Goblin"
    );

    let foreign = battlefield_card(&game, P1, CardId::GoblinToken);
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    for forged in [foreign, bears] {
        let err = submit_err(
            &mut game,
            P0,
            Action::ActivateCostedAbility {
                source: commander,
                ability: 1,
                mode: 0,
                targets: vec![Target::Player(OPPONENT)],
                cost_cards: vec![forged],
            },
        );
        assert!(
            format!("{err}").contains("sacrifice"),
            "a foreign or non-Goblin permanent cannot pay the cost: {err}"
        );
    }
}

#[test]
fn sacrificing_the_named_goblin_pays_the_cost_and_deals_two_damage() {
    let mut game = fixture(6505)
        .battlefield(P0, &[CardId::SiegeGangCommander, CardId::GoblinToken])
        .mana(P0, red(1, 1))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let commander = battlefield_card(&game, P0, CardId::SiegeGangCommander);
    let fodder = battlefield_card(&game, P0, CardId::GoblinToken);
    let life_before = game.state.players[usize::from(OPPONENT)].life;
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::ActivateCostedAbility {
            source: commander,
            ability: 1,
            mode: 0,
            targets: vec![Target::Player(OPPONENT)],
            cost_cards: vec![fodder],
        },
    );
    assert!(
        game.state.try_object(fodder).is_none(),
        "the cost is paid as part of activating, so the Goblin is already gone"
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        game.state.players[usize::from(OPPONENT)].life,
        life_before - 2,
        "the ability deals exactly 2 damage"
    );
    assert!(
        game.state.try_object(commander).is_some(),
        "the Commander itself was not the sacrificed Goblin"
    );
}

#[test]
fn the_commander_may_sacrifice_itself_and_the_damage_still_resolves() {
    // CR 118.3 + 608.2h: the source is the sacrificed Goblin, so it is already in
    // the graveyard when the ability resolves and the damage uses its last known
    // controller to reach the opponent.
    let mut game = fixture(6506)
        .battlefield(P0, &[CardId::SiegeGangCommander])
        .mana(P0, red(1, 1))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let commander = battlefield_card(&game, P0, CardId::SiegeGangCommander);
    let life_before = game.state.players[usize::from(OPPONENT)].life;
    let offers = commander_offers(&game);
    let mut cost_sets: Vec<Vec<ObjectId>> = offers
        .iter()
        .filter_map(|entry| match &entry.action {
            Action::ActivateCostedAbility { cost_cards, .. } => Some(cost_cards.clone()),
            _ => None,
        })
        .collect();
    cost_sets.sort_by_key(|set| set[0].0);
    cost_sets.dedup();
    assert_eq!(
        cost_sets,
        vec![vec![commander]],
        "the only Goblin P0 controls is the Commander itself"
    );
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::ActivateCostedAbility {
            source: commander,
            ability: 1,
            mode: 0,
            targets: vec![Target::Player(OPPONENT)],
            cost_cards: vec![commander],
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        game.state.object(commander).zone,
        Zone::Graveyard,
        "the Commander was sacrificed to pay its own cost"
    );
    assert_eq!(
        game.state.players[usize::from(OPPONENT)].life,
        life_before - 2,
        "the ability still resolves and deals 2 damage"
    );
}
