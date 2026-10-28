"""Restore two untracked review309 images from that archive's own commit.

Require a byte-for-byte match to the known EOL transformation before any write.
"""
import hashlib
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def main():
    for name in ("table-1024x640.png", "table-1280x720.png"):
        relative = "evidence/issue20/" + name
        target = ROOT / "evidence/issue17/review309-recovery/archive" / relative
        original = subprocess.run(
            ["git", "show", "e6babc6:" + relative],
            cwd=ROOT, check=True, capture_output=True,
        ).stdout
        damaged = target.read_bytes()
        transformed = original.replace(b"\r\n", b"\n").replace(b"\n", b"\r\n")
        if damaged != original:
            if damaged != transformed:
                raise RuntimeError("unknown bytes, refusing to rewrite: " + str(target))
            target.write_bytes(original)
        assert target.read_bytes() == original
        print(name, "RESTORED EXACT ARCHIVE BLOB", len(original), hashlib.sha256(original).hexdigest())


if __name__ == "__main__":
    main()
