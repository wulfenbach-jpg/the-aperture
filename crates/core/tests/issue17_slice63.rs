//! Slice 63 tracer bullet: Warchief, King and Dragonlord's Servant.
mod common;
use aperture_core::*;
use common::*;
const P2: PlayerId = 2;

fn red(generic: u32, red: u32) -> ManaPool {
    ManaPool {
        red,
        colorless: generic,
        ..ManaPool::default()
    }
}
fn cast(g: &mut Game, id: ObjectId) -> Result<Vec<Event>, EngineError> {
    let p = g.decision_owner().expect("priority");
    g.submit_action(
        p,
        Action::CastSpell {
            card: id,
            targets: Vec::new(),
        },
    )
}
fn token_ids(g: &Game, p: PlayerId) -> Vec<ObjectId> {
    g.state
        .battlefield(p)
        .into_iter()
        .filter(|o| o.card == CardId::GoblinToken)
        .map(|o| o.id)
        .collect()
}

#[test]
fn warchief_reduces_only_goblin_generic_costs_and_stacks() {
    let mut g = fixture(6301)
        .battlefield(P0, &[CardId::GoblinWarchief, CardId::GoblinWarchief])
        .hand(P0, &[CardId::BeetlebackChief])
        .library(P0, &[CardId::Mountain; 8])
        .library(P1, &[CardId::Forest; 8])
        .mana(P0, red(0, 2))
        .main(P0, Step::PrecombatMain)
        .build();
    let chief = hand_card(&g, P0, CardId::BeetlebackChief);
    let log = cast(&mut g, chief).expect("{2}{R}{R} reduced by two Warchiefs to {R}{R}");
    assert!(log.iter().any(|e| matches!(
        e,
        Event::Cast {
            card: CardId::BeetlebackChief,
            ..
        }
    )));

    let mut control = fixture(6302)
        .battlefield(P0, &[CardId::GoblinWarchief])
        .hand(P0, &[CardId::BeetlebackChief])
        .library(P0, &[CardId::Mountain; 8])
        .library(P1, &[CardId::Forest; 8])
        .mana(P0, red(0, 2))
        .main(P0, Step::PrecombatMain)
        .build();
    let chief = hand_card(&control, P0, CardId::BeetlebackChief);
    assert!(
        cast(&mut control, chief).is_err(),
        "one Warchief leaves one generic unpaid"
    );

    // Dragonlord's Servant is a Goblin but reduces Dragon spells, never Goblins.
    let mut dragonlord = fixture(6303)
        .battlefield(P0, &[CardId::DragonlordsServant])
        .hand(P0, &[CardId::GoblinInstigator])
        .library(P0, &[CardId::Mountain; 8])
        .library(P1, &[CardId::Forest; 8])
        .mana(P0, red(0, 1))
        .main(P0, Step::PrecombatMain)
        .build();
    let goblin = hand_card(&dragonlord, P0, CardId::GoblinInstigator);
    assert!(
        cast(&mut dragonlord, goblin).is_err(),
        "Dragonlord does not reduce Goblin spells"
    );
}

#[test]
fn warchief_haste_includes_self_and_other_controlled_goblins_only() {
    let mut g = fixture(6304)
        .battlefield_sick(
            P0,
            &[
                CardId::GoblinWarchief,
                CardId::GoblinInstigator,
                CardId::GrizzlyBears,
            ],
        )
        .battlefield_sick(P1, &[CardId::GoblinInstigator])
        .main(P0, Step::PrecombatMain)
        .build();
    let warchief = battlefield_card(&g, P0, CardId::GoblinWarchief);
    let own = battlefield_card(&g, P0, CardId::GoblinInstigator);
    let other = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let enemy = battlefield_card(&g, P1, CardId::GoblinInstigator);
    submit_ok(&mut g, P0, Action::Pass);
    let attack = g.legal_attackers(P0);
    assert!(
        attack.contains(&warchief),
        "Warchief's text includes itself"
    );
    assert!(attack.contains(&own));
    assert!(!attack.contains(&other), "non-Goblin gets no haste");
    assert!(
        !g.legal_attackers(P1).contains(&enemy),
        "opponent's Goblins are not hasted"
    );
}

