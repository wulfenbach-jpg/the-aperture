//! Slice 57: Rampaging Yao Guai (PIP 82), the pool's first `{X}` spell.
//!
//! Three printed clauses, all enabled by one package:
//!
//! 1. `{X}{G}{G}{G}` — X is announced as the spell is cast (CR 107.3a), paid as
//!    additional generic mana and carried with the spell (manifest value, entry
//!    counters, entry trigger), never guessed later.
//! 2. "This creature enters with X +1/+1 counters on it" — an entry replacement
//!    (CR 614.1c) applied before the permanent is on the battlefield.
//! 3. "When this creature enters, destroy any number of target artifacts and/or
//!    enchantments with total mana value X or less" — a target *set* under one
//!    aggregate restriction (CR 115.1, 601.2c), answered incrementally and
//!    committed by `Action::FinishTriggerTargets`.
mod common;
use aperture_core::counters::CounterKind;
use aperture_core::*;
use common::*;

/// Three Forests pay `{G}{G}{G}`; every extra Forest pays one more generic, so a
/// featureless six-Forest board can pay any X this suite asks for.
fn board(seed: u64, lands: usize) -> Fixture {
    let mut fixture = fixture_n(seed, 2)
        .hand(P0, &[CardId::RampagingYaoGuai])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain);
    for _ in 0..lands {
        fixture = fixture.battlefield(P0, &[CardId::Forest]);
    }
    fixture
}

fn cast_yao_guai(g: &mut Game, x: u8) -> ObjectId {
    let card = hand_card(g, P0, CardId::RampagingYaoGuai);
    submit_ok(
        g,
        P0,
        Action::CastSpellWithX {
            card,
            x,
            targets: Vec::new(),
        },
    );
    for _ in 0..64 {
        if g.state.pending.is_some() {
            break;
        }
        if g.state.stack.is_empty() && g.state.trigger_queue.is_empty() {
            break;
        }
        let p = g.decision_owner().expect("decision owner");
        submit_ok(g, p, Action::Pass);
    }
    battlefield_card(g, P0, CardId::RampagingYaoGuai)
}

fn aggregate_pending(g: &Game) -> (u8, Vec<Target>, Vec<Target>) {
    match &g.state.pending {
        Some(Pending::TriggerTarget {
            aggregate: Some(budget),
            legal,
            chosen,
            ..
        }) => (*budget, legal.clone(), chosen.clone()),
        other => panic!("expected an aggregate target choice, got {other:?}"),
    }
}

fn offered(g: &Game) -> Vec<Target> {
    g.legal_actions()
        .into_iter()
        .filter_map(|a| match a.action {
            Action::ChooseTriggerTarget { target } => Some(target),
            _ => None,
        })
        .collect()
}

fn counter_count(g: &Game, id: ObjectId) -> u32 {
    g.state.object(id).counters.get(CounterKind::PlusOnePlusOne)
}

fn tapped_lands(g: &Game) -> usize {
    g.state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::Forest && o.tapped)
        .count()
}

fn save(g: &Game) -> SessionSave {
    // Fixture-state round trips; these dummy decks do not replay the fixture.
    let decks = vec![DeckList::parse_practice("fixture", "40 Forest\n").unwrap(); 2];
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        decks,
        AiSettings::default(),
    )
    .unwrap();
    SessionSave::capture(g, replay, vec![], true)
}

fn loads(g: &Game) -> bool {
    SessionSave::from_json(&save(g).to_json()).is_ok()
}

// ----------------------------------------------------------------- the card

/// The catalog entry is the printed one: the fetched PIP 82 text verbatim, both
/// printed keywords, cost `{X}{G}{G}{G}`, and mana value 3 on the battlefield
/// (CR 202.3 reads X as 0 outside the stack).
#[test]
fn the_card_matches_its_printed_text_and_its_mana_value_is_three() {
    let def = CardId::RampagingYaoGuai.def();
    assert_eq!(def.name, "Rampaging Yao Guai");
    assert_eq!(def.cost, Some("{X}{G}{G}{G}"));
    assert!(def.has_x_cost(), "the printed cost contains {{X}}");
    assert_eq!(
        def.oracle,
        "Vigilance, trample\nThis creature enters with X +1/+1 counters on it.\n\
         When this creature enters, destroy any number of target artifacts and/or \
         enchantments with total mana value X or less."
    );
    assert_eq!(def.type_line, "Creature — Bear Mutant");
    assert_eq!((def.power, def.toughness), (Some(2), Some(2)));
    assert!(def.has_trample(), "printed trample");
    assert!(
        aperture_core::tokens::keywords(CardId::RampagingYaoGuai)
            .contains(&aperture_core::tokens::Keyword::Vigilance),
        "printed vigilance"
    );
    assert_eq!(def.mana_value(), 3, "{{X}} counts as 0 off the stack");
    let abilities = aperture_core::ability::abilities(CardId::RampagingYaoGuai);
    assert_eq!(abilities.len(), 1, "one printed triggered ability");
    assert_eq!(
        abilities[0].target_spec(),
        aperture_core::card::TargetSpec::ArtifactOrEnchantment
    );
}

