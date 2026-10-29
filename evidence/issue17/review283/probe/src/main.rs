//! Independent reviewer probe for slice 28 (Circle of the Moon Druid).
//!
//! Written by the reviewer from the printed text, NOT copied from the delta's
//! own tests. It reuses the repo's test fixture support (`crates/core/tests/
//! common/mod.rs`, read-only) and drives everything through the public action
//! API so no engine internal is trusted.

#[path = "../../../../../crates/core/tests/common/mod.rs"]
mod common;

use aperture_core::*;
use common::*;

fn druid() -> CardId {
    catalog::by_name("Circle of the Moon Druid").expect("druid in catalog")
}

fn green(generic: u32, pips: u32) -> ManaPool {
    ManaPool {
        green: pips,
        colorless: generic,
        ..ManaPool::default()
    }
}

/// The engine empties mana pools between steps, so a seat that needs mana on a
/// later turn is given it right before the cast (fixture-style direct write).
fn give_pool(g: &mut Game, player: PlayerId, pool: ManaPool) {
    g.state.players[player as usize].mana = pool;
}

/// Pass priority (only) until `player` holds it, so an instant can be cast in
/// another seat's main phase without ending the step.
fn give_priority(g: &mut Game, player: PlayerId) {
    for _ in 0..16 {
        if g.decision_owner() == Some(player) {
            return;
        }
        let Some(owner) = g.decision_owner() else {
            return;
        };
        let _ = g.submit_action(owner, Action::Pass);
    }
    panic!("priority never reached {player}");
}

fn drain(g: &mut Game) {
    for _ in 0..256 {
        if g.state.pending.is_none() && g.state.stack.is_empty() {
            return;
        }
        let Some(player) = g.decision_owner() else {
            return;
        };
        submit_ok(g, player, Action::Pass);
    }
    panic!("stack did not settle");
}

fn cast(g: &mut Game, player: PlayerId, card: CardId) -> ObjectId {
    let id = hand_card(g, player, card);
    let action = g
        .legal_actions()
        .into_iter()
        .find(|offered| matches!(&offered.action, Action::CastSpell { card, .. } if *card == id))
        .unwrap_or_else(|| panic!("{card:?} must be castable"))
        .action;
    submit_ok(g, player, action);
    drain(g);
    battlefield_card(g, player, card)
}

/// Cast a spell that needs one creature target and aim it at `target`.
fn cast_targeting(g: &mut Game, player: PlayerId, card: CardId, target: ObjectId) {
    let id = hand_card(g, player, card);
    let action = Action::CastSpell {
        card: id,
        targets: vec![Target::Permanent(target)],
    };
    assert!(
        g.legal_actions()
            .iter()
            .any(|offered| offered.action == action),
        "{card:?} with that target must be a legal action"
    );
    submit_ok(g, player, action);
    for _ in 0..64 {
        if g.state.pending.is_none() {
            break;
        }
        let owner = g.decision_owner().expect("a target is chosen");
        submit_ok(
            g,
            owner,
            Action::ChooseTriggerTarget {
                target: Target::Permanent(target),
            },
        );
    }
    drain(g);
}

fn stats(g: &Game, id: ObjectId) -> (i32, i32) {
    let o = g.state.object(id);
    (o.power(), o.toughness())
}

fn is_bear(g: &Game, id: ObjectId) -> bool {
    g.legal_targets(P0, TargetSpec::Bear, ObjectId(u32::MAX))
        .contains(&Target::Permanent(id))
}

fn is_human(g: &Game, id: ObjectId) -> bool {
    g.legal_targets(P0, TargetSpec::HumanYouControl, ObjectId(u32::MAX))
        .contains(&Target::Permanent(id))
}

fn advance_turn(g: &mut Game) -> PlayerId {
    let start = g.state.turn.number;
    for _ in 0..2000 {
        if g.state.turn.number != start {
            return g.state.turn.active;
        }
        assert!(!g.state.is_over(), "game ended while advancing");
        let owner = g.decision_owner().expect("decision owner");
        let action = auto_answer(g);
        let _ = g.submit_action(owner, action);
    }
    panic!("turn did not advance");
}