#[test]
fn king_anthem_and_mountainwalk_cross_controllers_and_exclude_source() {
    let mut g = fixture_n(6305, 3)
        .battlefield(
            P0,
            &[
                CardId::GoblinKing,
                CardId::GoblinKing,
                CardId::GoblinInstigator,
                CardId::GrizzlyBears,
            ],
        )
        .battlefield(P1, &[CardId::GoblinToken, CardId::GoblinToken])
        .battlefield(P2, &[CardId::GoblinToken])
        .library(P0, &[CardId::Mountain; 8])
        .library(P1, &[CardId::Forest; 8])
        .library(P2, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let kings: Vec<ObjectId> = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::GoblinKing)
        .map(|o| o.id)
        .collect();
    let own_goblin = battlefield_card(&g, P0, CardId::GoblinInstigator);
    let non_goblin = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let enemy_tokens = token_ids(&g, P1);
    let far_token = token_ids(&g, P2)[0];
    submit_ok(&mut g, P0, Action::Pass);
    assert_eq!(
        g.state.object(own_goblin).power(),
        3,
        "each King adds +1/+1"
    );
    assert_eq!(
        g.state.object(enemy_tokens[0]).power(),
        3,
        "opponents' Goblins get both anthems"
    );
    assert_eq!(
        g.state.object(far_token).power(),
        3,
        "all controllers' Goblins"
    );
    assert_eq!(
        g.state.object(kings[0]).power(),
        3,
        "a second King boosts the first"
    );
    assert_eq!(
        g.state.object(non_goblin).power(),
        2,
        "non-Goblin unaffected"
    );
    assert!(
        !g.state.object_has_no_abilities(enemy_tokens[0]),
        "King grants opposing Goblin tokens mountainwalk"
    );
    assert!(g.state.object_has_no_abilities(non_goblin));
}

#[test]
fn mountainwalk_reads_the_defending_players_effective_mountain_and_self_is_excluded() {
    let mut g = fixture(6306)
        .battlefield(P0, &[CardId::GoblinKing, CardId::GoblinInstigator])
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::Mountain])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareAttackers)
        .build();
    let king = battlefield_card(&g, P0, CardId::GoblinKing);
    let goblin = battlefield_card(&g, P0, CardId::GoblinInstigator);
    let blocker = battlefield_card(&g, P1, CardId::GrizzlyBears);
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    g.state.turn.priority = None;
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![goblin],
            defender: Some(P1),
        },
    );
    g.state.pending = Some(Pending::DeclareBlockers { player: P1 });
    g.state.turn.priority = None;
    assert!(
        g.submit_action(
            P1,
            Action::DeclareBlockers {
                blocks: vec![BlockDecl {
                    attacker: goblin,
                    blocker
                }]
            }
        )
        .is_err(),
        "defending Mountain makes another Goblin unblockable"
    );
    // A lone King does not grant itself mountainwalk: the same blocker is legal.
    g.state.pending = Some(Pending::DeclareBlockers { player: P1 });
    g.state.turn.attackers = vec![king];
    g.state.turn.blocked.clear();
    g.state.turn.blockers.clear();
    submit_ok(
        &mut g,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                attacker: king,
                blocker,
            }],
        },
    );

    let mut no_mountain = fixture(6307)
        .battlefield(P0, &[CardId::GoblinKing, CardId::GoblinInstigator])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareAttackers)
        .build();
    let goblin = battlefield_card(&no_mountain, P0, CardId::GoblinInstigator);
    let blocker = battlefield_card(&no_mountain, P1, CardId::GrizzlyBears);
    no_mountain.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    no_mountain.state.turn.priority = None;
    submit_ok(
        &mut no_mountain,
        P0,
        Action::DeclareAttackers {
            attackers: vec![goblin],
            defender: Some(P1),
        },
    );
    no_mountain.state.pending = Some(Pending::DeclareBlockers { player: P1 });
    no_mountain.state.turn.priority = None;
    submit_ok(
        &mut no_mountain,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                attacker: goblin,
                blocker,
            }],
        },
    );
}

