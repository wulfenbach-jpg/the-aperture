import re
p = r"C:/Users/alex3/AppData/Local/hermes/cache/scratch/r293-gates.log"
text = open(p, encoding="utf-8", errors="replace").read()
sec = text.split("=== workspace ===")[1]
passed = failed = suites = 0
for m in re.finditer(r"test result: (\w+)\. (\d+) passed; (\d+) failed;", sec):
    suites += 1
    passed += int(m.group(2))
    failed += int(m.group(3))
print("suites(reported lines):", suites, "passed:", passed, "failed:", failed)
print("FAILED lines:", [l for l in sec.splitlines() if l.startswith("FAILED")][:5])
