"""Build a private copy of the slice-26 journey driver under review280.

Copies the driver crate without its target dir, renames the package, and
redirects its output directory so the shipped evidence is never overwritten.
"""
import shutil
from pathlib import Path

root = Path(__file__).resolve().parents[3]
src = root / "evidence/issue17/slice26-journey-driver"
dst = root / "evidence/issue17/review280-journey"
if dst.exists():
    shutil.rmtree(dst)
(dst / "src").mkdir(parents=True)
shutil.copy2(src / "Cargo.lock", dst / "Cargo.lock")
toml = (src / "Cargo.toml").read_text(encoding="utf-8")
toml = toml.replace("issue17-slice26-journey", "issue17-review280-journey")
(dst / "Cargo.toml").write_text(toml, encoding="utf-8")
main = (src / "src/main.rs").read_text(encoding="utf-8")
assert 'evidence/issue17/slice26"' in main
main = main.replace('evidence/issue17/slice26"', 'evidence/issue17/review280-journey-out"')
(dst / "src/main.rs").write_text(main, encoding="utf-8")
print("copied to", dst)
print((dst / "Cargo.toml").read_text(encoding="utf-8"))
