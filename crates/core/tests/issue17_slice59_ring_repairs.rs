//! Focused real-trigger coverage for the corrected Ring package.
mod common;
use aperture_core::state::RingState;
use aperture_core::*;
use common::*;
const P2: PlayerId = 2;
const P3: PlayerId = 3;

fn ring(g: &mut Game, level: u8, bearer: ObjectId) {
    g.state.rings = vec![RingState::default(); g.state.seat_count()];
    g.state.rings[P0 as usize].temptations = level;
    g.state.rings[P0 as usize].bearer = Some(bearer);
}
fn attack(g: &mut Game, bearer: ObjectId, defender: PlayerId) {
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    g.state.turn.priority = None;
    submit_ok(
        g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![bearer],
            defender: Some(defender),
        },
    );
}
fn drain(g: &mut Game) {
    for _ in 0..200 {
        if g.state.is_over() || g.state.turn.step == Step::Cleanup {
            break;
        }
        if g.state.pending.is_some() {
            let a = auto_answer(g);
            let p = g.state.pending.as_ref().unwrap().player();
            submit_ok(g, p, a);
        } else {
            let p = g.decision_owner().unwrap();
            submit_ok(g, p, Action::Pass);
        }
    }
}

#[test]
fn ring_four_triggers_only_after_positive_combat_damage_and_hits_each_live_opponent() {
    let mut g = fixture_n(59501, 4)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .library(P2, &[CardId::Forest; 6])
        .library(P3, &[CardId::Forest; 6])
        .main(P0, Step::DeclareAttackers)
        .build();
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    ring(&mut g, 4, bear);
    attack(&mut g, bear, P3);
    let before = [20; 4];
    assert_eq!(g.state.player(P0).life, before[0]);
    assert_eq!(g.state.player(P1).life, before[1]);
    assert_eq!(g.state.player(P2).life, before[2]);
    drain(&mut g);
    assert_eq!(g.state.player(P1).life, before[1] - 3);
    assert_eq!(g.state.player(P2).life, before[2] - 3);
    assert_eq!(g.state.player(P3).life, before[3] - 5, "2 combat + 3 Ring");
}

#[test]
fn ring_four_does_not_trigger_when_fog_prevents_combat_damage() {
    let mut g = fixture_n(59502, 4)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .library(P2, &[CardId::Forest; 6])
        .library(P3, &[CardId::Forest; 6])
        .main(P0, Step::DeclareAttackers)
        .build();
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    ring(&mut g, 4, bear);
    g.state.prevent_all_combat_damage_turn = Some(1);
    attack(&mut g, bear, P3);
    drain(&mut g);
    assert_eq!(g.state.player(P1).life, 20);
    assert_eq!(g.state.player(P2).life, 20);
    assert_eq!(g.state.player(P3).life, 20);
}

#[test]
fn ring_three_delays_sacrifice_until_an_end_combat_stack_trigger() {
    let mut g = fixture(59503)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareAttackers)
        .build();
    let bearer = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let blocker = battlefield_card(&g, P1, CardId::GrizzlyBears);
    ring(&mut g, 3, bearer);
    attack(&mut g, bearer, P1);
    g.state.pending = Some(Pending::DeclareBlockers { player: P1 });
    g.state.turn.priority = None;
    submit_ok(
        &mut g,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                attacker: bearer,
                blocker,
            }],
        },
    );
    assert!(
        g.state.stack.iter().any(|s| s.card == CardId::TheRing),
        "becomes-blocked is a normal stack trigger"
    );
    assert_eq!(g.state.object(blocker).zone, Zone::Battlefield);
    for _ in 0..12 {
        if g.state.turn.step == Step::EndCombat && !g.state.stack.is_empty() {
            break;
        }
        if g.state.pending.is_some() {
            let a = auto_answer(&g);
            let p = g.state.pending.as_ref().unwrap().player();
            submit_ok(&mut g, p, a);
        } else if let Some(p) = g.decision_owner() {
            submit_ok(&mut g, p, Action::Pass);
        } else {
            break;
        }
    }
    assert_eq!(g.state.object(blocker).zone, Zone::Battlefield);
    assert!(
        g.state.stack.iter().any(|s| s.card == CardId::TheRing),
        "the delayed sacrifice waits on an EndCombat stack trigger"
    );
    drain(&mut g);
    assert_eq!(g.state.object(blocker).zone, Zone::Graveyard);
}

#[test]
fn ring_one_blocks_only_greater_power_and_evil_legend_choice_is_controlled() {
    let mut g = fixture(59504)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .battlefield(
            P1,
            &[
                CardId::HillGiant,
                CardId::GrizzlyBears,
                CardId::GrizzlyBears,
            ],
        )
        .main(P1, Step::DeclareBlockers)
        .build();
    let bearer = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let bigger = battlefield_card(&g, P1, CardId::HillGiant);
    let equal = battlefield_card(&g, P1, CardId::GrizzlyBears);
    ring(&mut g, 1, bearer);
    g.state.turn.attackers = vec![bearer];
    g.state.turn.defender = Some(P1);
    g.state.pending = Some(Pending::DeclareBlockers { player: P1 });
    g.state.turn.priority = None;
    assert!(
        g.submit_action(
            P1,
            Action::DeclareBlockers {
                blocks: vec![BlockDecl {
                    attacker: bearer,
                    blocker: bigger
                }]
            }
        )
        .is_err(),
        "a greater-power creature cannot block the Ring-bearer"
    );
    submit_ok(
        &mut g,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                attacker: bearer,
                blocker: equal,
            }],
        },
    );
    let _ = equal;
}

