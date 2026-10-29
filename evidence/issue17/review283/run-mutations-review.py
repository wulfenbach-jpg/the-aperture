#!/usr/bin/env python3
"""Reviewer's own mutation check for slice 28 (independent of the delta's script).

Three of the five mutations reproduce the delta's own discriminating mutations
(so the reviewer can see the REDs with their own eyes); two are the reviewer's
own, aimed at claims the delta does not mutate: (4) "It loses all other creature
types" (the layer 4 replacement, not merely its presence) and (5) the layer 7b
delta arithmetic (4/2 must be a *base*, not an addition on top of 2/4).

Every mutation is applied to the live file, the focused slice-28 suite is run,
and the file is restored from a byte copy; the script fails unless the pre- and
post-mutation sha256 match.
"""
import hashlib
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
ENGINE = ROOT / "crates/core/src/engine.rs"
OUT = pathlib.Path(__file__).resolve().parent / "mutations"
TARGET = ["cargo", "test", "-p", "aperture-core", "--test", "issue17_slice28", "--locked"]

MUTATIONS = [
    (
        "turntable-ignored",
        [
            (
                "if self.state.turn.active != controller {\n                continue;\n            }",
                "if false {\n                continue;\n            }",
            ),
            (
                "if object.zone == Zone::Battlefield && self.state.turn.active == object.controller {",
                "if object.zone == Zone::Battlefield {",
            ),
        ],
    ),
    (
        "layer7b-ignored",
        [
            (
                "                    object.pump_power = power - object.card.def().power.unwrap_or(0);\n                    object.pump_toughness = toughness - object.card.def().toughness.unwrap_or(0);",
                "                    let _ = (power, toughness);",
            ),
        ],
    ),
    (
        "layer4-ignored",
        [
            (
                "                } = static_ability\n                {\n                    effective = Some(*line);",
                "                } = static_ability\n                {\n                    effective = effective;",
            ),
        ],
    ),
    (
        "reviewer-layer4-keeps-printed-subtypes",
        [
            (
                "            Some(line) => line.has(subtype),",
                "            Some(line) => line.has(subtype) || object.card.def().has_subtype(subtype),",
            ),
        ],
    ),
    (
        "reviewer-layer7b-adds-instead-of-setting",
        [
            (
                "                    object.pump_power = power - object.card.def().power.unwrap_or(0);\n                    object.pump_toughness = toughness - object.card.def().toughness.unwrap_or(0);",
                "                    object.pump_power += power;\n                    object.pump_toughness += toughness;",
            ),
        ],
    ),
]


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    before = sha256(ENGINE)
    print(f"engine.rs sha256 before: {before}")
    failures = []
    for name, edits in MUTATIONS:
        backup = OUT / f"engine.rs.{name}.bak"
        shutil.copyfile(ENGINE, backup)
        text = ENGINE.read_text(encoding="utf-8")
        for old, new in edits:
            count = text.count(old)
            if count != 1:
                raise SystemExit(f"{name}: anchor occurs {count}x; refusing to mutate")
            text = text.replace(old, new)
        ENGINE.write_text(text, encoding="utf-8")
        try:
            result = subprocess.run(TARGET, cwd=ROOT, capture_output=True, text=True)
        finally:
            shutil.copyfile(backup, ENGINE)
            backup.unlink()
        after = sha256(ENGINE)
        failed = [
            line for line in result.stdout.splitlines()
            if line.startswith("test ") and "FAILED" in line
        ]
        log = OUT / f"red-{name}.log"
        log.write_text(
            f"# {name}\n# command: {' '.join(TARGET)}\n# exit: {result.returncode}\n"
            f"# sha256 before {before}\n# sha256 after restore {after}\n"
            f"# restored byte-for-byte: {before == after}\n\n"
            + result.stdout + result.stderr,
            encoding="utf-8",
        )
        print(f"{name}: exit={result.returncode} restored={before == after} failing_tests={len(failed)}")
        for line in failed:
            print(f"  {line}")
        if result.returncode == 0:
            failures.append(f"{name}: still GREEN under mutation")
        if before != after:
            failures.append(f"{name}: file not restored")
    final = sha256(ENGINE)
    print(f"engine.rs sha256 after all: {final}  equal={final == before}")
    if failures:
        print("FAILED:\n  " + "\n  ".join(failures))
        return 1
    print("all five mutations RED, engine.rs restored byte-for-byte")
    return 0


if __name__ == "__main__":
    sys.exit(main())
