#!/usr/bin/env python3
"""Regenerate the slice-32 discriminating mutation REDs.

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
CATALOG = ROOT / "crates/core/src/catalog.rs"
OUT = ROOT / "evidence/issue17/slice32"

REVEAL_TEST = "every_seat_reveals_in_apnap_order_and_creates_that_many_bears"
TRAMPLE_TEST = "surrak_and_goreclaw_prints_its_own_trample"
DECLINE_TEST = "a_mandatory_target_trigger_never_offers_or_accepts_declining"

MUTATIONS = [
    (
        "red-mutation-revealed-count-ignored.log",
        ENGINE,
        """            let count = cards.len();
            self.create_tokens(seat, CardId::BearToken, count, events);""",
        """            let count = 0;
            self.create_tokens(seat, CardId::BearToken, count, events);""",
        {REVEAL_TEST},
    ),
    (
        "red-mutation-tokens-created-before-the-last-answer.log",
        ENGINE,
        """        revealed.insert(player, cards.to_vec());""",
        """        self.create_tokens(player, CardId::BearToken, cards.len(), events);
        revealed.insert(player, cards.to_vec());""",
        {REVEAL_TEST},
    ),
    (
        "red-mutation-surrak-row-loses-trample.log",
        CATALOG,
        """    card_trample!(
        SurrakAndGoreclaw, "Surrak and Goreclaw\"""",
        """    card!(
        SurrakAndGoreclaw, "Surrak and Goreclaw\"""",
        {TRAMPLE_TEST},
    ),
    (
        "red-mutation-mandatory-target-declinable.log",
        ENGINE,
        """                }) if *p == player && trigger_target_is_optional(trigger.card, trigger.ability) => {""",
        """                }) if *p == player => {""",
        {DECLINE_TEST},
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
            "issue17_slice32",
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
    originals = {path: path.read_bytes() for path in {ENGINE, CATALOG}}
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
        log = OUT / name
        log.write_text(
            f"mutation: {name}\nfile: {path.relative_to(ROOT)}\n"
            f"sha256_before: {before[str(path)]}\n"
            f"sha256_after_restore: {digest(path)}\n"
            f"cargo_exit: {code}\nexpected_failures: {sorted(expected)}\n"
            f"observed_failures: {sorted(observed)}\n"
            f"discriminating: {observed == expected}\n\n{output}",
            encoding="utf-8",
        )
        print(f"{name}: exit={code} observed={sorted(observed)} ok={observed == expected}")
        if observed != expected:
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
