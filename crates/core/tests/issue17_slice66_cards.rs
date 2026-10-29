//! Slice 66 card behaviour for the two newly enabled "Sacrifice a Goblin"
//! cards, Pashalik Mons (DMR 133) and Goblin Trashmaster (M19 144).
//!
//! Every assertion is on observable engine behaviour: real zone changes, real
//! trigger resolution, real mana payment, and the *offered* action surface —
//! never on a hard-coded branch.
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

fn offers_for(game: &Game, player: PlayerId, source: ObjectId, ability: u8) -> Vec<LegalAction> {
    game.legal_actions()
        .into_iter()
        .filter(|entry| {
            matches!(
                entry.action,
                Action::ActivateCostedAbility { source: s, ability: a, .. }
                    if s == source && a == ability
            )
        })
        .inspect(|entry| {
            assert!(
                game.validate(player, &entry.action).is_ok(),
                "every offer must be legal: {:?}",
                entry.label
            )
        })
        .collect()
}

/// A permanent that died is either in its owner's graveyard or — when it was a
/// token — removed from the state entirely (CR 111.7).
fn died(game: &Game, id: ObjectId) -> bool {
    game.state
        .try_object(id)
        .is_none_or(|object| object.zone == Zone::Graveyard)
}

/// Pashalik Mons' dies watcher is narrowed to Goblins its controller controls,
/// and it includes the source itself.
#[test]
fn pashalik_pings_for_goblins_it_controls_and_for_itself_but_not_for_a_bear() {
    // (a) another Goblin you control dies -> one damage to any target.
    let mut game = fixture(6611)
        .battlefield(P0, &[CardId::PashalikMons, CardId::GoblinToken])
        .hand(P1, &[CardId::LightningStrike])
        .mana(P1, red(1, 1))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let token = battlefield_card(&game, P0, CardId::GoblinToken);
    let life_before = game.state.players[usize::from(OPPONENT)].life;
    let mut log = submit_ok(&mut game, P0, Action::Pass);
    let strike = hand_card(&game, OPPONENT, CardId::LightningStrike);
    let mut strike_log = submit_ok(
        &mut game,
        OPPONENT,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Permanent(token)],
        },
    );
    run_until(
        &mut game,
        &mut strike_log,
        |g| matches!(g.state.pending, Some(Pending::TriggerTarget { .. })),
        8000,
    );
    assert!(died(&game, token), "the Goblin really died");
    let target = game
        .legal_actions()
        .into_iter()
        .find_map(|entry| match entry.action {
            Action::ChooseTriggerTarget {
                target: Target::Player(OPPONENT),
            } => Some(Target::Player(OPPONENT)),
            _ => None,
        })
        .expect("Pashalik's trigger offers a player target");
    submit_ok(&mut game, P0, Action::ChooseTriggerTarget { target });
    run_until(
        &mut game,
        &mut strike_log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        game.state.players[usize::from(OPPONENT)].life,
        life_before - 1,
        "Pashalik deals exactly 1 damage when another Goblin you control dies"
    );
    let _ = &mut log;

    // (b) Pashalik itself dies -> its own death triggers it (CR 603.6c).
    let mut game = fixture(6612)
        .battlefield(P0, &[CardId::PashalikMons])
        .hand(P1, &[CardId::LightningStrike, CardId::LightningStrike])
        .mana(P1, red(2, 2))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let mons = battlefield_card(&game, P0, CardId::PashalikMons);
    let life_before = game.state.players[usize::from(OPPONENT)].life;
    submit_ok(&mut game, P0, Action::Pass);
    let strike = hand_card(&game, OPPONENT, CardId::LightningStrike);
    let mut strike_log = submit_ok(
        &mut game,
        OPPONENT,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Permanent(mons)],
        },
    );
    run_until(
        &mut game,
        &mut strike_log,
        |g| matches!(g.state.pending, Some(Pending::TriggerTarget { .. })),
        8000,
    );
    assert!(
        died(&game, mons),
        "Pashalik is in the graveyard, so this is its own death"
    );
    let target = game
        .legal_actions()
        .into_iter()
        .find_map(|entry| match entry.action {
            Action::ChooseTriggerTarget {
                target: Target::Player(OPPONENT),
            } => Some(Target::Player(OPPONENT)),
            _ => None,
        })
        .expect("the printed 'Pashalik Mons or another Goblin' includes itself");
    submit_ok(&mut game, P0, Action::ChooseTriggerTarget { target });
    run_until(
        &mut game,
        &mut strike_log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        game.state.players[usize::from(OPPONENT)].life,
        life_before - 1,
        "its own death triggers the watcher"
    );

    // (c) a non-Goblin you control dying and (d) an opponent's Goblin dying
    // both leave the watcher alone.
    let mut game = fixture(6613)
        .battlefield(P0, &[CardId::PashalikMons, CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::GoblinToken])
        .hand(P0, &[CardId::LightningStrike, CardId::LightningStrike])
        .mana(P0, red(2, 2))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let theirs = battlefield_card(&game, P1, CardId::GoblinToken);
    let life_before = game.state.players[usize::from(OPPONENT)].life;
    let strike = hand_card(&game, P0, CardId::LightningStrike);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Permanent(bears)],
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert!(died(&game, bears), "the Bear died");
    assert_eq!(
        game.state.players[usize::from(OPPONENT)].life,
        life_before,
        "a dying Bear is not a dying Goblin, so no damage is dealt"
    );

    let strike = hand_card(&game, P0, CardId::LightningStrike);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Permanent(theirs)],
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert!(died(&game, theirs), "the opponent's Goblin died");
    assert_eq!(
        game.state.players[usize::from(OPPONENT)].life,
        life_before,
        "'another Goblin YOU control dies' is a controller restriction, not just a subtype one"
    );
}

