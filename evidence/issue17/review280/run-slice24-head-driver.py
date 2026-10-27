"""Copy the HEAD slice-24 journey driver verbatim and run it as published.

Purpose: check whether the recorded slice-24 evidence is reproducible from the
committed bytes. Only the output directory and the crate name are redirected, so
the shipped evidence is never overwritten; the recorded assertion is unchanged.
"""
import shutil
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[3]
src = root / "evidence/issue17/slice24-journey-driver"
dst = root / "evidence/issue17/review280-slice24-driver"
if dst.exists():
    shutil.rmtree(dst)
(dst / "src").mkdir(parents=True)
shutil.copy2(src / "Cargo.lock", dst / "Cargo.lock")
toml = (src / "Cargo.toml").read_text(encoding="utf-8").replace(
    "issue17-slice24-journey", "issue17-review280-slice24"
)
(dst / "Cargo.toml").write_text(toml, encoding="utf-8")
main = (src / "src/main.rs").read_text(encoding="utf-8")
assert 'evidence/issue17/slice24"' in main
main = main.replace('evidence/issue17/slice24"', 'evidence/issue17/review280-slice24-out"')
(dst / "src/main.rs").write_text(main, encoding="utf-8")

log = root / "evidence/issue17/review280/slice24-driver-run.log"
with log.open("w", encoding="utf-8") as fh:
    proc = subprocess.run(
        ["cargo", "run", "--release", "--locked"],
        cwd=dst,
        stdout=fh,
        stderr=subprocess.STDOUT,
        text=True,
    )
print("slice24 HEAD driver exit", proc.returncode)
tail = log.read_text(encoding="utf-8", errors="replace").splitlines()[-25:]
print("\n".join(tail))