// ---------------------------------------------------------------- announcing X

/// CR 107.3a: the value is a real choice. The ordinary cast action is refused for
/// a card that prints `{X}` and the X cast is refused for a card that does not, so
/// X can never be silently zero.
#[test]
fn x_is_announced_only_through_the_x_cast_on_an_x_cost_card() {
    let mut g = board(57101, 5).hand(P0, &[CardId::GrizzlyBears]).build();
    let yao = hand_card(&g, P0, CardId::RampagingYaoGuai);
    let err = submit_err(
        &mut g,
        P0,
        Action::CastSpell {
            card: yao,
            targets: Vec::new(),
        },
    );
    assert!(
        err.to_string().contains("announce X"),
        "plain cast of an X spell is refused, got {err}"
    );
    let bears = hand_card(&g, P0, CardId::GrizzlyBears);
    let err = submit_err(
        &mut g,
        P0,
        Action::CastSpellWithX {
            card: bears,
            x: 0,
            targets: Vec::new(),
        },
    );
    assert!(
        err.to_string().contains("does not print"),
        "an X cast of a card without {{X}} is refused, got {err}"
    );
    assert!(
        g.state.turn.priority == Some(P0),
        "neither refusal mutated the turn"
    );
}

/// CR 107.3a + 601.2b: an unaffordable X is refused before any mutation, and the
/// supported bound is enforced instead of wrapping a `u8`.
#[test]
fn an_unaffordable_or_out_of_range_x_is_refused() {
    let mut g = board(57102, 3).build();
    let yao = hand_card(&g, P0, CardId::RampagingYaoGuai);
    let unaffordable = submit_err(
        &mut g,
        P0,
        Action::CastSpellWithX {
            card: yao,
            x: 2,
            targets: Vec::new(),
        },
    );
    assert!(
        matches!(unaffordable, EngineError::CannotPay { .. }),
        "three Forests cannot pay {{2}}{{G}}{{G}}{{G}}, got {unaffordable:?}"
    );
    assert_eq!(tapped_lands(&g), 0, "no mana was spent by a refused cast");
    let out_of_range = submit_err(
        &mut g,
        P0,
        Action::CastSpellWithX {
            card: yao,
            x: 21,
            targets: Vec::new(),
        },
    );
    assert!(
        out_of_range.to_string().contains("supported maximum"),
        "X beyond the bound is refused, got {out_of_range}"
    );
}

/// X=0 is a real cast: the spell costs exactly `{G}{G}{G}`, the creature enters
/// 2/2 with no counters, and the entry trigger still fires (it simply has nothing
/// it may target).
#[test]
fn x_zero_is_a_real_cast_with_no_counters_and_no_targets() {
    let mut g = board(57103, 3).build();
    let yao = cast_yao_guai(&mut g, 0);
    assert_eq!(tapped_lands(&g), 3, "{{G}}{{G}}{{G}} was paid");
    assert_eq!(counter_count(&g, yao), 0, "X=0 places no counters");
    assert_eq!(g.state.object(yao).power(), 2);
    assert_eq!(g.state.object(yao).toughness(), 2);
    assert!(
        g.state.pending.is_none(),
        "no artifact or enchantment is on the board, and the empty set needs no prompt"
    );
    assert_eq!(g.state.object(yao).zone, Zone::Battlefield);
}

/// X=2 pays two extra generic mana and the permanent enters with two +1/+1
/// counters, which are characteristics (layer 7d) rather than a pump.
#[test]
fn x_counters_are_placed_on_entry_and_the_generic_is_paid() {
    let mut g = board(57104, 5).build();
    let yao = cast_yao_guai(&mut g, 2);
    assert_eq!(tapped_lands(&g), 5, "{{2}}{{G}}{{G}}{{G}} was paid");
    assert_eq!(counter_count(&g, yao), 2, "X +1/+1 counters on entry");
    assert_eq!(g.state.object(yao).power(), 4);
    assert_eq!(g.state.object(yao).toughness(), 4);
    assert_eq!(
        g.state.object(yao).zone,
        Zone::Battlefield,
        "the spell became a permanent"
    );
}

