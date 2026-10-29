//! Evidence-only demonstration driver for slice 35, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access.
//!
//! Its purpose is the delivery-bar evidence review 297 requested: a *real*
//! four-seat / 40-life game from `decks/package-slice35.txt` in which Words of
//! Wilding (ONS 305) is actually **cast and activated**, and in which the armed
//! draw replacement is **consumed by a real draw step** — asserted from the
//! engine's own events, not from a test-only shortcut:
//!
//! * the draw step of the armed seat produces **no `Event::Draw`** for that
//!   seat and **no `Event::DrewFromEmpty`**, so nothing was drawn (CR 614.1a
//!   replaced the event, CR 614.6 the event never happens);
//! * that seat's library and hand sizes are **unchanged** across the action;
//! * exactly **one** `Event::PermanentEntered` for `CardId::BearToken` under
//!   that seat's control appears, and the token really is a 2/2 green Bear
//!   creature (so it is a token with the printed characteristics, not a card);
//! * the armed count drops by exactly one, and the seat neither loses nor is
//!   marked `drew_from_empty`.
//!
//! It also exercises the unspent half of the printed text in a real game: from
//! turn 6 each seat may arm two replacements in one upkeep, so one is consumed
//! by the draw step and the survivor is cleared at that turn's cleanup
//! (CR 514.2). The cleanup assertion is the mirror image: zero armed counts
//! afterwards and no token.
//!
//! The strategy prefers, in this order: arm a replacement during its own
//! upkeep (instant-speed activation), play a land, cast Words of Wilding, cast
//! a creature, and it always attacks with everything legal. Everything else is
//! the frozen heuristic, so the line is deterministic but is not a product
//! decider. It is a 40-card package, not a starter certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, ManaColor, ObjectId,
    PlayerId, Replay, ReplayStep, SessionSave, Zone,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice35.txt";
const SEED: u64 = 3535;
/// From this turn number on, a seat may arm two replacements in one upkeep so
/// that the spare is cleared at cleanup (CR 514.2) in a real game.
const SECOND_ARM_FROM_TURN: u32 = 6;

fn bear_tokens(g: &Game, seat: PlayerId) -> Vec<ObjectId> {
    g.state
        .objects
        .values()
        .filter(|object| {
            object.zone == Zone::Battlefield
                && object.controller == seat
                && object.card == CardId::BearToken
        })
        .map(|object| object.id)
        .collect()
}

fn words_on_battlefield(g: &Game, seat: PlayerId) -> Option<ObjectId> {
    g.state
        .battlefield(seat)
        .into_iter()
        .find(|object| object.card == CardId::WordsOfWilding)
        .map(|object| object.id)
}

