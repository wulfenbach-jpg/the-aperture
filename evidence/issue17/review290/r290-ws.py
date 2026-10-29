import re
text = open(r"C:/Users/alex3/AppData/Local/hermes/cache/scratch/r293-workspace-full.log", encoding="utf-8", errors="replace").read()
passed = failed = suites = ignored = 0
for m in re.finditer(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", text):
    suites += 1
    passed += int(m.group(2))
    failed += int(m.group(3))
    ignored += int(m.group(4))
print("suites:", suites, "passed:", passed, "failed:", failed, "ignored:", ignored)
print("exits:", [l for l in text.splitlines() if l.startswith("WORKSPACE_EXIT")])
print("failed lines:", [l for l in text.splitlines() if l.startswith("FAILED")][:5])
print("ignored tests:", [l for l in text.splitlines() if "ignored" in l and l.startswith("test ")][:5])