#[test]
fn effective_legend_group_is_owned_and_answered_only_by_its_controller() {
    // CR 704.5j: two printed legendary permanents with the same name controlled
    // by one player. A Ring-bearer is legendary too (CR 701.54c) but a single
    // legend beside an ordinary same-name permanent is not a legend group.
    let mut g = fixture(59509)
        .battlefield(
            P1,
            &[CardId::IsamaruHoundOfKonda, CardId::IsamaruHoundOfKonda],
        )
        .main(P1, Step::PrecombatMain)
        .build();
    let ids: Vec<_> = g
        .state
        .battlefield(P1)
        .into_iter()
        .filter(|object| object.card == CardId::IsamaruHoundOfKonda)
        .map(|object| object.id)
        .collect();
    g.submit_action(P1, Action::Pass)
        .expect("passing with two legends is a legal action");
    let Some(Pending::LegendaryChoice { player, options }) = &g.state.pending else {
        panic!("two same-name legendary permanents must produce a legend choice");
    };
    assert_eq!(*player, P1);
    assert_eq!(options.len(), 2);
    assert!(options.contains(&ids[0]) && options.contains(&ids[1]));
    let before = g.state.clone();
    assert!(
        g.submit_action(P0, Action::ChooseLegendaryKeep { permanent: ids[0] })
            .is_err(),
        "a foreign seat cannot answer P1's legend choice"
    );
    assert_eq!(g.state, before);
    submit_ok(
        &mut g,
        P1,
        Action::ChooseLegendaryKeep { permanent: ids[0] },
    );
    assert_eq!(g.state.object(ids[1]).zone, Zone::Graveyard);
}

#[test]
fn positive_ring_discard_save_roundtrips_and_resumes_the_same_trigger() {
    let mut g = fixture(59505)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .hand(P0, &[CardId::Forest])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareAttackers)
        .build();
    let bearer = battlefield_card(&g, P0, CardId::GrizzlyBears);
    ring(&mut g, 2, bearer);
    attack(&mut g, bearer, P1);
    for _ in 0..12 {
        if matches!(g.state.pending, Some(Pending::EffectDiscard { .. })) {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(matches!(
        g.state.pending,
        Some(Pending::EffectDiscard { player: P0 })
    ));
    let decks = vec![
        DeckList::parse_practice("fixture", "40 Forest\n").unwrap(),
        DeckList::parse_practice("fixture", "40 Forest\n").unwrap(),
    ];
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        decks,
        AiSettings::default(),
    )
    .unwrap();
    let before = g.state.clone();
    let json = SessionSave::capture(&g, replay, Vec::new(), true).to_json();
    let save = SessionSave::from_json(&json)
        .expect("a positive Ring discard decision is a valid resumable save");
    let mut loaded = Game::from_state(save.game).expect("the saved game restores");
    loaded
        .state
        .validate()
        .expect("restored state is structurally valid");
    assert_eq!(loaded.state.pending, before.pending);
    let cards: Vec<_> = loaded
        .state
        .player(P0)
        .hand
        .iter()
        .copied()
        .take(1)
        .collect();
    submit_ok(&mut loaded, P0, Action::Discard { cards });
    assert_eq!(loaded.state.turn.step, Step::DeclareAttackers);
    assert!(loaded.state.stack.is_empty());
    assert_eq!(loaded.state.turn.number, 1);
}

