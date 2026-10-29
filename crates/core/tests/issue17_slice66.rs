//! Slice 66 behaviour: review-365's functional defect (R365-1) and the
//! sacrifice-a-Goblin family it unblocks.
//!
//! R365-1 (CR 603.3 + 601.2h/602.2b): a triggered ability that triggers while
//! an activation cost is being paid is put on the stack **above** the ability
//! that paid for it, so it resolves first. Before this slice the dies trigger
//! of the sacrificed permanent was queued but never staged, so it resolved
//! below — after the ability — with no response window.
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

/// Index of the first `TriggerStaged` event for `card`.
fn staged_at(events: &[Event], card: CardId) -> Option<usize> {
    events.iter().position(|event| {
        matches!(
            event,
            Event::TriggerStaged { card: staged, .. } if *staged == card
        )
    })
}

/// Index of the first `LifeChange` that changed `player`'s life by `delta`.
fn life_change_at(events: &[Event], player: PlayerId, delta: i32) -> Option<usize> {
    events.iter().position(|event| {
        matches!(
            event,
            Event::LifeChange { player: seat, delta: d, .. } if *seat == player && *d == delta
        )
    })
}

/// R365-1, the reviewer's own probe shape: Siege-Gang Commander plus a tapped
/// Goblin Sharpshooter. Sacrificing a Goblin to pay the cost kills a creature,
/// which triggers the Sharpshooter's untap. That trigger must be on the stack
/// ABOVE the damage ability and must resolve BEFORE the damage.
#[test]
fn sacrifice_cost_dies_trigger_is_staged_above_the_ability_and_resolves_first() {
    let mut game = fixture(6601)
        .battlefield(
            P0,
            &[
                CardId::SiegeGangCommander,
                CardId::GoblinSharpshooter,
                CardId::GoblinToken,
            ],
        )
        .mana(P0, red(1, 1))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let commander = battlefield_card(&game, P0, CardId::SiegeGangCommander);
    let shooter = battlefield_card(&game, P0, CardId::GoblinSharpshooter);
    let fodder = battlefield_card(&game, P0, CardId::GoblinToken);
    object_mut(&mut game.state, shooter).tapped = true;

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

    // The stack is the ground truth for ordering: the trigger must sit above
    // the activated ability, not merely be queued.
    assert_eq!(
        game.state.stack.len(),
        2,
        "the ability and the dies trigger are both on the stack: {:?}",
        game.state
            .stack
            .iter()
            .map(|item| item.entry)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        game.state.stack[0].entry,
        StackEntry::Activated { ability: 1 },
        "the activated ability is below"
    );
    assert_eq!(
        game.state.stack[1].entry,
        StackEntry::Triggered { ability: 0 },
        "the Sharpshooter's dies trigger is above the ability it paid for (CR 602.2b)"
    );
    assert!(
        staged_at(&log, CardId::GoblinSharpshooter).is_some(),
        "the trigger really was staged during activation, not left queued"
    );

    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );

    let staged = staged_at(&log, CardId::GoblinSharpshooter).expect("trigger staged");
    let damage = life_change_at(&log, OPPONENT, -2).expect("the ability dealt its 2 damage");
    assert!(
        staged < damage,
        "resolution order: the dies trigger ({staged}) must resolve before the damage ({damage})"
    );
    let untap_resolution = log
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::AbilityResolved { source, .. } if *source == shooter
            )
        })
        .expect("the untap trigger resolved");
    assert!(
        untap_resolution < damage,
        "the Sharpshooter untaps before the damage resolves, so it could be used in response"
    );
    assert!(
        !game.state.object(shooter).tapped,
        "the Sharpshooter untapped: its trigger resolved"
    );
    assert_eq!(
        game.state.players[usize::from(OPPONENT)].life,
        life_before - 2,
        "the ability still resolves its 2 damage afterwards"
    );
}

/// R365-1, second shape: Festering Goblin's "when this dies, target creature
/// gets -1/-1" asks for its target BEFORE the ability it paid for resolves.
#[test]
fn sacrifice_cost_target_choice_trigger_resolves_before_the_ability() {
    let mut game = fixture(6602)
        .battlefield(
            P0,
            &[
                CardId::SiegeGangCommander,
                CardId::FesteringGoblin,
                CardId::GrizzlyBears,
            ],
        )
        .mana(P0, red(1, 1))
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let commander = battlefield_card(&game, P0, CardId::SiegeGangCommander);
    let festering = battlefield_card(&game, P0, CardId::FesteringGoblin);
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let life_before = game.state.players[usize::from(OPPONENT)].life;

    let mut log = submit_ok(
        &mut game,
        P0,
        Action::ActivateCostedAbility {
            source: commander,
            ability: 1,
            mode: 0,
            targets: vec![Target::Player(OPPONENT)],
            cost_cards: vec![festering],
        },
    );
    // The -1/-1 trigger needs a target, so staging it leaves a pending choice.
    assert!(
        matches!(game.state.pending, Some(Pending::TriggerTarget { .. })),
        "the dies trigger's target is asked for while the ability is still below it: {:?}",
        game.state.pending
    );
    assert_eq!(
        game.state.players[usize::from(OPPONENT)].life,
        life_before,
        "the 2 damage has NOT resolved yet: the trigger is above the ability"
    );
    assert_eq!(
        game.state.stack.len(),
        1,
        "the activated ability is still waiting below the trigger"
    );
    let chosen = game
        .legal_actions()
        .into_iter()
        .find_map(|entry| match entry.action {
            Action::ChooseTriggerTarget { target } if target == Target::Permanent(bears) => {
                Some(target)
            }
            _ => None,
        })
        .expect("the trigger offers the Bears as a target");
    let _ = submit_ok(
        &mut game,
        P0,
        Action::ChooseTriggerTarget { target: chosen },
    );
    run_until(
        &mut game,
        &mut log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        8000,
    );
    assert!(
        game.state.object(bears).toughness() <= 1,
        "the -1/-1 really resolved onto the chosen creature"
    );
    assert_eq!(
        game.state.players[usize::from(OPPONENT)].life,
        life_before - 2,
        "and the ability resolved afterwards"
    );
}