/// Pashalik's activated ability pays `{3}{R}` and a *chosen* Goblin, then makes
/// exactly two 1/1 Goblin tokens.
#[test]
fn pashalik_sacrifices_a_chosen_goblin_and_creates_two_tokens() {
    let mut game = fixture(6614)
        .battlefield(
            P0,
            &[
                CardId::PashalikMons,
                CardId::GoblinToken,
                CardId::GrizzlyBears,
            ],
        )
        .battlefield(P1, &[CardId::GoblinToken])
        .mana(P0, red(3, 1))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let mons = battlefield_card(&game, P0, CardId::PashalikMons);
    let fodder = battlefield_card(&game, P0, CardId::GoblinToken);
    let offers = offers_for(&game, P0, mons, 1);
    let mut named: Vec<ObjectId> = offers
        .iter()
        .filter_map(|entry| match &entry.action {
            Action::ActivateCostedAbility { cost_cards, .. } => Some(cost_cards[0]),
            _ => None,
        })
        .collect();
    named.sort_by_key(|id| id.0);
    named.dedup();
    assert_eq!(
        named,
        vec![mons, fodder],
        "one offer per Goblin P0 controls — never the opponent's Goblin, never the Bear"
    );

    let before = game
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::GoblinToken)
        .count();
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::ActivateCostedAbility {
            source: mons,
            ability: 1,
            mode: 0,
            targets: vec![],
            cost_cards: vec![fodder],
        },
    );
    assert!(
        died(&game, fodder),
        "the chosen Goblin is sacrificed as the cost is paid"
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    let after = game
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::GoblinToken)
        .count();
    assert_eq!(
        after,
        before - 1 + 2,
        "one token was sacrificed and two were created"
    );
    assert_eq!(
        game.state.player(P0).mana.total(),
        0,
        "the whole {{3}}{{R}} was paid"
    );
}

/// A forged sacrifice that names a non-Goblin or the opponent's Goblin is
/// refused, and refusal leaves the board and the pool untouched.
#[test]
fn pashalik_refuses_a_forged_sacrifice_without_mutating_state() {
    let mut game = fixture(6615)
        .battlefield(P0, &[CardId::PashalikMons, CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::GoblinToken])
        .mana(P0, red(3, 1))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let mons = battlefield_card(&game, P0, CardId::PashalikMons);
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let theirs = battlefield_card(&game, P1, CardId::GoblinToken);
    let objects_before = game.state.objects.len();
    for forged in [bears, theirs] {
        let err = submit_err(
            &mut game,
            P0,
            Action::ActivateCostedAbility {
                source: mons,
                ability: 1,
                mode: 0,
                targets: vec![],
                cost_cards: vec![forged],
            },
        );
        assert!(
            format!("{err}").contains("sacrifice"),
            "a non-Goblin or a foreign Goblin cannot pay the cost: {err}"
        );
    }
    assert_eq!(
        game.state.objects.len(),
        objects_before,
        "no refusal changed a zone"
    );
    assert!(game.state.stack.is_empty(), "nothing was put on the stack");
    assert_eq!(game.state.player(P0).mana.total(), 4, "no mana was spent");
}

