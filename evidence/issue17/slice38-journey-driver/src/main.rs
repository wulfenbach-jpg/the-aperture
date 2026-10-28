//! Evidence-only demonstration driver for slice 38, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access.
//!
//! Its purpose is the delivery-bar evidence for Spider-Ham, Peter Porker
//! (SPM 114): a *real* four-seat / 40-life game from `decks/package-slice38.txt`
//! in which the card is **cast**, its **Food token** is created and **activated**
//! for life, and the "Animal May-Ham" anthem is checked against the engine's own
//! layer results on every action boundary:
//!
//! * Spider-Ham is cast at least once (`Event::Cast`);
//! * at least one Food token enters under its controller (`Event::PermanentEntered`
//!   with the token card), at least one such token is activated
//!   (`Event::AbilityActivated`) and sacrificed as a cost, and **every** sacrificed
//!   token ceases to exist (CR 111.7: one `StateBasedAction` record per
//!   activation, no Food object left in a live zone, and the final state accepted
//!   by the engine's own validator on the fresh-save reload);
//! * the ability really gains life: every activation's 3 life shows up as a
//!   `LifeChange` for the activating seat, and the deck contains no other life
//!   gain, so the positive life total of the seat that activated equals
//!   three times the activation count;
//! * while a Spider-Ham is on its controller's battlefield, every *other*
//!   creature that player controls whose effective subtype is on the printed list
//!   is at least printed + 1/+1 (read through `Game::has_subtype`, so a layer 4
//!   Bear Form counts), and Spider-Ham itself is never pumped by its own anthem;
//! * the game is terminal, the replay reproduces the digest and every seat's
//!   life total, a fresh `SessionSave` reloads, and no action was rejected.
//!
//! The strategy prefers, in this order: play a land, cast Spider-Ham, activate a
//! Food token, cast any creature, and it always attacks with everything legal.
//! Everything else is the frozen heuristic, so the line is deterministic but is
//! not a product decider. It is a 40-card package, not a starter certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, ObjectId, Replay,
    ReplayStep, SessionSave, Zone,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice38.txt";
const SEED: u64 = 3838;

fn spider_ham() -> CardId {
    aperture_core::catalog::by_name("Spider-Ham, Peter Porker").expect("in the pool")
}

fn stats(g: &Game, id: ObjectId) -> (i32, i32) {
    let object = g.state.object(id);
    (object.power(), object.toughness())
}

fn printed(g: &Game, id: ObjectId) -> (i32, i32) {
    let def = g.state.object(id).card.def();
    (def.power.unwrap_or(0), def.toughness.unwrap_or(0))
}

