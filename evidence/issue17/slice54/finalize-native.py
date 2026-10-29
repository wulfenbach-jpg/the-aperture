"""Produce the final native handoff only from completed real receipts."""
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
HEAD = "1b134db04cd2ddf03520c47384bfef7e1695d935"
ci = json.loads((HERE / "exact-head-ci-1b134db.json").read_text(encoding="utf-8"))
assert ci["head"] == HEAD and ci["all_success"] and ci["finished"]
assert ci["total"] == 10 and all(c["head_sha"] == HEAD and c["conclusion"] == "success" for c in ci["checks"])
log = (HERE / "native-gates.log").read_text(encoding="utf-8")
assert log.startswith("head=" + HEAD)
sections = re.split(r"\n=== ([^=]+) ===\n", log)
gates = {}
for i in range(1, len(sections), 2):
    name, body = sections[i:i + 2]
    exits = re.findall(r"EXIT=(\d+) \(" + re.escape(name) + r"\)", body)
    assert exits == ["0"], (name, exits)
    results = [tuple(map(int, m)) for m in re.findall(r"test result: ok\. (\d+) passed; (\d+) failed", body)]
    gates[name] = {"exit": 0, "test_result_lines": len(results), "passed": sum(r[0] for r in results), "failed": sum(r[1] for r in results)}
assert gates["focused_core"]["passed"] == 107 and gates["focused_core"]["test_result_lines"] == 9
assert gates["policy_adapter"]["passed"] == 16 and gates["policy_adapter"]["test_result_lines"] == 2
assert "whole_field_supported_card_citations = []" in log
journey = json.loads((HERE / "native/demo-journey.json").read_text(encoding="utf-8"))
default = json.loads((HERE / "native/default-report.json").read_text(encoding="utf-8"))
assert journey["status"] == default["status"] == "terminal"
assert journey["snapshots_exact"] and journey["replay_digest_matches"] and default["replay_digest_matches"]
assert "EXIT=0 (separate_release_cli)" in (HERE / "native-journey.log").read_text(encoding="utf-8")
assert "EXIT=0 (selfplay)" in (HERE / "native-default-policy.log").read_text(encoding="utf-8")
assert "EXIT=0 (replay)" in (HERE / "native-default-policy.log").read_text(encoding="utf-8")
manifest = json.loads((ROOT / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))
ayula = next(s for s in manifest["starters"] if s["id"] == "ayula-bear-counters")
assert len(ayula["missing_cards"]) == len(ayula["missing_interactions"]) == 3
result = {
    "status": "implementation increment ready for independent review",
    "full_issue17_accepted": False,
    "independent_review_outcome": None,
    "published_pr": "https://github.com/wulfenbach-jpg/the-aperture/pull/46",
    "branch": "aperture/issue-17",
    "base": "dc9bb3c0788df022853e5bdcbc00cbfab8035963",
    "initial_product": "7fa3b133ec231dc85e140c7823973fa5b51c0a4e",
    "native_functional_repair": "2d5cc18c35eb9f3c33c71a1f2bf0e5605d340e7a",
    "code_corpus_head": HEAD,
    "evidence_tip": {"sha_source": "native task post-publication metadata; this result is written before its own evidence commit", "ci_claim": False},
    "increment_card": "Ursine Monstrosity (DSC36), complete printed clauses in the declared supported model",
    "source": {"printing": "73cc6df4-3564-4ace-bf8a-eac3e62d725a", "retrieved_at": "2026-10-28T23:54:49.142002+00:00", "fixture": "fixtures/starter-corpus/slice54-sourced.json"},
    "corpus": {"catalog": 163, "union": 459, "supported": 150, "missing": 309, "per_list": [len(s["missing_cards"]) for s in manifest["starters"]], "certified": sum(s["status"] == "certified" for s in manifest["starters"]), "desired": 8, "ayula_missing": ayula["missing_cards"], "both_ayula_gap_fields": 3, "whole_field_log": "evidence/issue17/slice54/native-gates.log (corpus section)"},
    "gates": gates,
    "focused_aggregate": {"suites": 11, "passed": 123, "failed": 0, "definition": "9 core suites107 plus2 AI suites16; not workspace or search total"},
    "new_test_delta": {"base": "dc9bb3c", "new_tests": 11, "files": {"crates/core/tests/issue17_slice54.rs": 4, "crates/core/tests/issue17_slice54_regressions.rs": 6, "crates/ai/tests/issue17_ursine_policy.rs": 1}},
    "ci": {"head": HEAD, "total": 10, "completed_success": 10, "all_keyed": True, "rerun_requested": False, "receipt": "evidence/issue17/slice54/exact-head-ci-1b134db.json", "evidence_tip_claim": False},
    "red": [
        {"receipt": "evidence/issue17/slice54/native-red-attack-requirements.log", "compiles": True, "failed": 2, "exit": 101, "symptoms": "wrong opponent accepted; zero requirements obeyed where two possible"},
        {"receipt": "evidence/issue17/slice54/native-red-policy.log", "compiles": True, "failed": 1, "exit": 101, "symptom": "real default heuristic wrong-defender refusal at step622"},
        {"receipt": "evidence/issue17/slice54/native-red-whole-field.log", "exit": 1, "symptom": "stale supported Ursine interaction detected before whole-field rewrite"}
    ],
    "journey": journey,
    "default_cli": {k: default[k] for k in ("status", "winner", "steps", "turns", "final_digest", "counts", "replay_digest_matches", "starting_life", "seat_count")},
    "children_inspected": ["t_26a7a4ff", "t_3c2fe51a"],
    "children_are": "scheduled certified-input evaluation wait and unassigned release-soak backlog, not review children; left gated",
    "disclosures": [
        "Original #17 unfinished: 1/8 certified, Ayula3, no new exact100-card certificate.",
        "Native co-authored this increment and is not its independent reviewer.",
        "Initial card construction implementation-first; no strict initial mechanic TDD claim. Native functional repairs have genuine pre-fix compiling assertion REDs.",
        "Initial native compile attempt used nonexistent P2/P3 aliases; retained as FAILED, not behavioral RED.",
        "Inherited single-defender combat model; conflicting requirements maximized within that model, not full multi-defender Commander.",
        "Only six supported card types can appear in supported graveyards; no arbitrary-card support.",
        "Narrow heuristic legality adapter only when no-attack unavailable; no search ranking/budget/sampling/model/provider change.",
        "No new local full-workspace test rerun; current exact-head CI supplies that gate.",
        "Source-left bounce and no-hand-grant tested; no new legal same-object Flash recast test claimed for this non-Flash card.",
        "Original fixture save is sandbox-marked with dummy replay decks; separate real journey uses non-sandbox save/load and full genuine replay.",
        "40-card journeys are not 100-card certificates or strength/GUI evidence; prior pre-repair slice54 recordings retained as superseded, not claimed compatible.",
        "Earlier SLICE54-HUMAN-CHECKPOINT and slice54/RESULT are superseded by FINAL documents; original test and source records retained.",
        "Inherited modified sessions JSONL and unrelated reviewer artifacts excluded from publication."
    ],
    "next_action": "Independent practical delta review of dc9bb3c..published evidence tip, code1b134db; then same writer/history/card/branch/PR46 completes NEXT-CARDS and exact100-card Ayula certification only when both gaps0.",
    "no_merge_master_push_closure_or_unlock": True,
}
out = HERE / "FINAL-RESULT.json"
out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print("FINAL-RESULT written from real completed receipts:", result["focused_aggregate"], result["ci"])