/// Advance the game (auto-answering) until `player` has priority in a main
/// phase with an empty stack, i.e. the moment sorcery-speed casting is legal.
fn run_to_main(g: &mut Game, player: PlayerId) {
    for _ in 0..4000 {
        if !g.state.is_over()
            && g.state.turn.active == player
            && matches!(g.state.turn.step, Step::PrecombatMain | Step::PostcombatMain)
            && g.state.stack.is_empty()
            && g.state.pending.is_none()
            && g.decision_owner() == Some(player)
        {
            return;
        }
        let owner = g.decision_owner().expect("decision owner while advancing");
        let action = auto_answer(g);
        let _ = g.submit_action(owner, action);
    }
    panic!("never reached {player}'s main phase");
}

fn game(seed: u64) -> Game {
    fixture(seed)
        .hand(P0, &[druid(), CardId::GiantGrowth, CardId::Shock])
        .mana(
            P0,
            ManaPool {
                green: 4,
                red: 2,
                colorless: 2,
                ..ManaPool::default()
            },
        )
        .library(P0, &[CardId::Forest; 14])
        .library(P1, &[CardId::Forest; 14])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build()
}

fn main() {
    let mut failures: Vec<String> = Vec::new();
    let mut check = |name: &str, cond: bool, detail: String| {
        println!("[{}] {name}: {detail}", if cond { "PASS" } else { "FAIL" });
        if !cond {
            failures.push(format!("{name}: {detail}"));
        }
    };

    // 1. Own turn: base 4/2, Bear, not Human.
    {
        let mut g = game(28301);
        let id = cast(&mut g, P0, druid());
        check(
            "own_turn_pt",
            stats(&g, id) == (4, 2),
            format!("power/toughness = {:?}", stats(&g, id)),
        );
        check(
            "own_turn_subtype",
            is_bear(&g, id) && !is_human(&g, id),
            format!("bear={} human={}", is_bear(&g, id), is_human(&g, id)),
        );
        check(
            "no_effect_created",
            g.state.effects.is_empty(),
            format!("effects={}", g.state.effects.len()),
        );
    }

    // 2. Opponent turn: printed 2/4 Human, not a Bear.
    {
        let mut g = game(28302);
        let id = cast(&mut g, P0, druid());
        assert_eq!(advance_turn(&mut g), P1);
        check(
            "opp_turn_pt",
            stats(&g, id) == (2, 4),
            format!("power/toughness = {:?}", stats(&g, id)),
        );
        check(
            "opp_turn_subtype",
            !is_bear(&g, id) && is_human(&g, id),
            format!("bear={} human={}", is_bear(&g, id), is_human(&g, id)),
        );
    }

    // 3. Two Druids owned by different seats: each answers its own controller.
    {
        let mut g = fixture(28303)
            .hand(P0, &[druid()])
            .hand(P1, &[druid()])
            .mana(P0, green(4, 2))
            .mana(P1, green(4, 2))
            .library(P0, &[CardId::Forest; 14])
            .library(P1, &[CardId::Forest; 14])
            .life(P0, 40)
            .life(P1, 40)
            .main(P0, Step::PrecombatMain)
            .build();
        let mine = cast(&mut g, P0, druid());
        assert_eq!(advance_turn(&mut g), P1);
        run_to_main(&mut g, P1);
        give_pool(&mut g, P1, green(4, 2));
        let theirs = cast(&mut g, P1, druid());
        check(
            "two_druids_per_seat",
            stats(&g, mine) == (2, 4) && stats(&g, theirs) == (4, 2),
            format!(
                "P0's {:?} (expect 2/4), P1's {:?} (expect 4/2)",
                stats(&g, mine),
                stats(&g, theirs)
            ),
        );
        check(
            "two_druids_subtype",
            !is_bear(&g, mine) && is_bear(&g, theirs),
            format!("P0 bear={} P1 bear={}", is_bear(&g, mine), is_bear(&g, theirs)),
        );
    }

    // 4. A pump on an opponent's turn adds to the printed 2/4, and the base
    //    reverts to 4/2 when the controller's own turn starts.
    {
        let mut g = game(28304);
        let id = cast(&mut g, P0, druid());
        assert_eq!(advance_turn(&mut g), P1);
        run_to_main(&mut g, P1);
        give_pool(
            &mut g,
            P0,
            ManaPool {
                green: 1,
                ..ManaPool::default()
            },
        );
        give_priority(&mut g, P0);
        cast_targeting(&mut g, P0, CardId::GiantGrowth, id);
        check(
            "pump_on_opp_turn",
            stats(&g, id) == (5, 7),
            format!("2/4 + 3/3 = {:?}", stats(&g, id)),
        );
        assert_eq!(advance_turn(&mut g), P0);
        check(
            "base_back_on_own_turn",
            stats(&g, id) == (4, 2),
            format!("after the pump expired: {:?}", stats(&g, id)),
        );
    }

    // 5. Lethal damage is judged against the *effective* toughness: 2 damage
    //    kills the 4/2 form but not the 2/4 form.
    {
        let mut g = game(28305);
        let id = cast(&mut g, P0, druid());
        cast_targeting(&mut g, P0, CardId::Shock, id);
        check(
            "lethal_on_own_turn",
            g.state
                .try_object(id)
                .map(|o| o.zone == Zone::Graveyard)
                .unwrap_or(false),
            format!(
                "zone after Shock on the 4/2 form = {:?}",
                g.state.try_object(id).map(|o| o.zone)
            ),
        );
    }
    {
        let mut g = game(28306);
        let id = cast(&mut g, P0, druid());
        assert_eq!(advance_turn(&mut g), P1);
        run_to_main(&mut g, P1);
        give_pool(
            &mut g,
            P0,
            ManaPool {
                red: 1,
                ..ManaPool::default()
            },
        );
        give_priority(&mut g, P0);
        cast_targeting(&mut g, P0, CardId::Shock, id);
        let survives = g
            .state
            .try_object(id)
            .map(|o| o.zone == Zone::Battlefield && o.damage == 2)
            .unwrap_or(false);
        check(
            "survives_on_opp_turn",
            survives,
            format!(
                "zone={:?} damage={:?}",
                g.state.try_object(id).map(|o| o.zone),
                g.state.try_object(id).map(|o| o.damage)
            ),
        );
    }

    // 6. The Druid returns from the graveyard/hand with the printed base again
    //    (no stale stored delta survives a zone change).
    {
        let mut g = fixture(28307)
            .hand(P0, &[druid(), CardId::Unsummon])
            .mana(
                P0,
                ManaPool {
                    green: 6,
                    blue: 2,
                    colorless: 2,
                    ..ManaPool::default()
                },
            )
            .library(P0, &[CardId::Forest; 14])
            .library(P1, &[CardId::Forest; 14])
            .life(P0, 40)
            .life(P1, 40)
            .main(P0, Step::PrecombatMain)
            .build();
        let id = cast(&mut g, P0, druid());
        assert_eq!(stats(&g, id), (4, 2));
        cast_targeting(&mut g, P0, CardId::Unsummon, id);
        let in_hand = g
            .state
            .try_object(id)
            .map(|o| o.zone == Zone::Hand)
            .unwrap_or(false);
        let again = cast(&mut g, P0, druid());
        check(
            "bounce_and_replay",
            in_hand && stats(&g, again) == (4, 2),
            format!(
                "bounced={in_hand} recast power/toughness={:?}",
                stats(&g, again)
            ),
        );
    }

    // 7. A save taken mid-turn round-trips, keeps the live 4/2, and the base
    //    still reverts on the next turn after loading.
    {
        let mut g = game(28308);
        let id = cast(&mut g, P0, druid());
        // Serialize exactly the way a save does, then rebuild the game.
        let text = serde_json::to_string(&g.state).expect("state serializes");
        let restored: GameState = serde_json::from_str(&text).expect("state deserializes");
        let mut loaded_game = Game::from_state(restored).expect("state reloads");
        check(
            "save_keeps_bear_form",
            stats(&loaded_game, id) == (4, 2),
            format!("loaded power/toughness = {:?}", stats(&loaded_game, id)),
        );
        assert_eq!(advance_turn(&mut loaded_game), P1);
        check(
            "save_reverts_next_turn",
            stats(&loaded_game, id) == (2, 4),
            format!("after loading and ending the turn: {:?}", stats(&loaded_game, id)),
        );
    }

    if failures.is_empty() {
        println!("\nALL PROBES PASSED");
    } else {
        println!("\nFAILURES:\n{}", failures.join("\n"));
        std::process::exit(1);
    }
}
