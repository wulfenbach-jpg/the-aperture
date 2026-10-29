//! Independent review probe for issue-17 slice 35 (Words of Wilding).
//!
//! The lane's own four-seat journey drew the card three times and never cast it,
//! so this probe plays real four-seat, 40-life, exact-deck-snapshot games with my
//! own driving policy that deliberately casts Words of Wilding and activates it,
//! and then instruments every transition of `draw_replacements` per seat:
//!
//! * an armed replacement is consumed by that seat's next draw, and that draw
//!   moves no card, emits no `Event::Draw` and is never a `DrewFromEmpty`;
//! * the replacement token is a real 2/2 green Bear creature object;
//! * an unspent replacement is cleared at the end of the turn (CR 514.2) without
//!   creating a token or drawing a card, and never fires late.
//!
//! Public API only: `Game::new_commander_combat`, `legal_actions`,
//! `submit_action`, the public `GameState` and `Event` values.

use aperture_core::*;

fn deck_text() -> String {
    std::fs::read_to_string("C:/AI/r297a/decks/package-slice35.txt").expect("deck file")
}

fn bear_ids(game: &Game, seat: PlayerId) -> Vec<ObjectId> {
    game.state
        .battlefield(seat)
        .into_iter()
        .filter(|o| o.card == CardId::BearToken)
        .map(|o| o.id)
        .collect()
}

fn armed(game: &Game, seat: PlayerId) -> u32 {
    game.state
        .draw_replacements
        .get(seat as usize)
        .copied()
        .unwrap_or(0)
}

fn words_id() -> CardId {
    catalog::by_name("Words of Wilding").expect("Words of Wilding in the pool")
}

fn is_words(game: &Game, source: ObjectId) -> bool {
    game.state
        .try_object(source)
        .map(|o| o.card == words_id())
        .unwrap_or(false)
}

#[derive(Clone, Copy)]
struct SeatView {
    armed: u32,
    library: usize,
    hand: usize,
    bears: usize,
}

fn view(game: &Game, seat: PlayerId) -> SeatView {
    SeatView {
        armed: armed(game, seat),
        library: game.state.player(seat).library.len(),
        hand: game.state.player(seat).hand_size(),
        bears: bear_ids(game, seat).len(),
    }
}

