import re, io, sys

path = "crates/core/src/engine.rs"
with open(path, "r", encoding="utf-8", newline="") as f:
    lines = f.readlines()

out = []
inserted = 0
for i, line in enumerate(lines):
    out.append(line)
    if re.match(r"^\s*affected: .*,$", line):
        nxt = lines[i + 1] if i + 1 < len(lines) else ""
        if "dead_object" not in nxt:
            indent = re.match(r"^(\s*)", line).group(1)
            nl = "\r\n" if line.endswith("\r\n") else "\n"
            out.append(f"{indent}dead_object: None,{nl}")
            inserted += 1

with open(path, "w", encoding="utf-8", newline="") as f:
    f.writelines(out)
print("inserted", inserted)
