//! Native prepublication regressions for the initial Ring implementation.
//! All setup below is explicit synthetic fixture state, not ordinary replay provenance.
mod common;
use aperture_core::state::RingState;
use aperture_core::*;
use common::*;

fn ring(g: &mut Game, level: u8, bearer: ObjectId) {
    g.state.rings = vec![RingState::default(); g.state.seat_count()];
    g.state.rings[0].temptations = level;
    g.state.rings[0].bearer = Some(bearer);
}
fn base(step: Step) -> Game {
    fixture(59111)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, step)
        .build()
}
fn sandbox_loads(g: &Game) -> bool {
    let decks = vec![DeckList::parse_practice("fixture", "40 Forest\n").unwrap(); 2];
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        decks,
        AiSettings::default(),
    )
    .unwrap();
    SessionSave::from_json(&SessionSave::capture(g, replay, vec![], true).to_json()).is_ok()
}
#[test]
fn ring_attack_draw_is_a_stack_trigger_not_a_declaration_side_effect() {
    let mut g = base(Step::DeclareAttackers);
    let bearer = battlefield_card(&g, P0, CardId::GrizzlyBears);
    ring(&mut g, 2, bearer);
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    g.state.turn.priority = None;
    let before = g.state.player(P0).hand_size();
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![bearer],
            defender: Some(P1),
        },
    );
    assert_eq!(
        g.state.player(P0).hand_size(),
        before,
        "Ring draw waits for its trigger to resolve (CR603.3)"
    );
    assert!(
        !g.state.stack.is_empty(),
        "Ring attack trigger must use the stack"
    );
    assert!(
        g.state.pending.is_none(),
        "not discarding during declaration"
    );
}
#[test]
fn ring_blocked_creates_a_stack_trigger_before_delaying_sacrifice() {
    let mut g = base(Step::DeclareBlockers);
    let bearer = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let blocker = battlefield_card(&g, P1, CardId::GrizzlyBears);
    ring(&mut g, 3, bearer);
    g.state.turn.attackers = vec![bearer];
    g.state.turn.defender = Some(P1);
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
        !g.state.stack.is_empty(),
        "becomes-blocked Ring ability must trigger, not schedule as a declaration side effect"
    );
    assert_eq!(g.state.object(blocker).zone, Zone::Battlefield);
}
#[test]
fn ring_state_refuses_another_seats_bearer_at_validate_and_sandbox_load() {
    let mut g = base(Step::PrecombatMain);
    let opponent = battlefield_card(&g, P1, CardId::GrizzlyBears);
    ring(&mut g, 1, opponent);
    let valid = g.state.validate().is_ok();
    let loads = sandbox_loads(&g);
    assert!(
        !valid && !loads,
        "foreign bearer validates={valid}, sandbox_loads={loads}"
    );
}
#[test]
fn forged_ring_pending_cannot_select_an_opponents_permanent() {
    let mut g = base(Step::PrecombatMain);
    let opponent = battlefield_card(&g, P1, CardId::GrizzlyBears);
    g.state.rings = vec![RingState::default(); 2];
    g.state.rings[0].temptations = 1;
    g.state.pending = Some(Pending::RingTemptation {
        player: P0,
        eligible: vec![opponent],
    });
    g.state.turn.priority = None;
    let valid = g.state.validate().is_ok();
    let loads = sandbox_loads(&g);
    let before = g.state.clone();
    let accepted = g
        .submit_action(
            P0,
            Action::ChooseRingBearer {
                bearer: Some(opponent),
            },
        )
        .is_ok();
    assert!(
        !valid && !loads && !accepted,
        "forged choice validates={valid}, sandbox_loads={loads}, accepted={accepted}"
    );
    assert_eq!(g.state, before, "refusal is atomic");
}
#[test]
fn ordinary_blocking_does_not_materialize_empty_ring_state() {
    let mut g = base(Step::DeclareBlockers);
    let bearer = battlefield_card(&g, P0, CardId::GrizzlyBears);
    g.state.turn.attackers = vec![bearer];
    g.state.turn.defender = Some(P1);
    g.state.pending = Some(Pending::DeclareBlockers { player: P1 });
    g.state.turn.priority = None;
    assert!(g.state.rings.is_empty());
    submit_ok(&mut g, P1, Action::DeclareBlockers { blocks: vec![] });
    assert!(
        g.state.rings.is_empty(),
        "an ordinary combat must not insert serialized Ring bookkeeping"
    );
}

#[test]
fn one_ring_bearer_does_not_make_ordinary_same_named_creatures_legendary() {
    let mut g = fixture(59112)
        .battlefield(P0, &[CardId::GrizzlyBears, CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bearer = battlefield_card(&g, P0, CardId::GrizzlyBears);
    ring(&mut g, 1, bearer);
    submit_ok(&mut g, P0, Action::Pass);
    assert!(
        !matches!(g.state.pending, Some(Pending::LegendaryChoice { .. })),
        "CR704.5j requires two legendary permanents, not one legend plus an ordinary same-name permanent"
    );
}

#[test]
fn forged_legend_choice_must_not_remove_a_nonlegendary_same_name() {
    let mut g = fixture(59113)
        .battlefield(P0, &[CardId::GrizzlyBears, CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let options: Vec<_> = g.state.battlefield(P0).into_iter().map(|o| o.id).collect();
    ring(&mut g, 1, options[0]);
    g.state.pending = Some(Pending::LegendaryChoice {
        player: P0,
        options: options.clone(),
    });
    g.state.turn.priority = None;
    let valid = g.state.validate().is_ok();
    let loads = sandbox_loads(&g);
    let accepted = g
        .submit_action(
            P0,
            Action::ChooseLegendaryKeep {
                permanent: options[0],
            },
        )
        .is_ok();
    assert!(
        !valid && !loads && !accepted,
        "one legend is not a legendary group: validates={valid}, sandbox_loads={loads}, accepted={accepted}"
    );
}
