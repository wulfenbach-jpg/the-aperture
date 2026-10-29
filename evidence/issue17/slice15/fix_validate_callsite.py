import io

p = "crates/core/src/engine.rs"
s = io.open(p, newline="").read()

old = """            } => self.validate_activate_ability(player, *source, *ability, *mode, targets, &[]),
            Action::ActivateCostedAbility {
                source,
                ability,
                mode,
                targets,
                cost_cards,
            } => self
                .validate_activate_ability(player, *source, *ability, *mode, targets, cost_cards),"""
new = """            } => self.validate_activate_ability(
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

if s.count(old) != 1:
    raise SystemExit(f"matches: {s.count(old)}")
s = s.replace(old, new)
io.open(p, "w", newline="").write(s)
print("validate arm rewritten")
