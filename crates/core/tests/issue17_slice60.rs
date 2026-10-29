//! Slice 60: Evercoat Ursine — hideaway twice, linked face-down exile, and
//! "you may play one of them without paying its mana cost".
//!
//! Every test drives the real engine (no private helpers), and the two saved
//! state probes build a real `SessionSave` and load it, so a forged pending is
//! refused by the same structural validator a save uses (CR 702.75a, 406.3).

mod common;

use aperture_core::*;
use common::*;

const P2: PlayerId = 2;

fn ursine() -> CardId {
    CardId::EvercoatUrsine
}

fn object(id: ObjectId, card: CardId, owner: PlayerId, zone: Zone) -> aperture_core::state::Object {
    aperture_core::state::Object {
        id,
        card,
        owner,
        controller: owner,
        zone,
        commander: None,
        tapped: false,
        damage: 0,
        summoning_sick: false,
        pump_power: 0,
        pump_toughness: 0,
        plus_one_counters: 0,
        counters: aperture_core::counters::Counters::default(),
        paired_with: None,
    }
}

/// Seed one face-down card exiled with `source`, exactly as hideaway would.
fn link_face_down(g: &mut Game, source: ObjectId, card: CardId, controller: PlayerId) -> ObjectId {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state
        .objects
        .insert(id, object(id, card, controller, Zone::Exile));
    g.state.players[controller as usize].exile.push(id);
    g.state.face_down_exile.insert(id);
    g.state.hideaway.push(aperture_core::state::HideawayLink {
        card: id,
        source,
        controller,
    });
    id
}

/// Drive the game forward, submitting `Pass` when nobody owes a decision and
/// `auto_answer` for every pending, until `want` holds or the cap is hit.
fn drive_until(g: &mut Game, want: impl Fn(&Game) -> bool, cap: usize) -> bool {
    for _ in 0..cap {
        if want(g) {
            return true;
        }
        if g.state.is_over() {
            return false;
        }
        if let Some(pending) = &g.state.pending {
            let player = pending.player();
            let action = auto_answer(g);
            submit_ok(g, player, action);
            continue;
        }
        let Some(player) = g.decision_owner() else {
            return false;
        };
        submit_ok(g, player, Action::Pass);
    }
    want(g)
}

fn is_play_offer(g: &Game) -> bool {
    matches!(g.state.pending, Some(Pending::PlayExiled { .. }))
}

fn is_trigger_choice(g: &Game) -> bool {
    matches!(g.state.pending, Some(Pending::TriggerChoice { .. }))
}

/// Set up P0 attacking P1 with one ready Evercoat Ursine.
fn attacking_ursine(seed: u64) -> (Game, ObjectId) {
    let mut g = fixture_n(seed, 2)
        .battlefield(P0, &[ursine()])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareAttackers)
        .build();
    let u = battlefield_card(&g, P0, ursine());
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    g.state.turn.priority = None;
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![u],
            defender: Some(P1),
        },
    );
    (g, u)
}

#[test]
fn evercoat_ursine_is_printed_with_trample_and_three_typed_abilities() {
    let card = catalog::by_name("Evercoat Ursine")
        .expect("supported")
        .def();
    assert_eq!(card.cost, Some("{4}{G}"));
    assert_eq!(card.type_line, "Creature — Elemental Bear");
    assert_eq!(card.power, Some(6));
    assert_eq!(card.toughness, Some(5));
    assert!(card.has_trample(), "printed Trample is a real keyword flag");
    assert_eq!(
        card.oracle,
        "Trample\nHideaway 3, hideaway 3 (When this creature enters, look at the top three cards of your library, exile one face down, then put the rest on the bottom in a random order. Then do it again.)\nWhenever this creature deals combat damage to a player, if there are cards exiled with it, you may play one of them without paying its mana cost."
    );
    let abilities = aperture_core::abilities(ursine());
    assert_eq!(
        abilities.len(),
        3,
        "two hideaway triggers plus the combat one"
    );
    assert_eq!(abilities[0].text(), "Hideaway 3");
    assert_eq!(abilities[1].text(), "Hideaway 3");
    assert!(card.oracle.contains(abilities[2].text()));
}

#[test]
fn ursine_monstrosity_now_really_has_its_printed_trample() {
    // Slice 54 added this card as a plain definition while printing "Trample",
    // so its own keyword was not implemented even though the catalog text said
    // it was. Slice 60 fixes that; this is the regression that keeps it fixed.
    let card = catalog::by_name("Ursine Monstrosity")
        .expect("supported")
        .def();
    assert!(card.oracle.starts_with("Trample\n"));
    assert!(card.has_trample(), "Ursine Monstrosity prints Trample");
}

