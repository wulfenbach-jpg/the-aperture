"""Apply one discriminating control mutation for the slice-53 Ruxa suite.

Each control changes exactly one product decision, so the focused suite's result
tells you whether the assertions actually depend on it. `run-controls.sh` restores
the file from git afterwards; run this only on a committed tree.
"""

import sys

ROOT = "."

CONTROLS = {
    # Every Ruxa clause asks "does this creature have no abilities?". If the shared
    # predicate always answers "no", the graveyard target set, the anthem and the
    # damage permission must all stop matching.
    "no_abilities_predicate": (
        "crates/core/src/state.rs",
        """    pub fn object_has_no_abilities(&self, id: ObjectId) -> bool {
        let Some(object) = self.objects.get(&id) else {""",
        """    pub fn object_has_no_abilities(&self, id: ObjectId) -> bool {
        // CONTROL: the shared predicate answers "no" for every permanent.
        return false;
        let Some(object) = self.objects.get(&id) else {""",
    ),
    # The permission's whole effect is the as_though_unblocked set. Ignoring it
    # must put the damage back on the blockers.
    "assignment_ignored": (
        "crates/core/src/engine.rs",
        """    fn deal_combat_damage(&mut self, as_unblocked: &[ObjectId], events: &mut Vec<Event>) {
        if self.state.turn.combat_damage_done {""",
        """    fn deal_combat_damage(&mut self, as_unblocked: &[ObjectId], events: &mut Vec<Event>) {
        // CONTROL: the accepted permission is discarded.
        let as_unblocked: &[ObjectId] = &[];
        if self.state.turn.combat_damage_done {""",
    ),
}


def main(argv: list[str]) -> int:
    if len(argv) != 1 or argv[0] not in CONTROLS:
        print(f"usage: mutate.py [{'|'.join(CONTROLS)}]")
        return 2
    path, old, new = CONTROLS[argv[0]]
    text = open(path, encoding="utf-8").read()
    if old not in text:
        print(f"anchor not found in {path}")
        return 1
    open(path, "w", encoding="utf-8", newline="").write(text.replace(old, new, 1))
    print(f"applied {argv[0]} to {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