// --------------------------------------------------- the aggregate target set

/// Only targets whose mana value fits the announced X are offered, and both
/// controllers' artifacts and enchantments are candidates (CR 115.1).
#[test]
fn the_offer_holds_every_artifact_and_enchantment_inside_the_budget() {
    let mut g = board(57105, 5)
        .battlefield(P1, &[CardId::SolRing, CardId::ArcaneSignet])
        .battlefield(P0, &[CardId::BeornsHospitality, CardId::HedronArchive])
        .build();
    cast_yao_guai(&mut g, 2);
    let (budget, legal, chosen) = aggregate_pending(&g);
    assert_eq!(budget, 2, "the announced X is the budget");
    assert!(chosen.is_empty());
    let ring = battlefield_card(&g, P1, CardId::SolRing);
    let signet = battlefield_card(&g, P1, CardId::ArcaneSignet);
    let bearscape = battlefield_card(&g, P0, CardId::BeornsHospitality);
    let archive = battlefield_card(&g, P0, CardId::HedronArchive);
    let mut names: Vec<ObjectId> = offered(&g)
        .into_iter()
        .map(|target| match target {
            Target::Permanent(id) => id,
            other => panic!("unexpected target {other:?}"),
        })
        .collect();
    names.sort_unstable();
    let mut expected = vec![ring, signet, bearscape];
    expected.sort_unstable();
    assert_eq!(
        names, expected,
        "mana value 1, 2 and 2 are offered; Hedron Archive (4) is not"
    );
    assert!(!legal.contains(&Target::Permanent(archive)));
}

/// CR 601.2c: the running total may never exceed X, and a committed set of
/// exactly X is legal.
#[test]
fn a_target_set_may_reach_but_never_exceed_the_budget() {
    let mut g = board(57106, 6)
        .battlefield(
            P1,
            &[
                CardId::SolRing,
                CardId::ArcaneSignet,
                CardId::CommandersSphere,
            ],
        )
        .build();
    cast_yao_guai(&mut g, 3);
    let (budget, _, _) = aggregate_pending(&g);
    assert_eq!(budget, 3);
    let ring = battlefield_card(&g, P1, CardId::SolRing);
    let signet = battlefield_card(&g, P1, CardId::ArcaneSignet);
    let sphere = battlefield_card(&g, P1, CardId::CommandersSphere);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(ring),
        },
    );
    // Sol Ring spent 1 of 3, so Signet (2) still fits and Sphere (3) does not.
    assert_eq!(
        offered(&g),
        vec![Target::Permanent(signet)],
        "only the affordable addition is offered"
    );
    let err = submit_err(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(sphere),
        },
    );
    assert!(
        err.to_string().contains("over the announced X")
            || err.to_string().contains("not one of the offered"),
        "3 + 1 would exceed X=3, got {err}"
    );
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(signet),
        },
    );
    let (_, _, chosen) = aggregate_pending(&g);
    assert_eq!(chosen.len(), 2, "1 + 2 is exactly the budget");
}

/// The chosen set is destroyed as one instruction (CR 701.7a, 704.3): every
/// member leaves the battlefield and the unchosen permanent survives.
#[test]
fn the_chosen_set_is_destroyed_together() {
    let mut g = board(57107, 6)
        .battlefield(
            P1,
            &[
                CardId::SolRing,
                CardId::ArcaneSignet,
                CardId::CommandersSphere,
            ],
        )
        .build();
    let yao = cast_yao_guai(&mut g, 3);
    assert!(yao != ObjectId(0));
    let ring = battlefield_card(&g, P1, CardId::SolRing);
    let signet = battlefield_card(&g, P1, CardId::ArcaneSignet);
    let sphere = battlefield_card(&g, P1, CardId::CommandersSphere);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(ring),
        },
    );
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(signet),
        },
    );
    submit_ok(&mut g, P0, Action::FinishTriggerTargets);
    let mut destroyed = Vec::new();
    for _ in 0..64 {
        if g.state.stack.is_empty() {
            break;
        }
        let p = g.decision_owner().expect("decision owner");
        let events = submit_ok(&mut g, p, Action::Pass);
        for event in events {
            if let Event::StateBasedAction { description } = event
                && description.contains("were destroyed")
            {
                destroyed.push(description);
            }
        }
    }
    assert_eq!(g.state.object(ring).zone, Zone::Graveyard);
    assert_eq!(g.state.object(signet).zone, Zone::Graveyard);
    assert_eq!(
        g.state.object(sphere).zone,
        Zone::Battlefield,
        "an unchosen artifact is untouched"
    );
    assert_eq!(destroyed.len(), 1, "one destroy instruction: {destroyed:?}");
}