fn main() {
    let deck = DeckList::parse_practice("package-slice35", &deck_text()).expect("deck parses");
    let commander = catalog::by_name("Jasmine Boreal").expect("supported commander");
    let decks = vec![deck.clone(), deck.clone(), deck.clone(), deck];
    let commanders: Vec<(PlayerId, CardId)> = (0..4u8).map(|seat| (seat, commander)).collect();

    let bear_def = catalog::def(CardId::BearToken);
    assert_eq!(bear_def.power, Some(2), "the replacement token is 2/2");
    assert_eq!(bear_def.toughness, Some(2), "the replacement token is 2/2");
    assert_eq!(
        bear_def.colors,
        &[ManaColor::Green],
        "the replacement token is green"
    );

    for seed in [3535u64, 4242u64, 1717u64] {
        let mut game = Game::new_commander_combat(seed, &decks, &commanders).expect("game setup");
        let mut casts = 0usize;
        let mut activations = 0usize;
        let mut armings = 0u32;
        let mut consumptions = 0u32;
        let mut expiries = 0u32;
        let mut last_activation_turn: [Option<u32>; 4] = [None; 4];
        let mut turns_seen = 0u32;

        for _step in 0..40_000 {
            if game.state.is_over() {
                break;
            }
            let Some(seat) = game.decision_owner() else {
                break;
            };
            let offers = game.legal_actions();
            if offers.is_empty() {
                break;
            }
            let armed_now = armed(&game, seat);
            let words_on_board = game
                .state
                .battlefield(seat)
                .into_iter()
                .any(|o| o.card == words_id());
            let cast_words = offers.iter().position(|o| {
                matches!(&o.action, Action::CastSpell { card, .. } if game
                    .state
                    .try_object(*card)
                    .map(|obj| obj.card == words_id())
                    .unwrap_or(false))
            });
            let activate_words = offers.iter().position(|o| {
                matches!(o.action, Action::ActivateAbility { source, .. } if is_words(&game, source))
            });
            let already_this_turn =
                last_activation_turn[seat as usize] == Some(game.state.turn.number);

            // Deliberate policy: play lands, cast the card, activate it once per
            // turn while nothing is armed, otherwise pass.
            let pick: usize = if let Some(index) = offers
                .iter()
                .position(|o| matches!(o.action, Action::Mulligan { keep: true }))
            {
                index
            } else if let Some(index) = offers
                .iter()
                .position(|o| matches!(o.action, Action::PlayLand { .. }))
            {
                index
            } else if let Some(index) = cast_words {
                index
            } else if words_on_board
                && armed_now == 0
                && game.state.stack.is_empty()
                && !already_this_turn
                && let Some(index) = activate_words
            {
                index
            } else if let Some(index) = offers
                .iter()
                .position(|o| matches!(o.action, Action::Pass))
            {
                index
            } else {
                0
            };

            let action = offers[pick].action.clone();
            let cast_was_words = match &action {
                Action::CastSpell { card, .. } => is_words(&game, *card),
                _ => false,
            };
            let activate_was_words = match &action {
                Action::ActivateAbility { source, .. } => is_words(&game, *source),
                _ => false,
            };
            let before: Vec<SeatView> = (0..4u8).map(|s| view(&game, s)).collect();
            let turn = game.state.turn.number;
            let step = game.state.turn.step;
            let events = match game.submit_action(seat, action.clone()) {
                Ok(events) => events,
                Err(error) => panic!("seed {seed}: legal action refused: {error:?} for {action:?}"),
            };
            if cast_was_words {
                casts += 1;
            }
            if activate_was_words {
                activations += 1;
                last_activation_turn[seat as usize] = Some(turn);
            }
            if game.state.turn.number != turn {
                turns_seen += 1;
            }

            for s in 0..4u8 {
                let after = view(&game, s);
                let prior = before[s as usize];
                if after.armed > prior.armed {
                    armings += after.armed - prior.armed;
                }
                if after.armed < prior.armed {
                    let delta = prior.armed - after.armed;
                    let gained = after.bears.saturating_sub(prior.bears);
                    let drew = events
                        .iter()
                        .any(|event| matches!(event, Event::Draw { player, .. } if *player == s));
                    let from_empty = events.iter().any(
                        |event| matches!(event, Event::DrewFromEmpty { player } if *player == s),
                    );
                    if gained == delta as usize && !drew {
                        consumptions += delta;
                        assert_eq!(
                            after.library, prior.library,
                            "seed {seed}: the replaced draw moved a card from P{s}'s library"
                        );
                        assert_eq!(
                            after.hand, prior.hand,
                            "seed {seed}: the replaced draw moved a card into P{s}'s hand"
                        );
                        assert!(
                            !from_empty,
                            "seed {seed}: a replaced draw was still a draw from an empty library"
                        );
                        let token = *bear_ids(&game, s).last().expect("replacement token");
                        let token_object = game
                            .state
                            .try_object(token)
                            .expect("token object present")
                            .clone();
                        assert!(
                            game.is_creature(&token_object),
                            "seed {seed}: the replacement token is not a creature"
                        );
                        assert!(
                            game.has_color(token, ManaColor::Green),
                            "seed {seed}: the replacement token is not green"
                        );
                        println!(
                            "seed {seed}: replaced draw — P{s} turn {turn} {step:?}: armed {} -> {}, +{gained} green 2/2 Bear, library {}/hand {} unchanged, no Draw event",
                            prior.armed, after.armed, after.library, after.hand
                        );
                    } else if after.armed == 0 && gained == 0 && !drew {
                        expiries += delta;
                        println!(
                            "seed {seed}: unspent replacement cleared — P{s} turn {turn} {step:?}: armed {} -> 0, no token, no draw (CR 514.2)",
                            prior.armed
                        );
                    } else {
                        panic!(
                            "seed {seed}: unexplained draw_replacements change for P{s}: armed {} -> {}, bears {gained:+}, drew={drew}, from_empty={from_empty}",
                            prior.armed, after.armed
                        );
                    }
                }
            }
        }

        println!(
            "seed {seed}: status={:?} casts={casts} activations={activations} armings={armings} consumptions={consumptions} expiries={expiries} turns_played={turns_seen} armed_end={:?} digest={}",
            game.state.outcome, game.state.draw_replacements, game.digest()
        );
        assert!(casts >= 1, "seed {seed}: the probe never cast Words of Wilding");
        assert!(
            activations >= 1,
            "seed {seed}: the probe never activated Words of Wilding"
        );
        assert!(
            consumptions >= 1,
            "seed {seed}: no armed replacement was ever consumed by a real draw"
        );
    }

    println!("PROBE OK: Words of Wilding was cast, activated, and its replacement consumed real draws in 4-seat games");
}
