"""Produce genuine RED evidence for slice 15, then restore the source.

Each defect is introduced by a minimal, surgical edit of the shipped code (never
by editing a test), the focused suite is run, and the original bytes are put
back. A RED that does not fail is a bug in this script, not evidence.
"""

import io
import subprocess
import sys

ENGINE = "crates/core/src/engine.rs"
LOG = "evidence/issue17/slice15"


def run(cmd):
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    return result.returncode, result.stdout + result.stderr


def patched(theirs, mine, label):
    original = io.open(ENGINE, newline="").read()
    if original.count(theirs) != 1:
        print(f"SKIP {label}: anchor not unique ({original.count(theirs)})")
        return
    io.open(ENGINE, "w", newline="").write(original.replace(theirs, mine))
    code, output = run("cargo test -p aperture-core --test issue17_slice15 --locked")
    io.open(ENGINE, "w", newline="").write(original)
    with io.open(f"{LOG}/red-{label}.log", "w", encoding="utf-8") as fh:
        fh.write(output)
    failed = [line for line in output.splitlines() if line.strip().endswith("FAILED")]
    print(f"{label}: exit={code} failing_tests={len(failed)}")
    for line in failed:
        print("   ", line.strip())
    if code == 0:
        sys.exit(f"RED {label} did not fail — this is not evidence")


# 1. The Bear-token effect no longer creates tokens.
patched(
    "            Effect::CreateBearTokens(n) => {\n"
    "                self.create_tokens(controller, CardId::BearToken, u32::from(n) as usize, events);\n"
    "                false\n"
    "            }",
    "            Effect::CreateBearTokens(_n) => false,",
    "no-bear-token",
)

# 2. "Activate only as a sorcery" is not enforced.
patched(
    "        if activated.sorcery_timing && !self.is_sorcery_timing(player) {",
    "        if false && activated.sorcery_timing && !self.is_sorcery_timing(player) {",
    "no-sorcery-timing",
)

# 3. Card costs are not validated against the printed cost.
patched(
    "        self.validate_card_costs(player, object.card, activated.cost, cost_cards)?;",
    "        let _ = cost_cards;",
    "no-cost-validation",
)

# 4. The graveyard ability is not offered from the graveyard.
patched(
    "        for (source, card) in graveyard_sources {\n"
    "            self.push_activation_actions(player, source, card, &mut out);\n"
    "        }",
    "        for (_source, _card) in graveyard_sources {\n"
    "            // RED: graveyard activations are never offered\n"
    "        }",
    "no-graveyard-offer",
)

code, output = run("cargo test -p aperture-core --test issue17_slice15 --locked")
with io.open(f"{LOG}/green-after-red.log", "w", encoding="utf-8") as fh:
    fh.write(output)
print("restored suite exit", code)
if code != 0:
    sys.exit("suite not green after restoring — stop")