/// "Any number" includes zero: committing an empty set resolves the ability and
/// destroys nothing.
#[test]
fn an_empty_set_resolves_and_destroys_nothing() {
    let mut g = board(57108, 4).battlefield(P1, &[CardId::SolRing]).build();
    cast_yao_guai(&mut g, 1);
    let ring = battlefield_card(&g, P1, CardId::SolRing);
    submit_ok(&mut g, P0, Action::FinishTriggerTargets);
    for _ in 0..64 {
        if g.state.stack.is_empty() {
            break;
        }
        let p = g.decision_owner().expect("decision owner");
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(
        g.state.object(ring).zone,
        Zone::Battlefield,
        "nothing was chosen, so nothing is destroyed"
    );
}

/// A target may not be added twice: the printed set is a set, not a multiset.
#[test]
fn the_same_target_cannot_be_added_twice() {
    let mut g = board(57109, 6)
        .battlefield(P1, &[CardId::SolRing, CardId::MindStone])
        .build();
    cast_yao_guai(&mut g, 3);
    let ring = battlefield_card(&g, P1, CardId::SolRing);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(ring),
        },
    );
    assert!(
        !offered(&g).contains(&Target::Permanent(ring)),
        "the already chosen target is not offered again"
    );
    let err = submit_err(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(ring),
        },
    );
    assert!(
        err.to_string().contains("already part of the chosen"),
        "a duplicate is refused, got {err}"
    );
}

/// CR 608.2b by execution: when one chosen target leaves the battlefield in
/// response, the ability still resolves and destroys the surviving one. When the
/// only chosen target leaves, every target is illegal and nothing is destroyed.
#[test]
fn a_departed_target_costs_only_its_own_destruction() {
    let mut g = board(57110, 7)
        .hand(P1, &[CardId::Unsummon])
        .battlefield(
            P1,
            &[CardId::FriendlyTeddy, CardId::SolRing, CardId::Island],
        )
        .build();
    cast_yao_guai(&mut g, 4);
    let teddy = battlefield_card(&g, P1, CardId::FriendlyTeddy);
    let ring = battlefield_card(&g, P1, CardId::SolRing);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(teddy),
        },
    );
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(ring),
        },
    );
    submit_ok(&mut g, P0, Action::FinishTriggerTargets);
    // The trigger is on the stack; P1 responds by bouncing the artifact creature.
    let bounce = hand_card(&g, P1, CardId::Unsummon);
    let mut bounced = false;
    for _ in 0..16 {
        let p = g.decision_owner().expect("decision owner");
        if p == P1
            && g.legal_actions().iter().any(|a| {
                a.action
                    == Action::CastSpell {
                        card: bounce,
                        targets: vec![Target::Permanent(teddy)],
                    }
            })
        {
            submit_ok(
                &mut g,
                P1,
                Action::CastSpell {
                    card: bounce,
                    targets: vec![Target::Permanent(teddy)],
                },
            );
            bounced = true;
            break;
        }
        if g.state.stack.is_empty() {
            break;
        }
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(bounced, "the response was actually cast");
    for _ in 0..64 {
        if g.state.stack.is_empty() {
            break;
        }
        let p = g.decision_owner().expect("decision owner");
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(g.state.object(teddy).zone, Zone::Hand);
    assert_eq!(
        g.state.object(ring).zone,
        Zone::Graveyard,
        "the surviving target is still destroyed"
    );
}

// ------------------------------------------------------------- persistence

/// A mid-choice save is a reachable state: it validates, loads through a real
/// `SessionSave::from_json`, and the reloaded game continues to the same result.
#[test]
fn a_mid_choice_save_loads_and_continues() {
    let mut g = board(57111, 6)
        .battlefield(P1, &[CardId::SolRing, CardId::ArcaneSignet])
        .build();
    cast_yao_guai(&mut g, 3);
    let ring = battlefield_card(&g, P1, CardId::SolRing);
    let signet = battlefield_card(&g, P1, CardId::ArcaneSignet);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(ring),
        },
    );
    assert!(g.state.validate().is_ok(), "the honest pending is valid");
    assert!(loads(&g), "the honest pending loads through SessionSave");
    let loaded = SessionSave::from_json(&save(&g).to_json()).expect("fixture state loads");
    let mut resumed = Game { state: loaded.game };
    assert_eq!(resumed.state.pending, g.state.pending);
    submit_ok(
        &mut resumed,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(signet),
        },
    );
    submit_ok(&mut resumed, P0, Action::FinishTriggerTargets);
    for _ in 0..64 {
        if resumed.state.stack.is_empty() {
            break;
        }
        let p = resumed.decision_owner().expect("decision owner");
        submit_ok(&mut resumed, p, Action::Pass);
    }
    assert_eq!(resumed.state.object(ring).zone, Zone::Graveyard);
    assert_eq!(resumed.state.object(signet).zone, Zone::Graveyard);
}

