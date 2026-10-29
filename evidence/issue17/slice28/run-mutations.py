#!/usr/bin/env python3
"""Slice 28 discriminating mutation REDs.

Each mutation is applied to the *current* bytes of a real source file, the
slice-28 test target is run, the output is saved, and the file is restored from
a byte copy taken before the mutation. The script fails unless the pre- and
post-mutation sha256 of every touched file are identical, so the tree is proven
to be back exactly as it was.

The point of each mutation is that it removes exactly one half of the new
behaviour and the focused test suite must then fail for that reason and no
other.
"""
import hashlib
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
ENGINE = ROOT / "crates/core/src/engine.rs"
OUT = ROOT / "evidence/issue17/slice28"
TARGET = ["cargo", "test", "-p", "aperture-core", "--test", "issue17_slice28", "--locked"]

MUTATIONS = [
    (
        "mutation-turntable-ignored",
        ENGINE,
        # Drop the "during your turn" condition from BOTH halves of Bear Form.
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
        "printed Bear Form is applied on every turn",
    ),
    (
        "mutation-layer7b-ignored",
        ENGINE,
        # The layer 7b half is recognised but applies nothing, so only the
        # subtype line reaches the object.
        [
            (
                "                    object.pump_power = power - object.card.def().power.unwrap_or(0);\n                    object.pump_toughness = toughness - object.card.def().toughness.unwrap_or(0);",
                "                    let _ = (power, toughness);",
            ),
        ],
        "the base power/toughness half is never applied",
    ),
    (
        "mutation-layer4-ignored",
        ENGINE,
        # The printed subtype line is recorded but never becomes effective.
        [
            (
                "                } = static_ability\n                {\n                    effective = Some(*line);",
                "                } = static_ability\n                {\n                    effective = effective;",
            ),
        ],
        "the layer 4 subtype line is never effective",
    ),
]


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    failures = []
    for name, path, edits, description in MUTATIONS:
        backup = path.with_suffix(path.suffix + f".{name}.bak")
        shutil.copyfile(path, backup)
        before = sha256(path)
        text = path.read_text(encoding="utf-8")
        for old, new in edits:
            if text.count(old) != 1:
                raise SystemExit(
                    f"{name}: anchor appears {text.count(old)} times in {path.name}; refusing to mutate"
                )
            text = text.replace(old, new)
        path.write_text(text, encoding="utf-8")
        try:
            result = subprocess.run(TARGET, cwd=ROOT, capture_output=True, text=True)
        finally:
            shutil.copyfile(backup, path)
            backup.unlink()
        after = sha256(path)
        log = OUT / f"red-{name}.log"
        log.write_text(
            f"# {name}: {description}\n"
            f"# file: {path.relative_to(ROOT)}\n"
            f"# sha256 before mutation: {before}\n"
            f"# sha256 after restore:   {after}\n"
            f"# restored byte-for-byte: {before == after}\n"
            f"# command: {' '.join(TARGET)}\n"
            f"# exit code: {result.returncode}\n\n"
            + result.stdout
            + result.stderr,
            encoding="utf-8",
        )
        print(f"{name}: exit={result.returncode} restored={before == after} log={log.name}")
        if result.returncode == 0:
            failures.append(f"{name}: the suite still passed under the mutation")
        if before != after:
            failures.append(f"{name}: {path.name} was not restored byte-for-byte")
        failed_lines = [line for line in result.stdout.splitlines() if line.startswith("test ") and "FAILED" in line]
        for line in failed_lines:
            print(f"  {line}")
    if failures:
        print("FAILED:\n  " + "\n  ".join(failures))
        return 1
    print("all mutations produced a RED and every file was restored byte-for-byte")
    return 0


if __name__ == "__main__":
    sys.exit(main())
