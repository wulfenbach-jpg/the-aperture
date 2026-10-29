#!/usr/bin/env python3
"""Regenerate the slice-31 discriminating mutation REDs.

Each mutation is applied to the CURRENT committed engine bytes, the focused
suite is run, the failing test names are recorded, and the file is restored
byte-for-byte (sha256 before == after). A mutation that changes no test result
is itself a failure: it would mean the suite does not discriminate.
"""
import hashlib
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
ENGINE = ROOT / "crates/core/src/engine.rs"
OUT = ROOT / "evidence/issue17/slice31"

MUTATIONS = [
    (
        "red-mutation-trample-counter-ignored.log",
        """            || object
                .counters
                .get(crate::counters::CounterKind::Trample)
                > 0
""",
        "            || false\n",
        {"trample_counter_lets_the_creature_trample_over_a_blocker"},
    ),
    (
        "red-mutation-bear-count-ignored.log",
        "                if bears < minimum as usize {",
        "                if false {",
        # Ignoring the gate draws in the decline path too, so the test that
        # asserts an unchanged hand after declining fails as well.
        {"two_bears_do_not_draw", "trigger_offers_up_to_one_target_including_declining"},
    ),
    (
        "red-mutation-optional-target-made-mandatory.log",
        "            let optional = spec == TargetSpec::UpToOneCreatureYouControl;",
        "            let optional = false;",
        # With one legal target the engine then chooses it for the controller,
        # so the decline that the "on your turn" test submits is refused.
        {"trigger_fires_only_on_its_controllers_turn"},
    ),
    (
        "red-mutation-bear-anthem-disabled.log",
        "            let Some((power, toughness)) = bonus else {",
        "            let Some((power, toughness)) = bonus.filter(|_| false) else {",
        {
            "chosen_creature_gets_a_trample_counter_and_becomes_a_bear",
            "trample_counter_lets_the_creature_trample_over_a_blocker",
            "the_bear_type_ends_at_cleanup_but_the_counter_stays",
        },
    ),
]


def digest():
    return hashlib.sha256(ENGINE.read_bytes()).hexdigest()


def run_suite():
    result = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "aperture-core",
            "--test",
            "issue17_slice31",
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
    original = ENGINE.read_bytes()
    before = digest()
    (OUT / "pre-mutation-sha256.txt").write_bytes((before + "\n").encode("utf-8"))
    problems = []
    for name, old, new, expected in MUTATIONS:
        # engine.rs is CRLF on disk, so the marker is widened to match.
        old_b = old.replace("\n", "\r\n").encode("utf-8")
        new_b = new.replace("\n", "\r\n").encode("utf-8")
        assert original.count(old_b) == 1, f"{name}: marker occurs {original.count(old_b)} times"
        ENGINE.write_bytes(original.replace(old_b, new_b))
        code, output = run_suite()
        ENGINE.write_bytes(original)
        observed = failed_tests(output)
        log = OUT / name
        log.write_text(
            f"mutation: {name}\nengine_sha256_before: {before}\n"
            f"engine_sha256_after_restore: {digest()}\n"
            f"cargo_exit: {code}\nexpected_failures: {sorted(expected)}\n"
            f"observed_failures: {sorted(observed)}\n"
            f"discriminating: {observed == expected}\n\n{output}",
            encoding="utf-8",
        )
        print(f"{name}: exit={code} observed={sorted(observed)} ok={observed == expected}")
        if observed != expected:
            problems.append(name)
    after = digest()
    (OUT / "post-mutation-sha256.txt").write_bytes((after + "\n").encode("utf-8"))
    assert after == before, f"engine.rs changed: {before} -> {after}"
    if problems:
        print("NON-DISCRIMINATING MUTATIONS:", problems)
        return 1
    print("ALL MUTATIONS DISCRIMINATE AND ENGINE RESTORED BYTE-FOR-BYTE")
    return 0


if __name__ == "__main__":
    sys.exit(main())
