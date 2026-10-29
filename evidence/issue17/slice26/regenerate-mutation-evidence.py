"""Regenerate the slice-26 mutation REDs against the current tree.

The first capture was taken before `cargo fmt` and before the final test file
wording, so its recorded hashes and panic line numbers did not describe the
reviewed blob. This script records the hashes of the exact files it mutates,
runs each mutation, writes the log, restores the file, and proves the restore by
re-hashing. It fails loudly if a mutation is not discriminating.
"""
import hashlib
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(r"C:/AI/aperture-worktrees/issue-17")
OUT = ROOT / "evidence" / "issue17" / "slice26"

ENGINE = ROOT / "crates/core/src/engine.rs"
ABILITY = ROOT / "crates/core/src/ability.rs"

MUT_A_ORIGINAL = """        match effective {
            Some(line) => line.has(subtype),
            None => object.card.def().has_subtype(subtype),
        }"""
MUT_A_MUTANT = """        match effective {
            // MUTATION: layer 4 ignored, printed subtypes only
            Some(_) => object.card.def().has_subtype(subtype),
            None => object.card.def().has_subtype(subtype),
        }"""

MUT_B_ORIGINAL = """        Effect::SetSourceCreatureSubtypes(crate::layer::SubtypeLine::BearBerserker),
    ],
    once_per_turn: true,"""
MUT_B_MUTANT = """        Effect::SetSourceCreatureSubtypes(crate::layer::SubtypeLine::BearBerserker),
    ],
    // MUTATION: printed once-each-turn limit dropped
    once_per_turn: false,"""


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_tests(log_name):
    result = subprocess.run(
        ["cargo", "test", "-p", "aperture-core", "--test", "issue17_slice26", "--locked"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    (OUT / log_name).write_text(result.stdout + result.stderr, encoding="utf-8")
    failed = [
        line.split()[1]
        for line in result.stdout.splitlines()
        if line.startswith("test ") and line.endswith("FAILED")
    ]
    return result.returncode, failed


def mutate(path, original, mutant):
    """Replace bytes, never text: a text round-trip would rewrite line endings."""
    blob = path.read_bytes()
    for flavour in ("\n", "\r\n"):
        old = original.replace("\n", flavour).encode("utf-8")
        new = mutant.replace("\n", flavour).encode("utf-8")
        if old in blob:
            path.write_bytes(blob.replace(old, new, 1))
            return
    sys.exit(f"anchor missing in {path}")


def restore(path, original, mutant):
    blob = path.read_bytes()
    for flavour in ("\n", "\r\n"):
        old = mutant.replace("\n", flavour).encode("utf-8")
        new = original.replace("\n", flavour).encode("utf-8")
        if old in blob:
            path.write_bytes(blob.replace(old, new, 1))
            return
    sys.exit(f"mutant anchor missing in {path}")


def main():
    before = {"engine": sha256(ENGINE), "ability": sha256(ABILITY)}
    lines = ["# sha256 of the files the mutations were applied to, current reviewed tree"]
    lines.append(f"{before['engine']} *crates/core/src/engine.rs")
    lines.append(f"{before['ability']} *crates/core/src/ability.rs")
    (OUT / "pre-mutation-sha256.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")

    mutate(ENGINE, MUT_A_ORIGINAL, MUT_A_MUTANT)
    code_a, failed_a = run_tests("red-mutation-layer4-ignored.log")
    restore(ENGINE, MUT_A_ORIGINAL, MUT_A_MUTANT)

    mutate(ABILITY, MUT_B_ORIGINAL, MUT_B_MUTANT)
    code_b, failed_b = run_tests("red-mutation-once-per-turn-dropped.log")
    restore(ABILITY, MUT_B_ORIGINAL, MUT_B_MUTANT)

    after = {"engine": sha256(ENGINE), "ability": sha256(ABILITY)}
    print("mutation A exit", code_a, "failed:", failed_a)
    print("mutation B exit", code_b, "failed:", failed_b)
    print("restored byte-for-byte:", before == after, after)
    if code_a == 0 or code_b == 0:
        sys.exit("a mutation did not fail the suite")
    if before != after:
        sys.exit("the tree was not restored byte-for-byte")
    expected_a = {
        "a_bear_target_filter_sees_the_new_subtype",
        "activation_pumps_the_source_and_makes_it_a_bear_for_a_real_bear_check",
    }
    expected_b = {
        "the_catalog_string_is_the_recorded_printing_and_the_ability_is_typed",
        "the_printed_once_each_turn_restriction_is_enforced",
    }
    if set(failed_a) != expected_a:
        sys.exit(f"mutation A is not discriminating as claimed: {failed_a}")
    if set(failed_b) != expected_b:
        sys.exit(f"mutation B is not discriminating as claimed: {failed_b}")
    print("both REDs are exactly the claimed tests")


if __name__ == "__main__":
    main()