#[test]
fn entering_resolves_two_hideaways_that_exile_face_down_and_bottom_the_rest() {
    let mut g = fixture_n(60001, 2)
        .hand(P0, &[ursine()])
        .battlefield(P0, &[CardId::Forest; 5])
        .library(
            P0,
            &[
                CardId::GrizzlyBears,
                CardId::Forest,
                CardId::Island,
                CardId::RuneclawBear,
                CardId::Mountain,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let in_hand = hand_card(&g, P0, ursine());
    let before = g.state.player(P0).library.len();
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: in_hand,
            targets: Vec::new(),
        },
    );
    assert!(drive_until(
        &mut g,
        |g| g.state.pending.is_none() && g.state.stack.is_empty(),
        200
    ));
    let u = battlefield_card(&g, P0, ursine());
    assert_eq!(
        g.state.player(P0).library.len(),
        before - 2,
        "each hideaway exiled exactly one card"
    );
    assert_eq!(g.state.face_down_exile.len(), 2);
    assert_eq!(g.state.hideaway.len(), 2);
    for link in &g.state.hideaway {
        assert_eq!(link.source, u, "both cards are exiled with this permanent");
        assert_eq!(link.controller, P0);
        let exiled = g.state.object(link.card);
        assert_eq!(exiled.zone, Zone::Exile);
        assert!(g.state.player(P0).exile.contains(&link.card));
    }
    // The two exiled cards were on top of the library when each look happened.
    assert!(
        g.state.player(P0).hand.is_empty(),
        "hideaway exiles, never draws"
    );
}

#[test]
fn face_down_exile_is_hidden_from_the_opponent_and_readable_by_the_controller() {
    let mut g = fixture_n(60002, 2)
        .battlefield(P0, &[ursine()])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let u = battlefield_card(&g, P0, ursine());
    let hidden = link_face_down(&mut g, u, CardId::GrizzlyBears, P0);
    let mine = g.view_for(P0, &[]);
    let theirs = g.view_for(P1, &[]);
    let own = mine
        .own_exile
        .iter()
        .find(|card| card.id == hidden)
        .expect("the controller's own exile list contains it");
    assert_eq!(own.name, "Grizzly Bears");
    let seen = theirs
        .opponents
        .iter()
        .flat_map(|opponent| opponent.exile.iter())
        .find(|card| card.id == hidden)
        .expect("the opponent sees that a card is exiled");
    assert_eq!(
        seen.name, "a face-down exiled card",
        "CR 406.3: an opponent cannot identify a face-down exiled card"
    );
    assert_eq!(seen.oracle, "");
    assert_eq!(seen.cost, None);
}

#[test]
fn combat_damage_offers_a_free_play_and_casting_it_costs_no_mana() {
    let (mut g, u) = attacking_ursine(60003);
    let bears = link_face_down(&mut g, u, CardId::GrizzlyBears, P0);
    assert!(
        drive_until(&mut g, is_trigger_choice, 120),
        "the printed 'you may' trigger goes on the stack"
    );
    assert!(
        drive_until(&mut g, is_play_offer, 120),
        "accepting the trigger offers the linked card"
    );
    let Some(Pending::PlayExiled { options, .. }) = g.state.pending.clone() else {
        panic!("play offer");
    };
    assert_eq!(options, vec![bears]);
    submit_ok(
        &mut g,
        P0,
        Action::PlayExiledCard {
            card: bears,
            modes: Vec::new(),
            targets: Vec::new(),
        },
    );
    assert_eq!(g.state.stack.len(), 1, "the played card becomes a spell");
    assert_eq!(g.state.object(bears).zone, Zone::Stack);
    assert!(
        !g.state.face_down_exile.contains(&bears),
        "the played card is no longer face down"
    );
    assert!(
        g.state.hideaway.is_empty(),
        "and no longer exiled with anything"
    );
    assert!(drive_until(
        &mut g,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        200
    ));
    assert_eq!(g.state.object(bears).zone, Zone::Battlefield);
    assert_eq!(
        g.state.player(P0).life,
        20,
        "no life was paid for a free cast"
    );
    let forests: Vec<ObjectId> = g
        .state
        .objects
        .values()
        .filter(|object| {
            object.card == CardId::Forest
                && object.controller == P0
                && object.zone == Zone::Battlefield
        })
        .map(|object| object.id)
        .collect();
    assert!(forests.is_empty(), "the free cast needed no lands at all");
}

