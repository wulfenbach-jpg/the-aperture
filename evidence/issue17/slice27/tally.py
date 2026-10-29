import re
t = open('evidence/issue17/slice27/workspace-gates.log', encoding='utf-8', errors='replace').read()
suites = re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed', t)
print("suites", len(suites), "passed", sum(int(p) for _, p, _ in suites),
      "failed", sum(int(f) for _, _, f in suites),
      "non-ok", [s for s, _, _ in suites if s != 'ok'])