/// Every forged version of this pending is refused by `GameState::validate`
/// *and* by a real load, so a save can never smuggle an illegal target set.
#[test]
fn forged_aggregate_pendings_are_refused_by_validate_and_by_a_real_load() {
    let mut g = board(57112, 5)
        .battlefield(P1, &[CardId::SolRing, CardId::CommandersSphere])
        .build();
    cast_yao_guai(&mut g, 1);
    let ring = battlefield_card(&g, P1, CardId::SolRing);
    let sphere = battlefield_card(&g, P1, CardId::CommandersSphere);
    let honest = g.state.pending.clone().expect("pending");
    let (_, legal, _) = aggregate_pending(&g);

    let with = |chosen: Vec<Target>, aggregate: Option<u8>, legal: Vec<Target>| match &honest {
        Pending::TriggerTarget {
            player,
            trigger,
            queued,
            ..
        } => Pending::TriggerTarget {
            player: *player,
            trigger: *trigger,
            legal,
            queued: queued.clone(),
            first: None,
            aggregate,
            chosen,
        },
        _ => unreachable!(),
    };

    // (a) a chosen target over the announced budget
    let mut over = g.state.clone();
    over.pending = Some(with(
        vec![Target::Permanent(sphere)],
        Some(1),
        legal.clone(),
    ));
    assert!(over.validate().is_err(), "3 mana value is over X=1");
    assert!(
        !loads(&Game { state: over }),
        "a real load refuses the same state"
    );

    // (b) a target that was never offered
    let mut unoffered = g.state.clone();
    unoffered.pending = Some(with(
        vec![Target::Permanent(sphere)],
        Some(3),
        legal.clone(),
    ));
    assert!(
        unoffered.validate().is_err(),
        "Commanders Sphere was never offered"
    );
    assert!(!loads(&Game { state: unoffered }));

    // (c) a budget that is not the trigger's announced X
    let mut wrong_budget = g.state.clone();
    wrong_budget.pending = Some(with(Vec::new(), Some(4), legal.clone()));
    assert!(
        wrong_budget.validate().is_err(),
        "the budget is the trigger's X"
    );
    assert!(!loads(&Game {
        state: wrong_budget
    }));

    // (d) the same target twice
    let mut duplicate = g.state.clone();
    duplicate.pending = Some(with(
        vec![Target::Permanent(ring), Target::Permanent(ring)],
        Some(3),
        legal.clone(),
    ));
    assert!(duplicate.validate().is_err(), "a set has no duplicates");
    assert!(!loads(&Game { state: duplicate }));

    // (e) a non-artifact, non-enchantment permanent
    let mut wrong_kind = g.state.clone();
    let bear = 0;
    wrong_kind.pending = Some(with(
        vec![Target::Permanent(ObjectId(bear))],
        Some(3),
        legal,
    ));
    assert!(
        wrong_kind.validate().is_err(),
        "a creature that is not an artifact can never be in the set"
    );
    assert!(!loads(&Game { state: wrong_kind }));

    // (f) an aggregate answer on a trigger that does not print it
    let mut wrong_trigger = g.state.clone();
    if let Some(Pending::TriggerTarget { trigger, .. }) = wrong_trigger.pending.clone() {
        let mut forged_trigger = trigger;
        forged_trigger.card = CardId::GrizzlyBears;
        forged_trigger.ability = 0;
        wrong_trigger.pending = Some(match &honest {
            Pending::TriggerTarget { queued, .. } => Pending::TriggerTarget {
                player: P0,
                trigger: forged_trigger,
                legal: Vec::new(),
                queued: queued.clone(),
                first: None,
                aggregate: Some(1),
                chosen: Vec::new(),
            },
            _ => unreachable!(),
        });
    }
    assert!(
        wrong_trigger.validate().is_err(),
        "only a printed aggregate ability may carry a set choice"
    );
    assert!(!loads(&Game {
        state: wrong_trigger
    }));

    // The honest state still round-trips, so the refusals discriminate.
    assert!(g.state.validate().is_ok());
    assert!(loads(&g));
}
