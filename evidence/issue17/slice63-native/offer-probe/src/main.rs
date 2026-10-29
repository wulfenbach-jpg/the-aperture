//! Native coauthor trace audit, not independent review or product policy.
use aperture_core::{Action, CardId, Event, Game, Replay};

fn main() {
    let path = std::env::args().nth(1).expect("replay path");
    let text = std::fs::read_to_string(path).unwrap();
    let replay: Replay = serde_json::from_str(&text).unwrap();
    let commanders: Vec<_> = replay.commanders.iter().map(|s| (s.seat, s.card)).collect();
    let (mut game, mut events) =
        Game::new_commander_combat_logged(replay.seed, &replay.seats, &commanders).unwrap();
    assert_eq!(game.state.first_player, replay.first_player);
    let mut discounted_dragons = Vec::new();
    for (index, step) in replay.steps.iter().enumerate() {
        assert_eq!(game.revision(), step.revision);
        assert_eq!(game.decision(), step.decision);
        let view = game.view_for(step.player, &events);
        assert!(
            view.legal.iter().any(|offer| offer.action == step.action),
            "step {index}: accepted action was not offered: {:?}",
            step.action
        );
        let is_dragon = match &step.action {
            Action::CastSpell { card, .. } => {
                game.state.object(*card).card == CardId::VolcanicDragon
            }
            _ => false,
        };
        let servants = game
            .state
            .battlefield(step.player)
            .iter()
            .filter(|o| o.card == CardId::DragonlordsServant)
            .count() as u32;
        let before = game.state.players[step.player as usize].mana.total();
        let batch = game
            .submit_action(step.player, step.action.clone())
            .unwrap();
        if is_dragon && servants > 0 {
            let tapped = batch
                .iter()
                .filter(|e| matches!(e, Event::ManaTapped { player, .. } if *player == step.player))
                .count() as u32;
            let after = game.state.players[step.player as usize].mana.total();
            let spent = before + tapped - after;
            let expected = 6 - servants.min(4);
            assert_eq!(spent, expected, "actual Dragon payment must be reduced");
            discounted_dragons.push(serde_json::json!({"step": index, "seat": step.player, "servants": servants, "mana_paid": spent, "printed_mana": 6}));
        }
        events.extend(batch);
    }
    assert_eq!(game.digest(), replay.final_digest);
    assert!(game.state.is_over());
    assert!(!discounted_dragons.is_empty());
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "accepted_actions_checked": replay.steps.len(),
            "all_actions_exactly_offered": true,
            "discounted_dragon_payments": discounted_dragons,
            "final_digest": game.digest(),
            "terminal": true,
            "self_check_not_independent_review": true
        }))
        .unwrap()
    );
}