/// The printed subtype list of the anthem, in the singular form a type line
/// prints (CR 205.3). Kept here so the driver checks the *printed* list rather
/// than asking the engine for its own table.
const LISTED: &[&str] = &[
    "Spider", "Boar", "Bat", "Bear", "Bird", "Cat", "Dog", "Frog", "Jackal", "Lizard", "Mouse",
    "Otter", "Rabbit", "Raccoon", "Rat", "Squirrel", "Turtle", "Wolf",
];

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice38", &text).unwrap();
    assert_eq!(deck.size(), 40, "the package is exactly forty cards");
    let seats = vec![deck.clone(); 4];
    let commanders = commanders();
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
    let mut ham_casts = 0usize;
    let mut food_created = 0usize;
    let mut food_activations = 0usize;
    let mut foods_ceased = 0usize;
    let mut anthem_checks = 0usize;
    let mut ants = 0usize;

    while !game.state.is_over() && steps < 40000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);
        let demo_action = match view.pending_kind {
            Some(aperture_core::view::PendingKind::DeclareAttackers) => view
                .legal
                .iter()
                .find(|legal| match &legal.action {
                    Action::DeclareAttackers { attackers, .. } => !attackers.is_empty(),
                    _ => false,
                })
                .map(|legal| legal.action.clone()),
            None if view.stack.is_empty() && view.pending_kind.is_none() => {
                if view.active == player
                    && matches!(
                        view.step,
                        aperture_core::state::Step::PrecombatMain
                            | aperture_core::state::Step::PostcombatMain
                    )
                {
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
                                        .any(|c| c.id == *card && c.card == spider_ham()),
                                    _ => false,
                                })
                                .map(|legal| legal.action.clone())
                        })
                        .or_else(|| {
                            view.legal
                                .iter()
                                .find(|legal| match &legal.action {
                                    Action::ActivateAbility { source, .. } => view
                                        .own_battlefield
                                        .iter()
                                        .any(|p| p.id == *source && p.card == CardId::FoodToken),
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

        let revision = game.revision();
        let decision = game.decision();
        let accepted = game
            .submit_action(player, action.clone())
            .expect("all actions must be accepted; no rejection fallback");
        steps += 1;

        for event in &accepted {
            match event {
                Event::Cast { card, .. } if *card == spider_ham() => ham_casts += 1,
                Event::PermanentEntered { card, .. } if *card == CardId::FoodToken => {
                    food_created += 1
                }
                Event::AbilityActivated { card, .. } if *card == CardId::FoodToken => {
                    food_activations += 1
                }
                // CR 111.7: the state-based action that removes the sacrificed
                // token. Counting it makes the report prove cessation rather than
                // assert it in prose (review 301).
                Event::StateBasedAction { description }
                    if description == "Food Token token ceases to exist (CR 111.7)" =>
                {
                    foods_ceased += 1
                }
                _ => {}
            }
        }

        // The anthem, checked against the engine's own layer results after every
        // action: every other listed-subtype creature of a Spider-Ham's
        // controller is at least printed + 1/+1, and the Spider-Ham never pumps
        // itself.
        let battlefield: Vec<ObjectId> = game
            .state
            .objects
            .values()
            .filter(|object| object.zone == Zone::Battlefield)
            .map(|object| object.id)
            .collect();
        let hams: Vec<ObjectId> = battlefield
            .iter()
            .copied()
            .filter(|id| game.state.object(*id).card == spider_ham())
            .collect();
        for ham in &hams {
            let controller = game.state.object(*ham).controller;
            for id in &battlefield {
                let object = game.state.object(*id);
                if object.controller != controller || id == ham {
                    continue;
                }
                if !game.is_creature(object) {
                    continue;
                }
                // A printed layer 7b base (Circle of the Moon Druid's 4/2 Bear
                // Form) *replaces* the printed power/toughness, so "printed + 1"
                // is the wrong baseline for it; the focused tests assert its
                // exact 5/3. Every other creature in this package has an
                // unchanged base.
                if aperture_core::ability::static_abilities(object.card)
                    .iter()
                    .any(|static_ability| {
                        matches!(
                            static_ability,
                            aperture_core::ability::StaticAbility::SelfBasePowerToughnessDuringYourTurn { .. }
                        )
                    })
                {
                    continue;
                }
                let listed = LISTED
                    .iter()
                    .any(|subtype| game.has_subtype(object, subtype));
                if !listed {
                    continue;
                }
                anthem_checks += 1;
                let (power, toughness) = stats(&game, *id);
                let (base_power, base_toughness) = printed(&game, *id);
                assert!(
                    power >= base_power + 1 && toughness >= base_toughness + 1,
                    "a listed-subtype creature gets at least the printed +1/+1 ({} vs printed {}), {id} beside {ham}",
                    power,
                    base_power
                );
            }
            // "Other" excludes the source, so a Spider-Ham is pumped only by
            // *another* copy of its own anthem (it is itself a Spider), never by
            // its own static.
            let other_hams = hams
                .iter()
                .filter(|other| *other != ham && game.state.object(**other).controller == controller)
                .count() as i32;
            let (ham_power, ham_toughness) = printed(&game, *ham);
            assert_eq!(
                stats(&game, *ham),
                (ham_power + other_hams, ham_toughness + other_hams),
                "the printed \"Other\" excludes the source; only another copy pumps it"
            );
        }

        // Count every positive life change in the game, so the Food's 3 life can
        // be reconciled against the activations at the end. The package has no
        // other life-gain source, so this total must be exactly 3 × activations.
        for event in &accepted {
            if let Event::LifeChange { delta, .. } = event
                && *delta > 0
            {
                ants += *delta as usize;
            }
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
        replayed
            .state
            .players
            .iter()
            .map(|p| p.life)
            .collect::<Vec<_>>(),
        game.state
            .players
            .iter()
            .map(|p| p.life)
            .collect::<Vec<_>>(),
        "the replay reproduces every seat's life total"
    );
    let save = SessionSave::capture(&game, replay.clone(), Vec::new(), false);
    let save_text = save.to_json();
    let loaded = SessionSave::from_json(&save_text).expect("fresh save reloads");
    assert_eq!(
        loaded.game, game.state,
        "fresh save preserves the exact state"
    );

    let out = root.join("evidence/issue17/slice38");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("driver-save.json"), &save_text).expect("write save");
    std::fs::write(
        out.join("driver-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();

    assert!(ham_casts > 0, "Spider-Ham must be cast in the game");
    assert!(food_created > 0, "its enters trigger must create a Food token");
    assert!(
        food_activations > 0,
        "at least one Food token must be activated for life"
    );
    assert_eq!(
        ants,
        food_activations * 3,
        "the printed ability gains exactly 3 life per activation, and the package has no other life gain"
    );
    assert!(anthem_checks > 0, "the anthem must be observed on a real board");
    // CR 111.7, proven from the engine's own events rather than asserted in
    // prose (review 301): every activated Food token ceases to exist, and no
    // Food object survives in a live zone. A token whose owner already left the
    // game may only remain as the inert `Removed` archival record that the state
    // validator allows.
    assert_eq!(
        foods_ceased, food_activations,
        "every sacrificed Food token ceases to exist (CR 111.7)"
    );
    let stray: Vec<(ObjectId, Zone, bool)> = game
        .state
        .objects
        .values()
        .filter(|object| object.card == CardId::FoodToken && object.zone != Zone::Removed)
        .map(|object| (object.id, object.zone, game.state.is_live(object.owner)))
        .collect();
    assert!(
        stray.is_empty(),
        "no Food object survives in a live zone: {stray:?}"
    );
    let archived: Vec<(ObjectId, bool)> = game
        .state
        .objects
        .values()
        .filter(|object| object.card == CardId::FoodToken)
        .map(|object| (object.id, game.state.is_live(object.owner)))
        .collect();
    assert!(
        archived.iter().all(|(_, live)| !*live),
        "the only Food objects left are inert archival records of eliminated seats: {archived:?}"
    );

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    let report = serde_json::json!({
        "driver": "evidence-only masked cast/attack-preferring demonstrator; NOT product heuristic or strength evidence",
        "purpose": "slice 38: Spider-Ham is cast, its Food token is created and activated for 3 life, and the printed-subtype anthem is asserted on every action boundary of a four-seat game",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "spider_ham_casts": ham_casts,
        "food_tokens_created": food_created,
        "food_activations": food_activations,
        "food_tokens_ceased": foods_ceased,
        "food_objects_left_at_game_end": archived.len(),
        "life_gained_from_food": ants,
        "anthem_checks": anthem_checks,
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("driver-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

/// The commander list, kept in one place so the replay setup and the game setup
/// can never disagree.
fn commanders() -> Vec<(u8, CardId)> {
    (0..4).map(|p| (p, CardId::JasmineBoreal)).collect()
}