/// Goblin Trashmaster's anthem is "other Goblins you control", so it never
/// pumps itself and never pumps a non-Goblin.
#[test]
fn trashmaster_pumps_other_goblins_only() {
    let mut game = fixture(6616)
        .battlefield(
            P0,
            &[
                CardId::GoblinTrashmaster,
                CardId::GoblinToken,
                CardId::GrizzlyBears,
            ],
        )
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    // Layer 7c statics are recomputed when priority moves, so round-trip one
    // pass through the opponent before reading the pumped characteristics.
    let mut log = submit_ok(&mut game, P0, Action::Pass);
    run_until(&mut game, &mut log, |g| g.decision_owner() == Some(P0), 64);
    let trash = battlefield_card(&game, P0, CardId::GoblinTrashmaster);
    let token = battlefield_card(&game, P0, CardId::GoblinToken);
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    assert_eq!(
        (
            game.state.object(trash).power(),
            game.state.object(trash).toughness()
        ),
        (3, 3),
        "printed 3/3: 'other' excludes the Trashmaster itself"
    );
    assert_eq!(
        (
            game.state.object(token).power(),
            game.state.object(token).toughness()
        ),
        (2, 2),
        "the 1/1 Goblin token is pumped by the anthem"
    );
    assert_eq!(
        (
            game.state.object(bears).power(),
            game.state.object(bears).toughness()
        ),
        (2, 2),
        "a non-Goblin is untouched"
    );
}

/// Goblin Trashmaster: "Sacrifice a Goblin: Destroy target artifact" — a
/// chosen Goblin cost, an artifact-only target, and a real destruction.
#[test]
fn trashmaster_destroys_a_targeted_artifact_and_refuses_an_enchantment() {
    let mut game = fixture(6617)
        .battlefield(P0, &[CardId::GoblinTrashmaster, CardId::GoblinToken])
        // The artifact belongs to the opponent: "target artifact" is not
        // restricted to the activator's permanents.
        .battlefield(P1, &[CardId::ArcaneSignet, CardId::WordsOfWilding])
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let trash = battlefield_card(&game, P0, CardId::GoblinTrashmaster);
    let fodder = battlefield_card(&game, P0, CardId::GoblinToken);
    let signet = battlefield_card(&game, P1, CardId::ArcaneSignet);
    let enchantment = battlefield_card(&game, P1, CardId::WordsOfWilding);

    let offers = offers_for(&game, P0, trash, 0);
    assert!(
        offers.iter().any(|entry| matches!(
            &entry.action,
            Action::ActivateCostedAbility { cost_cards, targets, .. }
                if *cost_cards == vec![fodder] && *targets == vec![Target::Permanent(signet)]
        )),
        "the offer names the Goblin and the opponent's artifact: {:?}",
        offers.iter().map(|e| e.label.clone()).collect::<Vec<_>>()
    );
    assert!(
        !offers.iter().any(|entry| matches!(
            &entry.action,
            Action::ActivateCostedAbility { targets, .. }
                if *targets == vec![Target::Permanent(enchantment)]
        )),
        "an enchantment is never offered as a 'target artifact'"
    );

    // A forged enchantment target is refused by validate, and the refusal does
    // not sacrifice anything.
    let err = submit_err(
        &mut game,
        P0,
        Action::ActivateCostedAbility {
            source: trash,
            ability: 0,
            mode: 0,
            targets: vec![Target::Permanent(enchantment)],
            cost_cards: vec![fodder],
        },
    );
    assert!(
        format!("{err}").contains("artifact"),
        "the enchantment target is refused: {err}"
    );
    assert!(
        game.state.object(fodder).zone == Zone::Battlefield,
        "a refused activation does not pay its cost"
    );

    let mut log = submit_ok(
        &mut game,
        P0,
        Action::ActivateCostedAbility {
            source: trash,
            ability: 0,
            mode: 0,
            targets: vec![Target::Permanent(signet)],
            cost_cards: vec![fodder],
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        game.state.object(signet).zone,
        Zone::Graveyard,
        "the artifact was destroyed and moved to its owner's graveyard"
    );
    assert!(
        died(&game, fodder),
        "the Goblin was sacrificed to pay the cost"
    );
    assert!(
        game.state.object(trash).zone == Zone::Battlefield,
        "the Trashmaster itself is still on the battlefield"
    );
    assert!(
        game.state.object(enchantment).zone == Zone::Battlefield,
        "the enchantment was never touched"
    );
}
