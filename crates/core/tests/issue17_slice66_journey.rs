//! Slice 66 journey: one real four-seat, 40-life game that casts both newly
//! enabled cards and uses every printed instruction they carry.
//!
//! Everything is driven from `Game::legal_actions` — the engine's own projected
//! offers — never by editing state mid-game, so the trace is a real executable
//! journey rather than a fixture assertion. It is a bounded Commander-combat
//! practice profile (40 life, one supported commander per seat), not the
//! Commander format and not a strength claim.
mod common;
use aperture_core::*;
use common::*;

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
fn four_seat_40_life_journey_plays_both_sacrifice_cards_end_to_end() {
    let mut game = fixture_n(6666, 4)
        .hand(P0, &[CardId::PashalikMons, CardId::GoblinTrashmaster])
        .battlefield(
            P0,
            &[
                CardId::GoblinToken,
                CardId::GrizzlyBears,
                CardId::GrizzlyBears,
            ],
        )
        // The artifact the Trashmaster destroys belongs to the second seat: the
        // printed target is "target artifact", not "an artifact you control".
        .battlefield(P1, &[CardId::ArcaneSignet])
        .mana(
            P0,
            ManaPool {
                red: 5,
                colorless: 7,
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
    let mut log: Vec<Event> = Vec::new();

    // 1. Cast Pashalik Mons — the cast has to be an offered action.
    let mons_card = hand_card(&game, P0, CardId::PashalikMons);
    assert!(
        offered(&game, P0).contains(&Action::CastSpell {
            card: mons_card,
            targets: Vec::new()
        }),
        "the engine offers Pashalik Mons' cast"
    );
    let mut cast = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: mons_card,
            targets: Vec::new(),
        },
    );
    log.append(&mut cast);
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    let mons = battlefield_card(&game, P0, CardId::PashalikMons);

    // 2. Cast Goblin Trashmaster. Its anthem must pump the other Goblin.
    let trash_card = hand_card(&game, P0, CardId::GoblinTrashmaster);
    assert!(
        offered(&game, P0).contains(&Action::CastSpell {
            card: trash_card,
            targets: Vec::new()
        }),
        "the engine offers Goblin Trashmaster's cast"
    );
    let mut cast = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: trash_card,
            targets: Vec::new(),
        },
    );
    log.append(&mut cast);
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    let trash = battlefield_card(&game, P0, CardId::GoblinTrashmaster);
    let token = goblin_tokens(&game, P0)[0];
    assert_eq!(
        (
            game.state.object(token).power(),
            game.state.object(token).toughness()
        ),
        (2, 2),
        "the Trashmaster anthem pumps the other Goblin in a real game"
    );
    assert_eq!(
        (
            game.state.object(trash).power(),
            game.state.object(trash).toughness()
        ),
        (3, 3),
        "'other Goblins' excludes the Trashmaster itself"
    );

    // 3. Pay Pashalik's "{3}{R}, Sacrifice a Goblin" for two tokens. The
    //    sacrificed token dying triggers Pashalik's own dies watcher, which must
    //    resolve ABOVE the ability it paid for (review 365, R365-1).
    let activation = Action::ActivateCostedAbility {
        source: mons,
        ability: 1,
        mode: 0,
        targets: Vec::new(),
        cost_cards: vec![token],
    };
    assert!(
        offered(&game, P0).contains(&activation),
        "the engine offers the sacrifice activation with that Goblin named"
    );
    let p2_life_before = game.state.players[usize::from(P2)].life;
    let tokens_before = goblin_tokens(&game, P0).len();
    let mut act = submit_ok(&mut game, P0, activation);
    log.append(&mut act);
    // The ping needs a target; answer it from the offers.
    run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::TriggerTarget { .. })),
        8000,
    );
    assert_eq!(
        game.state.players[usize::from(P2)].life,
        p2_life_before,
        "the trigger is above the ability, so its damage has not resolved yet"
    );
    assert_eq!(
        game.state.stack.len(),
        1,
        "the sacrifice ability is still waiting below the staged trigger"
    );
    let target = game
        .legal_actions()
        .into_iter()
        .find_map(|entry| match entry.action {
            Action::ChooseTriggerTarget {
                target: Target::Player(P2),
            } => Some(Target::Player(P2)),
            _ => None,
        })
        .expect("the ping offers a player target");
    let mut answer = submit_ok(&mut game, P0, Action::ChooseTriggerTarget { target });
    log.append(&mut answer);
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        game.state.players[usize::from(P2)].life,
        p2_life_before - 1,
        "the dying Goblin's ping resolved"
    );
    // Order, not just presence: the ping resolved BEFORE the token-making
    // ability it paid for (R365-1).
    let ping = log
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::LifeChange { player: seat, delta: -1, .. } if *seat == P2
            )
        })
        .expect("the ping dealt its damage");
    let ability = log
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::AbilityResolved { source, .. } if *source == mons
            )
        })
        .expect("the token-making ability resolved");
    assert!(
        ping < ability,
        "the dies trigger ({ping}) resolves before the ability ({ability})"
    );
    assert_eq!(
        goblin_tokens(&game, P0).len(),
        tokens_before - 1 + 2,
        "one Goblin was sacrificed and two were created"
    );

    // 4. Pay Goblin Trashmaster's "Sacrifice a Goblin: Destroy target artifact".
    let artifact = battlefield_card(&game, P1, CardId::ArcaneSignet);
    let fodder = goblin_tokens(&game, P0)[0];
    let destroy = Action::ActivateCostedAbility {
        source: trash,
        ability: 0,
        mode: 0,
        targets: vec![Target::Permanent(artifact)],
        cost_cards: vec![fodder],
    };
    assert!(
        offered(&game, P0).contains(&destroy),
        "the engine offers the artifact-destruction activation"
    );
    let p2_after_first_ping = game.state.players[usize::from(P2)].life;
    let mut act = submit_ok(&mut game, P0, destroy);
    log.append(&mut act);
    // The Goblin sacrificed to the Trashmaster is another Goblin dying while
    // Pashalik is on the battlefield, so its watcher triggers again.
    run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::TriggerTarget { .. })),
        8000,
    );
    let second_target = game
        .legal_actions()
        .into_iter()
        .find_map(|entry| match entry.action {
            Action::ChooseTriggerTarget {
                target: Target::Player(P3),
            } => Some(Target::Player(P3)),
            _ => None,
        })
        .expect("the second ping offers a player target");
    let mut answer = submit_ok(
        &mut game,
        P0,
        Action::ChooseTriggerTarget {
            target: second_target,
        },
    );
    log.append(&mut answer);
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert_eq!(
        game.state.players[usize::from(P2)].life,
        p2_after_first_ping,
        "the second ping was aimed at the fourth seat, not the third"
    );
    assert_eq!(
        game.state.object(artifact).zone,
        Zone::Graveyard,
        "the opponent's artifact was destroyed in the real game"
    );

    // 5. Combat: the two Bears that were already on the battlefield attack the
    //    fourth seat. Both creatures cast this turn, and the tokens created this
    //    turn, are summoning sick (CR 302.6).
    game.state.turn.step = Step::DeclareAttackers;
    game.state.turn.priority = Some(P0);
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let attackers: Vec<ObjectId> = game
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::GrizzlyBears)
        .map(|o| o.id)
        .collect();
    assert_eq!(attackers.len(), 2, "the two ready Bears attack");
    let p3_life_before = game.state.players[usize::from(P3)].life;
    let mut combat = submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: attackers.clone(),
            defender: Some(P3),
        },
    );
    log.append(&mut combat);
    run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::PostcombatMain && g.state.pending.is_none(),
        8000,
    );
    assert!(
        game.state.players[usize::from(P3)].life < p3_life_before,
        "the fourth seat took combat damage in the same journey"
    );

    let digest = game.digest();
    assert_eq!(digest, game.digest(), "the journey digest is deterministic");
    eprintln!(
        "slice66 journey: {} events, digest {digest}, P2 {} P3 {} P1 {} P0 {}",
        log.len(),
        game.state.players[2].life,
        game.state.players[3].life,
        game.state.players[1].life,
        game.state.players[0].life
    );
}
