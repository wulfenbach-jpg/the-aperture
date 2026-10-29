use aperture_core::ids::{ObjectId, Zone};
use aperture_core::replay::SessionSave;
use aperture_core::state::TriggerRef;
use aperture_core::CardId;
fn case(label: &str, apply: impl FnOnce(&mut SessionSave, ObjectId)) {
    let text = std::fs::read_to_string("evidence/issue17/slice12-review-demo-save.json").unwrap();
    let mut save = SessionSave::from_json(&text).unwrap();
    let id = ObjectId(541);
    assert_eq!(save.game.object(id).zone, Zone::Removed);
    apply(&mut save, id);
    match save.game.validate() {
        Ok(()) => println!("{label}: VALIDATED"),
        Err(e) => println!("{label}: REJECTED: {e}"),
    }
}
fn main() {
    case("trigger_queue", |s, id| s.game.trigger_queue.push(TriggerRef { source: id, card: CardId::GoblinToken, controller: 0, ability: 0, seq: 0 }));
    case("revealed", |s, id| s.game.revealed.push(id));
    case("hand_list", |s, id| s.game.players[0].hand.push(id));
}