fn armed(g: &Game, seat: PlayerId) -> u32 {
    g.state
        .draw_replacements
        .get(seat as usize)
        .copied()
        .unwrap_or(0)
}

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice35", &text).unwrap();
    assert_eq!(deck.size(), 40, "the package is exactly forty cards");
    let seats = vec![deck.clone(); 4];
    let commanders: Vec<_> = (0..4).map(|p| (p, CardId::JasmineBoreal)).collect();
    let (mut game, mut events) =
        Game::new_commander_combat_logged(SEED, &seats, &commanders).unwrap();
    let setup = commanders
        .iter()
        .map(|&(seat, card)| CommanderSetup { seat, card })
        .collect();
    let mut replay = Replay::new_commander_combat(
        SEED,
        game.state.first_player,
        seats.clone(),
        setup,
        AiSettings::default(),
    )
    .unwrap();
    let mut baseline = HeuristicPolicy::new();
    let mut steps = 0usize;
    let mut replaced_draws = 0usize;
    let mut replaced_by_seat = [0usize; 4];
    let mut unspent_cleared = 0usize;
    let mut unspent_cleared_by_seat = [0usize; 4];
    let mut tokens_from_replacements = 0usize;
    let mut armed_total = 0usize;
    while !game.state.is_over() && steps < 40000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);
        let demo_action = match view.pending_kind {
            Some(PendingKind::DeclareAttackers) => view
                .legal
                .iter()
                .find(|legal| match &legal.action {
                    Action::DeclareAttackers { attackers, .. } => !attackers.is_empty(),
                    _ => false,
                })
                .map(|legal| legal.action.clone()),
            None if view.stack.is_empty() && view.pending_kind.is_none() => {
                let want_arms = if game.state.turn.number >= SECOND_ARM_FROM_TURN {
                    2
                } else {
                    1
                };
                let can_arm = view.step == Step::Upkeep
                    && view.active == player
                    && game.state.turn.priority == Some(player)
                    && words_on_battlefield(&game, player).is_some()
                    && armed(&game, player) < want_arms;
                if can_arm {
                    view.legal
                        .iter()
                        .find(|legal| match &legal.action {
                            Action::ActivateAbility { source, .. } => game
                                .state
                                .try_object(*source)
                                .is_some_and(|object| {
                                    object.card == CardId::WordsOfWilding
                                        && object.zone == Zone::Battlefield
                                }),
                            _ => false,
                        })
                        .map(|legal| legal.action.clone())
                } else if view.active == player
                    && matches!(view.step, Step::PrecombatMain | Step::PostcombatMain)
                {
                    // Play a land, then the card under test, then creatures.
                    view.legal
                        .iter()
                        .find(|legal| matches!(legal.action, Action::PlayLand { .. }))
                        .map(|legal| legal.action.clone())
                        .or_else(|| {
                            view.legal
                                .iter()
                                .find(|legal| match &legal.action {
                                    Action::CastSpell { card, .. } => view
                                        .own_hand
                                        .iter()
                                        .any(|c| c.id == *card && c.card == CardId::WordsOfWilding),
                                    _ => false,
                                })
                                .map(|legal| legal.action.clone())
                        })
                        .or_else(|| {
                            view.legal
                                .iter()
                                .find(|legal| match &legal.action {
                                    Action::CastSpell { card, .. } => view
                                        .own_hand
                                        .iter()
                                        .any(|c| c.id == *card && c.card.def().is_creature()),
                                    _ => false,
                                })
                                .map(|legal| legal.action.clone())
                        })
                } else {
                    None
                }
            }
            _ => None,
        };
        let action = demo_action.unwrap_or_else(|| baseline.decide(&view).action);

        // Snapshot the drawing seat's zones before the action, so the draw step
        // assertions below compare against the state the engine actually saw.
        let active = game.state.turn.active;
        let pre_library = game.state.players[active as usize].library.len();
        let pre_hand = game.state.players[active as usize].hand.len();
        let pre_bears = bear_tokens(&game, active).len();
        let pre_armed = armed(&game, active);
        let pre_armed_by_seat: [u32; 4] =
            [0, 1, 2, 3].map(|seat| armed(&game, seat as PlayerId));
        let pre_armed_total: u32 = pre_armed_by_seat.iter().sum();

        let revision = game.revision();
        let decision = game.decision();
        let accepted = game
            .submit_action(player, action.clone())
            .expect("all actions must be accepted; no rejection fallback");
        steps += 1;

        // --- the draw step: the replacement must swallow the draw ---
        if accepted
            .iter()
            .any(|event| matches!(event, Event::StepBegin { step: Step::Draw }))
        {
            if pre_armed > 0 {
                assert!(
                    !accepted
                        .iter()
                        .any(|event| matches!(event, Event::Draw { player: p, .. } if *p == active)),
                    "CR 614.1a/614.6: a replaced draw is never performed"
                );
                assert!(
                    !accepted.iter().any(
                        |event| matches!(event, Event::DrewFromEmpty { player: p } if *p == active)
                    ),
                    "CR 704.5b: a replaced draw attempted nothing, so no empty-library draw"
                );
                assert_eq!(
                    game.state.players[active as usize].library.len(),
                    pre_library,
                    "no card may leave the library for a replaced draw"
                );
                assert_eq!(
                    game.state.players[active as usize].hand.len(),
                    pre_hand,
                    "no card may enter the hand for a replaced draw"
                );
                let entered: Vec<_> = accepted
                    .iter()
                    .filter_map(|event| match event {
                        Event::PermanentEntered {
                            object,
                            card,
                            controller,
                        } if *card == CardId::BearToken && *controller == active => Some(*object),
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    entered.len(),
                    1,
                    "exactly one Bear token replaces the draw, got {entered:?}"
                );
                let post_bears = bear_tokens(&game, active);
                assert_eq!(
                    post_bears.len(),
                    pre_bears + 1,
                    "the replacement adds exactly one Bear token"
                );
                let token = game
                    .state
                    .try_object(entered[0])
                    .expect("the new token exists on the battlefield");
                assert!(game.is_creature(token), "the token is a creature");
                assert_eq!((token.power(), token.toughness()), (2, 2), "a 2/2");
                assert!(
                    game.has_color(token.id, ManaColor::Green),
                    "the token is green"
                );
                assert!(
                    game.state.is_live(active),
                    "a replaced draw cannot eliminate the seat"
                );
                assert!(!game.state.players[active as usize].drew_from_empty);
                assert_eq!(
                    armed(&game, active),
                    pre_armed - 1,
                    "exactly one armed replacement is consumed"
                );
                replaced_draws += 1;
                replaced_by_seat[active as usize] += 1;
                tokens_from_replacements += 1;
            }
        }

        // --- cleanup: an unspent replacement ends with the turn (CR 514.2) ---
        if pre_armed_total > 0
            && accepted
                .iter()
                .any(|event| matches!(event, Event::StepBegin { step: Step::Cleanup }))
        {
            let leftover: u32 = (0..4)
                .map(|seat| armed(&game, seat as PlayerId))
                .sum();
            assert_eq!(
                leftover, 0,
                "CR 514.2: every unspent one-shot replacement ends in cleanup"
            );
            // Which seats held the spare? The snapshot taken before this action
            // still records it, because cleanup clears the field inside this
            // same accepted action. Count every armed unit, not one per seat:
            // the heuristic may also arm the ability in a main phase, and those
            // arms are cleared here too.
            for (seat, count) in pre_armed_by_seat.iter().enumerate() {
                unspent_cleared += *count as usize;
                unspent_cleared_by_seat[seat] += *count as usize;
            }
        }

        if accepted.iter().any(|event| {
            matches!(
                event,
                Event::AbilityActivated {
                    card: CardId::WordsOfWilding,
                    ..
                }
            )
        }) {
            armed_total += 1;
        }
        events.extend(accepted);
        replay.record(ReplayStep {
            player,
            revision,
            decision,
            action,
        });
    }
    assert!(game.state.is_over(), "capped at {steps} steps, not terminal");
    assert_eq!(replay.seats, seats, "exact deck snapshots");
    assert!(
        !events.iter().any(|e| matches!(e, Event::Truncated { .. })),
        "no rejected actions"
    );
    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();
    let (replayed, replay_events) = replay.run().unwrap();
    assert_eq!(replayed.digest(), game.digest());
    assert_eq!(
        replayed.state.players.iter().map(|p| p.life).collect::<Vec<_>>(),
        game.state.players.iter().map(|p| p.life).collect::<Vec<_>>(),
        "the replay reproduces every seat's life total"
    );
    let save = SessionSave::capture(&game, replay.clone(), Vec::new(), false);
    let save_text = save.to_json();
    let loaded = SessionSave::from_json(&save_text).expect("fresh save reloads");
    assert_eq!(
        loaded.game, game.state,
        "fresh save preserves the exact state"
    );
    let out = root.join("evidence/issue17/slice35");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("driver-save.json"), save_text).expect("write save");
    std::fs::write(
        out.join("driver-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    assert!(
        casts.get("Words of Wilding").copied().unwrap_or(0) > 0,
        "Words of Wilding must be cast in the game"
    );
    let activations = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::AbilityActivated {
                    card: CardId::WordsOfWilding,
                    ..
                }
            )
        })
        .count();
    assert!(
        activations > 0,
        "the printed {{1}} ability must be activated"
    );
    assert!(
        replaced_draws > 0,
        "a real draw step must consume an armed replacement"
    );
    assert_eq!(
        tokens_from_replacements, replaced_draws,
        "one Bear token per replaced draw"
    );
    assert!(armed_total >= replaced_draws, "never more consumptions than arms");
    assert!(
        unspent_cleared > 0,
        "the spare armed replacement must be cleared at cleanup in a real game"
    );
    // The printed text is exhaustive: every armed "next time you would draw this
    // turn" replacement is either consumed by a draw or ends at that turn's
    // cleanup. Nothing else may happen to it, so the counts must balance.
    let leftover_at_end: usize = (0..4)
        .map(|seat| armed(&game, seat as PlayerId) as usize)
        .sum();
    assert_eq!(
        armed_total,
        replaced_draws + unspent_cleared + leftover_at_end,
        "every armed replacement is consumed by a draw or ends in cleanup (CR 514.2)"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::DrewFromEmpty { .. })),
        "no seat may attempt to draw from an empty library in this line"
    );

    let report = serde_json::json!({
        "driver": "evidence-only masked arm/cast/attack-preferring demonstrator; NOT product heuristic or strength evidence",
        "purpose": "review 297: the card is cast, its ability is activated, and a real draw step consumes the armed replacement",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "words_activations": activations,
        "replacements_armed": armed_total,
        "draws_replaced": replaced_draws,
        "draws_replaced_by_seat": replaced_by_seat,
        "unspent_replacements_cleared_at_cleanup": unspent_cleared,
        "unspent_cleared_by_seat": unspent_cleared_by_seat,
        "armed_remaining_at_game_end": leftover_at_end,
        "bear_tokens_from_replacements": tokens_from_replacements,
        "drew_from_empty_events": 0,
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("driver-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
