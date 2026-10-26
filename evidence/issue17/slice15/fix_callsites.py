import io

p = "crates/core/src/engine.rs"
s = io.open(p, newline="").read()

old_validate = """            } => self.validate_activate_ability(player, *source, *ability, *mode, targets, &[]),
            Action::ActivateCostedAbility {
                source,
                ability,
                mode,
                targets,
                cost_cards,
            } => self
                .validate_activate_ability(player, *source, *ability, *mode, targets, cost_cards),"""
new_validate = """            } => self.validate_activate_ability(
                player,
                ActivationRequest {
                    source: *source,
                    ability: *ability,
                    mode: *mode,
                    targets,
                    cost_cards: &[],
                },
            ),
            Action::ActivateCostedAbility {
                source,
                ability,
                mode,
                targets,
                cost_cards,
            } => self.validate_activate_ability(
                player,
                ActivationRequest {
                    source: *source,
                    ability: *ability,
                    mode: *mode,
                    targets,
                    cost_cards,
                },
            ),"""
if s.count(old_validate) != 1:
    raise SystemExit(f"validate arm: {s.count(old_validate)} matches")
s = s.replace(old_validate, new_validate)

old_apply = """            } => {
                self.apply_activate_ability(player, *source, *ability, *mode, targets, &[], events)
            }"""
new_apply = """            } => self.apply_activate_ability(
                player,
                ActivationRequest {
                    source: *source,
                    ability: *ability,
                    mode: *mode,
                    targets,
                    cost_cards: &[],
                },
                events,
            ),"""
if s.count(old_apply) != 1:
    raise SystemExit(f"apply plain arm: {s.count(old_apply)} matches")
s = s.replace(old_apply, new_apply)

old_costed = """            } => self.apply_activate_ability(
                player,
                *source,
                *ability,
                *mode,
                targets,
                cost_cards,
                events,
            ),"""
new_costed = """            } => self.apply_activate_ability(
                player,
                ActivationRequest {
                    source: *source,
                    ability: *ability,
                    mode: *mode,
                    targets,
                    cost_cards,
                },
                events,
            ),"""
if s.count(old_costed) != 1:
    raise SystemExit(f"apply costed arm: {s.count(old_costed)} matches")
s = s.replace(old_costed, new_costed)

io.open(p, "w", newline="").write(s)
print("patched call sites")