#[test]
fn all_three_cards_cast_from_hand_through_the_stack() {
    let mut g = fixture(6308)
        .battlefield(
            P0,
            &[
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .hand(
            P0,
            &[
                CardId::GoblinWarchief,
                CardId::GoblinKing,
                CardId::DragonlordsServant,
            ],
        )
        .library(P0, &[CardId::Mountain; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    for card in [
        CardId::GoblinWarchief,
        CardId::GoblinKing,
        CardId::DragonlordsServant,
    ] {
        let id = hand_card(&g, P0, card);
        let log = cast(&mut g, id).unwrap_or_else(|e| panic!("{card:?} cast: {e:?}"));
        assert!(
            log.iter()
                .any(|e| matches!(e, Event::Cast { card: c, .. } if *c == card))
        );
        for _ in 0..8 {
            if g.state.battlefield(P0).iter().any(|o| o.id == id) {
                break;
            }
            let p = g.decision_owner().unwrap();
            submit_ok(&mut g, p, Action::Pass);
        }
        assert!(
            g.state.battlefield(P0).iter().any(|o| o.id == id),
            "{card:?} resolved"
        );
    }
}

#[test]
fn warchief_haste_recipients_lose_ability_and_haste_when_it_departs() {
    let mut g = fixture(6309)
        .battlefield_sick(
            P0,
            &[
                CardId::RuxaPatientProfessor,
                CardId::GoblinWarchief,
                CardId::GoblinToken,
                CardId::GoblinToken,
                CardId::GrizzlyBears,
            ],
        )
        .battlefield(P1, &[CardId::GoblinToken])
        .main(P0, Step::PrecombatMain)
        .build();
    let own_tokens: Vec<ObjectId> = token_ids(&g, P0);
    let live_token = own_tokens[0];
    let graveyard_token = own_tokens[1];
    let enemy_token = token_ids(&g, P1)[0];
    let non_goblin = battlefield_card(&g, P0, CardId::GrizzlyBears);
    if let Some(object) = g.state.objects.get_mut(&graveyard_token) {
        object.zone = Zone::Graveyard;
    }
    g.state.players[P0 as usize].graveyard.push(graveyard_token);
    assert!(
        g.state.object_has_no_abilities(graveyard_token),
        "not on the battlefield"
    );
    submit_ok(&mut g, P0, Action::Pass);
    assert!(
        !g.state.object_has_no_abilities(live_token),
        "Warchief grants it haste"
    );
    assert!(
        g.state.object_has_no_abilities(enemy_token),
        "not controlled by Warchief's controller"
    );
    assert!(g.state.object_has_no_abilities(non_goblin), "not a Goblin");
    let warchief = battlefield_card(&g, P0, CardId::GoblinWarchief);
    if let Some(object) = g.state.objects.get_mut(&warchief) {
        object.zone = Zone::Graveyard;
    }
    g.state.players[P0 as usize].graveyard.push(warchief);
    assert!(
        g.state.object_has_no_abilities(live_token),
        "haste grant departed"
    );
    assert!(
        !g.legal_attackers(P0).contains(&live_token),
        "vanilla token is summoning sick without haste"
    );
}

#[test]
fn warchief_reduces_generic_but_never_the_goblin_colored_pip() {
    let mut g = fixture(6310)
        .battlefield(P0, &[CardId::GoblinWarchief])
        .hand(P0, &[CardId::GoblinInstigator])
        .library(P0, &[CardId::Mountain; 8])
        .library(P1, &[CardId::Forest; 8])
        .mana(
            P0,
            ManaPool {
                colorless: 1,
                ..ManaPool::default()
            },
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let goblin = hand_card(&g, P0, CardId::GoblinInstigator);
    assert!(
        cast(&mut g, goblin).is_err(),
        "{{1}}{{R}} loses its generic pip but still requires red"
    );
}

#[test]
fn servant_static_is_typed_for_dragons_not_goblins() {
    let statics = aperture_core::ability::static_abilities(CardId::DragonlordsServant);
    assert!(
        statics.iter().any(|ability| matches!(
            ability,
            aperture_core::ability::StaticAbility::SubtypeSpellsYouCastCostLess {
                subtype: "Dragon",
                generic: 1,
                ..
            }
        )),
        "the printed subtype is Dragon, never Goblin"
    );
    let warchief = aperture_core::ability::static_abilities(CardId::GoblinWarchief);
    assert!(warchief.iter().any(|ability| matches!(
        ability,
        aperture_core::ability::StaticAbility::SubtypeSpellsYouCastCostLess {
            subtype: "Goblin",
            generic: 1,
            ..
        }
    )));
}
