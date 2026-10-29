"""Run the slice-24 journey driver exactly as committed at HEAD (git show bytes).

The working tree carries an uncommitted edit to this file, so the copy must come
from `git show HEAD:<path>` rather than the file on disk. Only the output
directory and the crate name are redirected; the recorded assertion is unchanged.
"""
import shutil
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[3]
rel = "evidence/issue17/slice24-journey-driver"
dst = root / "evidence/issue17/review280-slice24-head"
if dst.exists():
    shutil.rmtree(dst)
(dst / "src").mkdir(parents=True)
src = root / rel
shutil.copy2(src / "Cargo.lock", dst / "Cargo.lock")
toml = (src / "Cargo.toml").read_text(encoding="utf-8").replace(
    "issue17-slice24-journey", "issue17-review280-slice24head"
)
(dst / "Cargo.toml").write_text(toml, encoding="utf-8")

head_main = subprocess.run(
    ["git", "show", f"HEAD:{rel}/src/main.rs"],
    cwd=root,
    capture_output=True,
    text=True,
    check=True,
).stdout
assert 'description.contains("enters with a +1/+1 counter and gains haste")' in head_main, "HEAD bytes not what we expect"
assert 'evidence/issue17/slice24"' in head_main
head_main = head_main.replace('evidence/issue17/slice24"', 'evidence/issue17/review280-slice24-head-out"')
(dst / "src/main.rs").write_text(head_main, encoding="utf-8")

log = root / "evidence/issue17/review280/slice24-head-driver.log"
with log.open("w", encoding="utf-8") as fh:
    proc = subprocess.run(
        ["cargo", "run", "--release", "--offline"],
        cwd=dst,
        stdout=fh,
        stderr=subprocess.STDOUT,
        text=True,
    )
print("HEAD slice24 driver exit", proc.returncode)
print("\n".join(log.read_text(encoding="utf-8", errors="replace").splitlines()[-18:]))
