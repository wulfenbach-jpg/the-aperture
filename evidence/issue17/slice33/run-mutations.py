#!/usr/bin/env python3
"""Regenerate the slice-33 discriminating mutation REDs.

Each mutation is applied to the CURRENT committed bytes, the focused suite is
run, the failing test names are recorded, and every touched file is restored
byte-for-byte (sha256 before == after). A mutation that changes no test result
is itself a failure: it would mean the suite does not discriminate.
"""
import hashlib
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
ENGINE = ROOT / "crates/core/src/engine.rs"
ABILITY = ROOT / "crates/core/src/ability.rs"
OUT = ROOT / "evidence/issue17/slice33"

LIVE_TEST = "the_animation_adds_the_bear_type_and_reads_the_land_count_live"
CLEANUP_TEST = "the_animation_outlives_cleanup_and_ends_with_the_permanent"
LANDFALL_TEST = "landfall_targets_a_creature_you_control_and_puts_one_counter"

MUTATIONS = [
    (
        "red-mutation-characteristic-defining-value-snapshotted.log",
        ENGINE,
        """            let value = self.lands_controlled_by(controller);""",
        """            let value = 0;""",
        {LIVE_TEST},
    ),
    (
        "red-mutation-added-bear-subtype-ignored.log",
        ENGINE,
        """                ContinuousKind::AddCreatureSubtypes(line) if line.has(subtype) => added = true,""",
        """                ContinuousKind::AddCreatureSubtypes(line) if line.has(subtype) && false => {
                    added = true
                }""",
        {LIVE_TEST},
    ),
    (
        "red-mutation-animation-expires-at-cleanup.log",
        ENGINE,
        """                            "becomes a creature with type {} (this effect doesn't end) ({})",
                            line.text(),
                            Layer::Type.rules()
                        ),
                        duration: EffectDuration::WhileSourceOnBattlefield,""",
        """                            "becomes a creature with type {} (this effect doesn't end) ({})",
                            line.text(),
                            Layer::Type.rules()
                        ),
                        duration: EffectDuration::UntilEndOfTurn,""",
        {CLEANUP_TEST},
    ),
    (
        "red-mutation-landfall-watches-the-wrong-event.log",
        ABILITY,
        """        event: TriggerEvent::LandEnters,
        optional: false,
        target: TargetSpec::CreatureYouControl,""",
        """        event: TriggerEvent::SpellCast,
        optional: false,
        target: TargetSpec::CreatureYouControl,""",
        {LANDFALL_TEST},
    ),
]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_suite():
    result = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "aperture-core",
            "--test",
            "issue17_slice33",
            "--locked",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=900,
    )
    return result.returncode, result.stdout + result.stderr


def failed_tests(output):
    return {
        line.split(" ... ")[0].replace("test ", "").strip()
        for line in output.splitlines()
        if line.startswith("test ") and line.rstrip().endswith("FAILED")
    }


def main():
    originals = {path: path.read_bytes() for path in {ENGINE, ABILITY}}
    before = {str(path): digest(path) for path in originals}
    (OUT / "pre-mutation-sha256.txt").write_text(
        "".join(f"{name} {value}\n" for name, value in sorted(before.items())),
        encoding="utf-8",
    )
    problems = []
    for name, path, old, new, expected in MUTATIONS:
        original = originals[path]
        for eol in ("\r\n", "\n"):
            old_b = old.replace("\n", eol).encode("utf-8")
            if original.count(old_b) == 1:
                new_b = new.replace("\n", eol).encode("utf-8")
                break
        else:
            raise AssertionError(f"{name}: marker not found exactly once in {path}")
        path.write_bytes(original.replace(old_b, new_b))
        code, output = run_suite()
        path.write_bytes(original)
        observed = failed_tests(output)
        discriminating = expected <= observed
        log = OUT / name
        log.write_text(
            f"mutation: {name}\nfile: {path.relative_to(ROOT)}\n"
            f"sha256_before: {before[str(path)]}\n"
            f"sha256_after_restore: {digest(path)}\n"
            f"cargo_exit: {code}\nexpected_failures: {sorted(expected)}\n"
            f"observed_failures: {sorted(observed)}\n"
            f"discriminating: {discriminating}\n"
            "note: the expectation is that every named test fails; a mutation may\n"
            "also fail neighbouring tests that share the animated-permanent\n"
            "precondition, which is reported here rather than hidden.\n\n"
            f"{output}",
            encoding="utf-8",
        )
        print(f"{name}: exit={code} observed={sorted(observed)} ok={discriminating}")
        if not discriminating:
            problems.append(name)
    after = {str(path): digest(path) for path in originals}
    (OUT / "post-mutation-sha256.txt").write_text(
        "".join(f"{name} {value}\n" for name, value in sorted(after.items())),
        encoding="utf-8",
    )
    assert after == before, f"a mutated file changed: {before} -> {after}"
    if problems:
        print("NON-DISCRIMINATING MUTATIONS:", problems)
        return 1
    print("ALL MUTATIONS DISCRIMINATE AND EVERY FILE RESTORED BYTE-FOR-BYTE")
    return 0


if __name__ == "__main__":
    sys.exit(main())
