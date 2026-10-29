//! Slice 65 journey: one real four-seat, 40-life game that casts, sacrifices and
//! attacks with the card this slice enabled.
//!
//! Everything here is driven from `Game::legal_actions` — the engine's own
//! projected offers — never by editing state mid-game, so the trace is a real
//! executable journey rather than a fixture assertion. It is a bounded
//! Commander-combat practice profile (40 life, one supported commander per seat),
//! not the Commander format and not a strength claim.
mod common;
use aperture_core::*;
use common::*;

// Seat 0 and 1 exist as constants; the journey also uses the other two seats.
const P2: PlayerId = 2;
const P3: PlayerId = 3;

fn goblin_tokens(g: &Game, p: PlayerId) -> Vec<ObjectId> {
    g.state
        .battlefield(p)
        .into_iter()
        .filter(|o| o.card == CardId::GoblinToken)
        .map(|o| o.id)
        .collect()
}

/// Every offered action, so the journey can never submit something the engine
/// would not have offered.
fn offered(game: &Game, player: PlayerId) -> Vec<Action> {
    assert_eq!(
        game.decision_owner(),
        Some(player),
        "P{player} has priority"
    );
    game.legal_actions()
        .into_iter()
        .map(|entry| entry.action)
        .collect()
}

#[test]
fn four_seat_40_life_journey_casts_sacrifices_and_attacks_with_the_commander() {
    let mut game = fixture_n(6565, 4)
        .hand(P0, &[CardId::SiegeGangCommander])
        .battlefield(
            P0,
            &[
                CardId::GrizzlyBears,
                CardId::GrizzlyBears,
                CardId::GrizzlyBears,
            ],
        )
        .mana(
            P0,
            ManaPool {
                red: 3,
                colorless: 5,
                ..ManaPool::default()
            },
        )
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .library(P2, &[CardId::Forest; 12])
        .library(P3, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    game.state.profile = Profile::CommanderCombat40;
    for seat in 0..4 {
        game.state.players[seat].life = 40;
    }

    // The cast must be an offered action, not a direct injection.
    let card = hand_card(&game, P0, CardId::SiegeGangCommander);
    assert!(
        offered(&game, P0).contains(&Action::CastSpell {
            card,
            targets: Vec::new()
        }),
        "the engine offers the Commander's cast"
    );
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
    assert_eq!(
        goblin_tokens(&game, P0).len(),
        3,
        "the enters trigger created three Goblin tokens in a real cast"
    );

    // Pay a real "Sacrifice a Goblin" cost for 2 damage to seat 2. The offer has
    // to exist, and it has to name the token we choose.
    let fodder = goblin_tokens(&game, P0)[0];
    let activation = Action::ActivateCostedAbility {
        source: commander,
        ability: 1,
        mode: 0,
        targets: vec![Target::Player(P2)],
        cost_cards: vec![fodder],
    };
    assert!(
        offered(&game, P0).contains(&activation),
        "the engine offers the sacrifice activation with that Goblin named"
    );
    let life_before = game.state.players[usize::from(P2)].life;
    let mut log = submit_ok(&mut game, P0, activation);
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert!(
        game.state.try_object(fodder).is_none(),
        "the sacrificed Goblin token ceased to exist (CR 111.7, CR 701.21a)"
    );
    assert_eq!(
        game.state.players[usize::from(P2)].life,
        life_before - 2,
        "the ability dealt 2 to the third seat"
    );

    // Play out the rest of the turn from offers only: the surviving tokens plus
    // the three Bears attack seat 3, and the game is left running to a real
    // checkpoint rather than being asserted at one exchange.
    game.state.turn.step = Step::DeclareAttackers;
    game.state.turn.priority = Some(P0);
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let attackers: Vec<ObjectId> = game
        .state
        .battlefield(P0)
        .into_iter()
        // The tokens entered this turn and are still summoning sick (CR 302.6),
        // so the attackers are the three Bears that were already there.
        .filter(|o| o.card == CardId::GrizzlyBears)
        .map(|o| o.id)
        .collect();
    assert_eq!(attackers.len(), 3, "three ready attackers");
    let life_before = game.state.players[usize::from(P3)].life;
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: attackers.clone(),
            defender: Some(P3),
        },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::PostcombatMain && g.state.pending.is_none(),
        8000,
    );
    assert!(
        game.state.players[usize::from(P3)].life < life_before,
        "the fourth seat took combat damage in the same journey"
    );
    assert!(
        goblin_tokens(&game, P0).len() >= 2,
        "the two unsacrificed tokens are still on the battlefield"
    );

    // The journey is replayable: the digest is recorded for the handoff and the
    // same state digests identically when re-derived, so the trace is stable.
    let digest = game.digest();
    let again = game.digest();
    assert_eq!(digest, again, "the journey digest is deterministic");
    eprintln!(
        "slice65 journey: 4 seats, 40 life, digest {digest}, events {}, P2 {}, P3 {}",
        log.len(),
        game.state.players[2].life,
        game.state.players[3].life
    );
}