#[test]
fn two_queued_rangers_and_a_source_left_trigger_use_captured_controller_and_intervening_if() {
    let mut g = fixture(59506)
        .battlefield(
            P0,
            &[
                CardId::DunedainRangers,
                CardId::DunedainRangers,
                CardId::GrizzlyBears,
            ],
        )
        .hand(P0, &[CardId::Forest])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let bearer = battlefield_card(&g, P0, CardId::GrizzlyBears);
    // Two Rangers and no Ring-bearer: both landfall triggers queue. The first
    // resolution chooses a bearer; the second must fail its intervening-if.
    g.state.rings = vec![RingState::default(); g.state.seat_count()];
    let land = g.state.player(P0).hand[0];
    let events = submit_ok(&mut g, P0, Action::PlayLand { card: land });
    assert!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                Event::TriggerStaged {
                    card: CardId::DunedainRangers,
                    ..
                }
            ))
            .count()
            >= 2,
        "both landfall triggers are staged before the first resolves"
    );
    for _ in 0..12 {
        if matches!(g.state.pending, Some(Pending::RingTemptation { .. })) {
            break;
        }
        if g.state.pending.is_some() {
            let a = auto_answer(&g);
            let p = g.state.pending.as_ref().unwrap().player();
            submit_ok(&mut g, p, a);
        } else if let Some(p) = g.decision_owner() {
            submit_ok(&mut g, p, Action::Pass);
        } else {
            break;
        }
    }
    assert!(matches!(
        g.state.pending,
        Some(Pending::RingTemptation { player: P0, .. })
    ));
    // The trigger's source leaves while its choice waits. Validation and
    // resolution must use the captured controller, not a live source object.
    let source = g
        .state
        .stack
        .last()
        .expect("the resolving Rangers trigger is on the stack")
        .object;
    if let Some(object) = g.state.objects.get_mut(&source) {
        object.zone = Zone::Graveyard;
    }
    g.state.players[P0 as usize].graveyard.push(source);
    if let Some(Pending::RingTemptation { eligible, .. }) = &mut g.state.pending {
        // The source was also an eligible creature; once it has left it is no
        // longer selectable. Nothing else in the offer is rebuilt.
        eligible.retain(|id| *id != source);
    }
    if let Err(error) = g.state.validate() {
        panic!("source-left Ring temptation must remain valid: {error:?}");
    }
    submit_ok(
        &mut g,
        P0,
        Action::ChooseRingBearer {
            bearer: Some(bearer),
        },
    );
    for _ in 0..16 {
        if g.state.stack.is_empty() && g.state.pending.is_none() {
            break;
        }
        if g.state.pending.is_some() {
            panic!("the second queued landfall trigger must fizzle on its intervening-if");
        }
        if let Some(p) = g.decision_owner() {
            submit_ok(&mut g, p, Action::Pass);
        } else {
            break;
        }
    }
    assert_eq!(g.state.rings[P0 as usize].temptations, 1);
    assert_eq!(g.state.rings[P0 as usize].bearer, Some(bearer));
}

#[test]
fn ring_three_delayed_sacrifice_does_not_hit_a_bounced_and_recast_incarnation() {
    let mut g = fixture(59507)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareAttackers)
        .build();
    let bearer = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let blocker = battlefield_card(&g, P1, CardId::GrizzlyBears);
    ring(&mut g, 3, bearer);
    attack(&mut g, bearer, P1);
    g.state.pending = Some(Pending::DeclareBlockers { player: P1 });
    g.state.turn.priority = None;
    submit_ok(
        &mut g,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                attacker: bearer,
                blocker,
            }],
        },
    );
    // Resolve the becomes-blocked trigger, which schedules the delayed marker.
    for _ in 0..8 {
        if g.state
            .effects
            .iter()
            .any(|effect| effect.kind == aperture_core::layer::ContinuousKind::RingDelayedSacrifice)
        {
            break;
        }
        if g.state.pending.is_some() {
            let a = auto_answer(&g);
            let p = g.state.pending.as_ref().unwrap().player();
            submit_ok(&mut g, p, a);
        } else if let Some(p) = g.decision_owner() {
            submit_ok(&mut g, p, Action::Pass);
        } else {
            break;
        }
    }
    assert!(g.state.effects.iter().any(|effect| {
        effect.kind == aperture_core::layer::ContinuousKind::RingDelayedSacrifice
    }));
    // Bounce the old blocker to hand, then put a fresh incarnation of the same
    // card onto the battlefield with a new ObjectId before EndCombat.
    let old = g.state.objects.remove(&blocker).unwrap();
    let mut bounced = old.clone();
    bounced.zone = Zone::Hand;
    g.state.objects.insert(blocker, bounced);
    g.state.players[P1 as usize].hand.push(blocker);
    let new_id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    let mut recast = old;
    recast.id = new_id;
    recast.zone = Zone::Battlefield;
    recast.damage = 0;
    g.state.objects.insert(new_id, recast);
    // CR 506.4: a bounced blocker leaves combat; only its new incarnation stays
    // on the battlefield and outside this combat.
    g.state.turn.blockers.retain(|(b, _)| *b != blocker);
    g.state.turn.blocked.retain(|a| *a != blocker);
    g.state.turn.declared_blockers.retain(|b| *b != blocker);
    g.state.turn.unpaired_blockers.retain(|b| *b != blocker);
    drain(&mut g);
    assert_eq!(g.state.object(blocker).zone, Zone::Hand);
    assert_eq!(
        g.state.object(new_id).zone,
        Zone::Battlefield,
        "the old delayed sacrifice cannot hit the recast incarnation"
    );
}

#[test]
fn emblem_abilities_are_not_abilities_of_the_ring_bearer() {
    let mut g = fixture(59508)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bearer = battlefield_card(&g, P0, CardId::GrizzlyBears);
    ring(&mut g, 1, bearer);
    assert!(
        g.state.object_has_no_abilities(bearer),
        "The Ring's triggered abilities do not belong to the creature"
    );
    assert!(
        !aperture_core::ability::abilities(CardId::TheRing).is_empty(),
        "the emblem itself carries the abilities"
    );
    assert!(aperture_core::catalog::by_name("The Ring").is_none());
    assert!(
        !aperture_core::catalog::all()
            .iter()
            .any(|def| def.id == CardId::TheRing)
    );
}
