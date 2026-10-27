"""Review 296: independent re-run of the two slice-34 mutations.

Runs out of tree in the reviewer's own export of the reviewed head (no edit to
the lane's checkout). Each mutation is applied to the exported bytes, the
focused suite is run, and the file is restored from a saved copy whose sha256 is
verified afterwards.
"""
import hashlib
import pathlib
import subprocess
import sys

ROOT = pathlib.Path("C:/AI/r296cand")
ENGINE = ROOT / "crates/core/src/engine.rs"
ABILITY = ROOT / "crates/core/src/ability.rs"
OUT = pathlib.Path("C:/AI/aperture-worktrees/issue-17/evidence/issue17/review296")


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


def failed(output):
    return sorted(
        line.split(" ... ")[0].replace("test ", "").strip()
        for line in output.splitlines()
        if line.startswith("test ") and line.rstrip().endswith("FAILED")
    )


MUTATIONS = [
    (
        "mutation-a-source-excluded",
        ENGINE,
        "o.zone == Zone::Battlefield && self.is_creature(o) && o.controller == _caster\n                })\n                .map(|o| Target::Permanent(o.id))\n                .collect(),\n            // CR 115.1: \"another target creature you control\"",
        "o.zone == Zone::Battlefield\n                    && self.is_creature(o)\n                    && o.controller == _caster\n                    && o.id != exclude\n                })\n                .map(|o| Target::Permanent(o.id))\n                .collect(),\n            // CR 115.1: \"another target creature you control\"",
    ),
    (
        "mutation-b-little-bear-plain-spec",
        ABILITY,
        "    target: TargetSpec::AnotherCreatureYouControl,",
        "    target: TargetSpec::CreatureYouControl,",
    ),
]


def main():
    print("baseline sha256:", digest(ENGINE), digest(ABILITY))
    failures = []
    for name, path, old, new in MUTATIONS:
        original = path.read_bytes()
        before = hashlib.sha256(original).hexdigest()
        for eol in ("\r\n", "\n"):
            old_b = old.replace("\n", eol).encode()
            if original.count(old_b) == 1:
                new_b = new.replace("\n", eol).encode()
                break
        else:
            raise AssertionError(f"{name}: marker not found exactly once in {path}")
        path.write_bytes(original.replace(old_b, new_b))
        try:
            code, output = run_suite()
        finally:
            path.write_bytes(original)
        observed = failed(output)
        restored = digest(path)
        (OUT / f"{name}.log").write_text(
            f"mutation: {name}\nfile: {path.relative_to(ROOT)}\n"
            f"sha256_before: {before}\nrestored_ok: {restored == hashlib.sha256(original).hexdigest()}\n"
            f"cargo_exit: {code}\nfailed_tests: {observed}\n\n{output}",
            encoding="utf-8",
        )
        print(f"{name}: exit={code} failed={observed} restored={restored == hashlib.sha256(original).hexdigest()}")
        if len(observed) == 0:
            failures.append(name)
    print("final sha256:", digest(ENGINE), digest(ABILITY))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