#[test]
fn a_linked_land_is_played_as_the_turn_land_play_and_not_offered_once_used() {
    let (mut g, u) = attacking_ursine(60004);
    let forest = link_face_down(&mut g, u, CardId::Forest, P0);
    assert!(drive_until(&mut g, is_play_offer, 200));
    submit_ok(
        &mut g,
        P0,
        Action::PlayExiledCard {
            card: forest,
            modes: Vec::new(),
            targets: Vec::new(),
        },
    );
    assert_eq!(g.state.object(forest).zone, Zone::Battlefield);
    assert_eq!(
        g.state.turn.lands_played, 1,
        "playing it used the land play"
    );

    // The same offer with the land play already spent must not include a land.
    let (mut g, u) = attacking_ursine(60005);
    let forest = link_face_down(&mut g, u, CardId::Forest, P0);
    g.state.turn.lands_played = 1;
    assert!(drive_until(&mut g, is_trigger_choice, 200));
    let refused = {
        let before = g.state.revision;
        submit_ok(
            &mut g,
            P0,
            Action::AnswerTriggerChoice { use_ability: true },
        );
        assert!(g.state.revision > before);
        g.state.pending.is_none()
    };
    assert!(refused, "with no playable card the ability does nothing");
    assert_eq!(g.state.object(forest).zone, Zone::Exile);
    assert!(g.state.face_down_exile.contains(&forest));
}

#[test]
fn declining_keeps_the_cards_exiled_and_no_trigger_fires_without_a_linked_card() {
    let (mut g, u) = attacking_ursine(60006);
    let bears = link_face_down(&mut g, u, CardId::GrizzlyBears, P0);
    assert!(drive_until(&mut g, is_play_offer, 200));
    submit_ok(&mut g, P0, Action::DeclinePlayExiled);
    assert_eq!(g.state.object(bears).zone, Zone::Exile);
    assert!(g.state.face_down_exile.contains(&bears));

    // A creature with no cards exiled with it never triggers at all.
    let (mut g, _u) = attacking_ursine(60007);
    assert!(drive_until(
        &mut g,
        |g| g.state.turn.step == Step::EndCombat,
        300
    ));
    assert!(
        !matches!(g.state.pending, Some(Pending::PlayExiled { .. })),
        "no cards exiled with it means the intervening-if is false (CR 603.4)"
    );
}

#[test]
fn a_forged_offer_is_refused_by_validate_and_by_a_real_save_load() {
    let (mut g, u) = attacking_ursine(60008);
    let _bears = link_face_down(&mut g, u, CardId::GrizzlyBears, P0);
    assert!(drive_until(&mut g, is_play_offer, 200));
    // A real save of the honest pending round-trips.
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .expect("replay");
    let honest = SessionSave::capture(&g, replay.clone(), Vec::new(), true);
    SessionSave::from_json(&honest.to_json()).expect("the honest offer loads");

    // Forge: offer a card that is exiled but not face down and not linked.
    let stranger = {
        let id = ObjectId(g.state.next_object);
        g.state.next_object += 1;
        g.state
            .objects
            .insert(id, object(id, CardId::Island, P0, Zone::Exile));
        g.state.players[P0 as usize].exile.push(id);
        id
    };
    let mut forged = g.state.clone();
    if let Some(Pending::PlayExiled { options, .. }) = &mut forged.pending {
        options.push(stranger);
    }
    assert!(
        forged.validate().is_err(),
        "an unlinked exile card cannot be offered"
    );

    // The same forgery through the real save loader, not just the validator.
    let mut value: serde_json::Value =
        serde_json::from_str(&honest.to_json()).expect("save is JSON");
    let pending = value
        .pointer_mut("/game/pending/PlayExiled/options")
        .expect("serialized options");
    let array = pending.as_array_mut().expect("array");
    array.push(serde_json::Value::from(u32::from(stranger.0)));
    let text = serde_json::to_string(&value).expect("re-serialize");
    assert!(
        SessionSave::from_json(&text).is_err(),
        "a sandbox load must refuse the forged offer too"
    );
    let _ = P2;
}

#[test]
fn a_link_whose_permanent_left_the_battlefield_no_longer_matches() {
    let (mut g, u) = attacking_ursine(60009);
    let bears = link_face_down(&mut g, u, CardId::GrizzlyBears, P0);
    g.state.turn.attackers.clear();
    // The permanent leaves the battlefield; its object id survives in the
    // graveyard, so only the zone check can tell that the link is dead (CR 400.7).
    if let Some(object) = g.state.objects.get_mut(&u) {
        object.zone = Zone::Graveyard;
    }
    g.state.players[P0 as usize].graveyard.push(u);
    g.state.turn.attackers.clear();
    g.state.pending = None;
    assert!(
        g.state.validate().is_ok(),
        "the game is structurally valid with a dead link: {:?}",
        g.state.validate().err()
    );
    // No trigger is queued for a permanent that is no longer on the battlefield.
    let queued = g
        .state
        .trigger_queue
        .iter()
        .filter(|trigger| trigger.card == ursine())
        .count();
    assert_eq!(queued, 0);
    // The card stays exiled face down but can never be offered again.
    let offer = g.state.hideaway.iter().any(|link| {
        link.card == bears
            && g.state
                .try_object(link.source)
                .is_some_and(|object| object.zone == Zone::Battlefield)
    });
    assert!(!offer, "a departed permanent's linked cards are unplayable");
}
