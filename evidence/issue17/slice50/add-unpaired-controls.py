from pathlib import Path
p=Path('crates/core/tests/issue17_slice46.rs')
s=p.read_text(encoding='utf-8')
s=s.replace('''    g.state.validate().unwrap();
    cast_at_creature(&mut g, P0, CardId::LightningStrike, blocker);''','''    g.state.validate().unwrap();
    assert_eq!(g.state.turn.unpaired_blockers, vec![blocker]);
    let mut loaded = fixture_save_round_trip(&g);
    cast_at_creature(&mut loaded, P0, CardId::LightningStrike, blocker);
    settle_through_combat(&mut loaded);
    cast_at_creature(&mut g, P0, CardId::LightningStrike, blocker);''')
s=s.replace('''        "the blocker never left combat before dying"
    );
}''','''        "the blocker never left combat before dying"
    );
    assert_eq!(g.state, loaded.state, "actual saved-state load continues identically");
    assert!(g.state.turn.unpaired_blockers.is_empty(), "combat ended");
}''')
s+='''
// Fixture saves exercise the actual loader and continuation. Dummy replay decks
// do not claim these hand-built initial states are reproducible legal replays.
fn fixture_save(g: &Game) -> SessionSave {
    let decks = vec![DeckList::parse_practice("fixture", "40 Forest\\n").unwrap(); 2];
    let replay = Replay::new(g.state.seed, g.state.first_player, decks, AiSettings::default()).unwrap();
    SessionSave::capture(g, replay, vec![], true)
}
fn fixture_save_round_trip(g: &Game) -> Game {
    let loaded = SessionSave::from_json(&fixture_save(g).to_json()).unwrap();
    Game { state: loaded.game }
}

#[test]
fn unpaired_blocker_and_paired_blocker_are_not_blocking_alone() {
    let mut g = fixture(5004)
        .hand(P0, &[CardId::Unsummon, CardId::LightningStrike, CardId::LightningStrike])
        .battlefield(P0, &[witness(), CardId::OrdinaryBear, CardId::OrdinaryBear, CardId::Island])
        .battlefield(P0, &[CardId::Mountain; 4])
        .battlefield(P1, &[CardId::GrizzlyBears; 2])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let attackers: Vec<_> = g.state.battlefield(P0).iter().filter(|o| o.card == CardId::OrdinaryBear).map(|o| o.id).collect();
    let blockers: Vec<_> = g.state.battlefield(P1).iter().map(|o| o.id).collect();
    attack_and_block(&mut g, &attackers, vec![
        BlockDecl { blocker: blockers[0], attacker: attackers[0] },
        BlockDecl { blocker: blockers[1], attacker: attackers[1] },
    ]);
    cast_at_creature(&mut g, P0, CardId::Unsummon, attackers[0]);
    assert_eq!(g.state.turn.unpaired_blockers, vec![blockers[0]]);
    cast_at_creature(&mut g, P0, CardId::LightningStrike, blockers[1]);
    assert_eq!(g.state.object(blockers[1]).zone, Zone::Graveyard);
    assert!(clue_tokens(&g, P0).is_empty(), "the unpaired blocker still counts beside it");
    cast_at_creature(&mut g, P0, CardId::LightningStrike, blockers[0]);
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(blockers[0]).zone, Zone::Exile);
    assert_eq!(clue_tokens(&g, P0).len(), 1);
}

#[test]
fn bounced_and_recast_unpaired_blocker_is_not_still_blocking() {
    let mut g = fixture(5005)
        .hand(P0, &[CardId::Unsummon, CardId::Unsummon, CardId::LightningStrike])
        .battlefield(P0, &[witness(), CardId::OrdinaryBear, CardId::Island, CardId::Island, CardId::Mountain, CardId::Mountain])
        .battlefield(P1, &[CardId::AshcoatBear, CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let attacker = battlefield_card(&g, P0, CardId::OrdinaryBear);
    let blocker = battlefield_card(&g, P1, CardId::AshcoatBear);
    attack_and_block(&mut g, &[attacker], vec![BlockDecl { blocker, attacker }]);
    cast_at_creature(&mut g, P0, CardId::Unsummon, attacker);
    assert_eq!(g.state.turn.unpaired_blockers, vec![blocker]);
    cast_at_creature(&mut g, P0, CardId::Unsummon, blocker);
    assert!(g.state.turn.unpaired_blockers.is_empty());
    submit_ok(&mut g, P0, Action::Pass);
    let recast = g.legal_actions().into_iter().find(|a| matches!(a.action, Action::CastSpell { card, .. } if card == blocker)).expect("same card's Flash recast is offered").action;
    submit_ok(&mut g, P1, recast);
    for _ in 0..8 {
        if g.state.stack.is_empty() { break; }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(g.state.object(blocker).zone, Zone::Battlefield);
    assert_eq!(g.decision_owner(), Some(P0));
    cast_at_creature(&mut g, P0, CardId::LightningStrike, blocker);
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(blocker).zone, Zone::Graveyard);
    assert!(clue_tokens(&g, P0).is_empty(), "same ObjectId is not the old blocking incarnation");
}

#[test]
fn malformed_unpaired_blockers_fail_validation_and_real_save_load() {
    let base = fixture(5006)
        .battlefield(P0, &[witness(), CardId::OrdinaryBear])
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::Forest])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareBlockers)
        .build();
    let blocker = battlefield_card(&base, P1, CardId::GrizzlyBears);
    let own = battlefield_card(&base, P0, CardId::OrdinaryBear);
    let land = battlefield_card(&base, P1, CardId::Forest);
    for case in 0..8 {
        let mut g = Game { state: base.state.clone() };
        g.state.turn.defender = Some(P1);
        g.state.turn.unpaired_blockers = vec![blocker];
        match case {
            0 => g.state.turn.unpaired_blockers.push(blocker),
            1 => g.state.turn.unpaired_blockers = vec![ObjectId(u32::MAX)],
            2 => g.state.turn.unpaired_blockers = vec![own],
            3 => g.state.turn.unpaired_blockers = vec![land],
            4 => g.state.turn.step = Step::PostcombatMain,
            5 => g.state.turn.defender = None,
            6 => g.state.turn.blockers.push((blocker, own)),
            7 => { let o = g.state.objects.get_mut(&blocker).unwrap(); o.zone = Zone::Hand; g.state.player_mut(P1).hand.push(blocker); },
            _ => unreachable!(),
        }
        assert!(g.state.validate().is_err(), "validator admitted case {case}");
        assert!(SessionSave::from_json(&fixture_save(&g).to_json()).is_err(), "real loader admitted case {case}");
    }
}
'''
p.write_text(s, encoding='utf-8')
