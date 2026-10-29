//! Slice 55: two honesty defects in the approved Ursine Monstrosity increment.
//!
//! Both are behaviour, not prose: the trigger's event line narrated a grant that
//! the resolving ability had skipped, and the coach told the viewer that an
//! indestructible creature with lethal damage "will be destroyed".
mod common;
use aperture_core::layer::{ContinuousKind, EffectDuration, Layer};
use aperture_core::*;
use common::*;

/// The retained slice-54 control kept asserting the *absence* of the effect, so
/// the false part — the event line claiming the grant — survived it. This is the
/// same scenario with the log itself under assertion.
#[test]
fn departed_source_trigger_reports_the_absence_of_its_grants() {
    let mut g = fixture_n(55101, 4)
        .battlefield(P0, &[CardId::UrsineMonstrosity, CardId::Island])
        .hand(P0, &[CardId::Unsummon])
        .library(P0, &[CardId::Forest; 16])
        .library(P1, &[CardId::Forest; 16])
        .library(2, &[CardId::Forest; 16])
        .library(3, &[CardId::Forest; 16])
        .main(P0, Step::PrecombatMain)
        .build();
    let u = battlefield_card(&g, P0, CardId::UrsineMonstrosity);
    for _ in 0..32 {
        if g.state.turn.step == Step::BeginCombat && !g.state.stack.is_empty() {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(g.state.turn.step, Step::BeginCombat);
    assert_eq!(g.state.stack.len(), 1);
    let bounce = hand_card(&g, P0, CardId::Unsummon);
    let action = Action::CastSpell {
        card: bounce,
        targets: vec![Target::Permanent(u)],
    };
    assert!(g.legal_actions().iter().any(|a| a.action == action));
    submit_ok(&mut g, P0, action);
    let mut events = Vec::new();
    for _ in 0..64 {
        if g.state.stack.is_empty()
            && g.state.trigger_queue.is_empty()
            && g.state.resolving.is_none()
        {
            break;
        }
        let p = g.decision_owner().unwrap();
        events.extend(submit_ok(&mut g, p, Action::Pass));
    }
    assert_eq!(g.state.object(u).zone, Zone::Hand);
    assert!(
        events.iter().any(|e| matches!(e, Event::Milled { .. })),
        "the ability still resolves and mills after its source left (CR 608.2b)"
    );
    let lines: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            Event::StateBasedAction { description } => Some(description.clone()),
            _ => None,
        })
        .collect();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("has left the battlefield")),
        "the log must say the grants were skipped, got {lines:?}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("gains indestructible")),
        "the log must not narrate a grant that did not happen, got {lines:?}"
    );
}

/// Control: with the source still on the battlefield the same event line keeps
/// reporting the grant, so the new wording did not silence the ordinary case.
#[test]
fn a_present_source_still_reports_its_grant() {
    let mut g = fixture_n(55102, 2)
        .battlefield(P0, &[CardId::UrsineMonstrosity])
        .library(P0, &[CardId::Forest; 16])
        .library(P1, &[CardId::Forest; 16])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut events = Vec::new();
    for _ in 0..32 {
        if g.state.turn.step == Step::BeginCombat
            && g.state.stack.is_empty()
            && g.state.trigger_queue.is_empty()
        {
            break;
        }
        let p = g.decision_owner().unwrap();
        events.extend(submit_ok(&mut g, p, Action::Pass));
    }
    let u = battlefield_card(&g, P0, CardId::UrsineMonstrosity);
    assert!(g.state.effects.iter().any(|e| e.target == u
        && e.kind == ContinuousKind::Indestructible
        && e.duration == EffectDuration::UntilEndOfTurn));
    let lines: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            Event::StateBasedAction { description } => Some(description.clone()),
            _ => None,
        })
        .collect();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("gains indestructible")),
        "the ordinary grant is still narrated, got {lines:?}"
    );
}

fn coach_text(g: &Game, viewer: PlayerId) -> String {
    coach::explain(g, viewer, &g.legal_actions())
        .into_iter()
        .map(|e| e.text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn coach_does_not_promise_destruction_of_an_indestructible_creature() {
    let mut g = fixture_n(55103, 2)
        .battlefield(P0, &[CardId::UrsineMonstrosity])
        .library(P0, &[CardId::Forest; 16])
        .library(P1, &[CardId::Forest; 16])
        .main(P0, Step::PrecombatMain)
        .build();
    let u = battlefield_card(&g, P0, CardId::UrsineMonstrosity);
    // Fixture premise: the creature has already taken lethal damage.
    g.state.objects.get_mut(&u).unwrap().damage = 3;
    let before = coach_text(&g, P0);
    assert!(
        before.contains("will be destroyed"),
        "control: an ordinary creature with lethal damage is still reported, got {before}"
    );

    let id = g.state.next_effect;
    g.state.next_effect += 1;
    let timestamp = g.state.next_timestamp;
    g.state.next_timestamp += 1;
    g.state
        .effects
        .push(aperture_core::layer::ContinuousEffect {
            id,
            timestamp,
            source: Some(u),
            card: CardId::UrsineMonstrosity,
            controller: P0,
            layer: Layer::Ability,
            target: u,
            kind: ContinuousKind::Indestructible,
            duration: EffectDuration::UntilEndOfTurn,
        });

    let after = coach_text(&g, P0);
    assert!(
        !after.contains("will be destroyed"),
        "an indestructible creature is not destroyed by lethal damage (CR 702.12b), got {after}"
    );
    assert!(
        after.contains("indestructible") && after.contains("lethal damage"),
        "the coach states the real outcome, got {after}"
    );
    assert_eq!(g.state.object(u).zone, Zone::Battlefield);
}
