"""Slice 23 mutation REDs: each mutation must fail exactly the assertions it targets.

Every mutation is applied to the committed source text, the focused suite is run,
and the file is restored byte-for-byte from the copy taken before the mutation.
"""
import hashlib
import pathlib
import subprocess

ROOT = pathlib.Path("C:/AI/aperture-worktrees/issue-17")
ENGINE = ROOT / "crates/core/src/engine.rs"
OUT = ROOT / "evidence/issue17/slice23"


def run_tests() -> tuple[int, str]:
    proc = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "aperture-core",
            "--test",
            "issue17_slice23",
            "--locked",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return proc.returncode, proc.stdout + proc.stderr


MUTATIONS = [
    (
        "red-mutation-cost-reduction-disabled.log",
        """        let def = card.def();
        if !def.is_creature() {
            return 0;
        }""",
        """        let def = card.def();
        if !def.is_creature() {
            return 0;
        }
        // MUTATION: the static reduction never applies.
        return 0;""",
    ),
    (
        "red-mutation-threshold-ignored.log",
        """                            && object.is_creature()
                            && object.power() >= minimum_power""",
        """                            && object.is_creature()
                            && (minimum_power <= 0 || true)""",
    ),
    (
        "red-mutation-trample-grant-ignored.log",
        """        object.card.def().has_trample()
            || self.state.effects.iter().any(|effect| {
                effect.kind == ContinuousKind::GrantsTrample && effect.target == object.id
            })""",
        """        object.card.def().has_trample()""",
    ),
]

original_bytes = ENGINE.read_bytes()
before = hashlib.sha256(original_bytes).hexdigest()
print("engine.rs sha256 before:", before)
OUT.mkdir(parents=True, exist_ok=True)
try:
    for log_name, needle, replacement in MUTATIONS:
        text = original_bytes.decode("utf-8")
        assert text.count(needle) == 1, (log_name, text.count(needle))
        mutated = text.replace(needle, replacement)
        ENGINE.write_bytes(mutated.encode("utf-8"))
        code, output = run_tests()
        (OUT / log_name).write_text(
            f"mutation: {log_name}\nfile: crates/core/src/engine.rs\n"
            f"test command: cargo test -p aperture-core --test issue17_slice23 --locked\n"
            f"exit code: {code}\n\n{output}",
            encoding="utf-8",
        )
        failed = [line for line in output.splitlines() if line.startswith("test ") and "FAILED" in line]
        print(f"\n=== {log_name}: exit {code}, failed lines:")
        for line in failed:
            print("   ", line)
finally:
    ENGINE.write_bytes(original_bytes)
after = hashlib.sha256(ENGINE.read_bytes()).hexdigest()
print("\nengine.rs sha256 after restore:", after)
assert after == before, "engine.rs was not restored byte-for-byte"
print("restored byte-for-byte")
