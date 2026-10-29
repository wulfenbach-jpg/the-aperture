import json

PATH = "evidence/issue17/slice5/review/slice5-review-1.json"
data = json.load(open(PATH, encoding="utf-8"))
print("verdict:", data["verdict"])
print("findings:", [(f["id"], f["severity"]) for f in data["findings"]])
print("keys:", sorted(data))
required = {"verdict", "findings", "tallies", "report_paths"}
print("top-level keys present:", required.issubset(data))
