#!/usr/bin/env python3
"""Slice 29 discriminating mutation REDs.

Each mutation is applied to the *current* bytes of a real source file, the
slice-29 test target is run, the output is saved, and the file is restored from
a byte copy taken before the mutation. The script fails unless the pre- and
post-mutation sha256 of every touched file are identical, so the tree is proven
to be back exactly as it was.

Each mutation removes exactly one piece of the animation and the focused suite
must then fail for that reason and no other.
"""
import hashlib
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
ENGINE = ROOT / "crates/core/src/engine.rs"
OUT = ROOT / "evidence/issue17/slice29"
TARGET = ["cargo", "test", "-p", "aperture-core", "--test", "issue17_slice29", "--locked"]

MUTATIONS = [
    (
        "mutation-layer4-addition-ignored",
        ENGINE,
        # The layer 4 creature-type addition is recorded but never makes the
        # permanent a creature.
        [
            (
                "                effect.layer == Layer::Type\n"
                "                    && effect.target == object.id\n"
                "                    && matches!(effect.kind, ContinuousKind::AddCreatureSubtypes(_))",
                "                effect.layer == Layer::Type && effect.target == object.id && false",
            ),
        ],
        "the additive layer 4 type change never makes the artifact a creature",
    ),
    (
        "mutation-layer5-ignored",
        ENGINE,
        # Layer 5 is recorded but the colour query ignores it, so the animated
        # Bear stays colourless.
        [
            (
                "            if let ContinuousKind::SetColor(chosen) = effect.kind {\n"
                "                set = Some(chosen);\n"
                "            }",
                "            if let ContinuousKind::SetColor(_) = effect.kind {\n"
                "                set = None;\n"
                "            }",
            ),
        ],
        "the layer 5 colour set is never effective",
    ),
    (
        "mutation-layer7b-ignored",
        ENGINE,
        # The 7b base setter is created but applies no base power/toughness, so
        # the animated artifact stays 0/0.
        [
            (
                "                ContinuousKind::SetBasePowerToughness { power, toughness } => {\n"
                "                    Some((effect.timestamp, effect.target, power, toughness))\n"
                "                }",
                "                ContinuousKind::SetBasePowerToughness { power, toughness } => {\n"
                "                    let _ = (power, toughness);\n"
                "                    None\n"
                "                }",
            ),
        ],
        "the layer 7b base power/toughness setter never applies",
    ),
    (
        "mutation-entry-sickness-dropped",
        ENGINE,
        # The CR 302.6 restriction is no longer recorded for a permanent that
        # can animate itself, so a Bear animated on the turn it arrived attacks.
        [
            (
                "        object.summoning_sick = is_creature || crate::ability::can_become_creature(card);",
                "        object.summoning_sick = is_creature;",
            ),
        ],
        "the animation-capable permanent no longer records summoning sickness on arrival",
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
        for line in result.stdout.splitlines():
            if line.startswith("test ") and "FAILED" in line:
                print(f"  {line}")
    if failures:
        print("FAILED:\n  " + "\n  ".join(failures))
        return 1
    print("all mutations produced a RED and every file was restored byte-for-byte")
    return 0


if __name__ == "__main__":
    sys.exit(main())
