"""Build the recovery handoff from real receipts, not hand-entered totals."""
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
CODE = "f7e1d419368b8fee1ffd46bd43af78d4207305c0"


def main():
    ci = json.loads((OUT / "exact-head-ci-f7e1d41.json").read_text(encoding="utf-8"))
    assert ci["head"] == CODE and ci["all_keyed"] and ci["all_success"], "CI not green at exact code head"
    gates = (OUT / "recovery-gates.log").read_text(encoding="utf-8")
    assert "head=" + CODE in gates
    exits = re.findall(r"EXIT=(\d+) \(([^)]+)\)", gates)
    assert len(exits) == 6 and all(rc == "0" for rc, _ in exits), exits
    results = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored", gates)
    assert len(results) == 8 and all(failed == "0" for _, failed, _ in results)
    focused = sum(int(passed) for passed, _, _ in results)
    journey = json.loads((OUT / "ruxa/demo-journey.json").read_text(encoding="utf-8"))
    journey_log = (OUT / "journey.log").read_text(encoding="utf-8")
    assert "head=" + CODE in journey_log
    assert "EXIT=0 (driver)" in journey_log and "EXIT=0 (cli_replay)" in journey_log
    assert journey["seats"] == 4 and journey["status"] == "terminal" and journey["snapshots_exact"]
    audit = (OUT / "whole-field-audit.log").read_text(encoding="utf-8")
    assert "AUDIT OK" in audit and "EXIT=0" in audit
    subprocess.run(["git", "diff", "--exit-code", "--", "crates", "decks", "fixtures", "README.md", "docs"], cwd=ROOT, check=True)
    old_result = OUT / "RESULT.json"
    if not (OUT / "RESULT-pre-recovery.json").exists():
        (OUT / "RESULT-pre-recovery.json").write_bytes(old_result.read_bytes())
    result = {
        "slice": 53, "issue": 17, "published_pr": "https://github.com/wulfenbach-jpg/the-aperture/pull/46",
        "base": "36eabc34bff449ec43a8cddbceeffb691c226a6e",
        "heads": {"record_corrections": "6d656fb035f0db05b74b551d7de002f9ad497b06", "initial_product": "fa3c7bbdd82c29682c5c276a045c4c24cc49f7e0", "final_code_and_ci": CODE},
        "evidence_commit_identity": "This file belongs to an evidence-only descendant of final_code_and_ci. The actual post-commit published tip is recorded in the native task handoff; no CI pass is claimed for that evidence tip.",
        "card": {"name": "Ruxa, Patient Professor", "printing": "C21 66", "source_record": "fixtures/starter-corpus/slice53-sourced.json", "retrieved_at": "2026-10-28T22:05:00.000000+00:00", "clauses_implemented": 3},
        "record_items_closed": ["R337-1", "R337-2", "R337-3"],
        "recovery_repair": "A battlefield-only trample static grant wrongly reached graveyard creature cards in the shared no-abilities predicate. The new regression compiled and failed before repair (red-graveyard-grant.log, EXIT=101); only battlefield recipients now receive that grant.",
        "post_first_answer_continuation": "two_eligible_attackers_are_asked_in_order_and_damage_is_dealt_once now validates, captures, really loads after answer 1 of 2, and compares second-answer events and full final state. This is after-implementation strengthened coverage, not retroactive TDD.",
        "corpus": {"catalog": 162, "union": 459, "supported": 149, "missing": 310, "per_list": [55,0,51,48,4,52,52,53], "ayula_missing": 4, "certified": 1, "desired": 8, "whole_field_receipt": "whole-field-audit.log"},
        "current_local_gates": {"code_head": CODE, "receipt": "recovery-gates.log", "fmt": 0, "focused_suites": len(results), "focused_passed": focused, "focused_failed": 0, "clippy_workspace_all_targets_deny_warnings": 0, "release_workspace_locked": 0, "journey": 0},
        "retained_pre_recovery_gates": {"code_head": "fa3c7bbdd82c29682c5c276a045c4c24cc49f7e0", "workspace": "workspace-tests.log and workspace-tests-rerun.log each have 134 result lines / 962 passed / 0 failed / 1 ignored / EXIT=0; no doc-test headers. Not claimed re-run locally after the recovery repair.", "search": "gates.log ai_search: actual --test search, 9 passed, EXIT=0; not part of the current focused total", "failed_workspace_link_attempt": "gates.log workspace_tests EXIT=101 is a retained linker failure, not green"},
        "ci": {"head": CODE, "receipt": "exact-head-ci-f7e1d41.json", "total": ci["total"], "all_keyed": True, "all_success": True, "rerun_requested_by_recovery": False},
        "journey": journey,
        "disclosures": [
            "Native wrote the recovery repair and is not its independent reviewer.",
            "Initial card implementation was implementation-first. The predicate/assignment mutation controls at fa3c7bb are post-hoc discrimination only, not TDD. The recovery graveyard-grant repair has its own genuine pre-fix assertion RED.",
            "The first recovery fixture expected a target prompt for a sole legal target; the engine correctly auto-selected it. Two cards now force a real choice. The failed attempt is retained in recovery-fixture-failure.log and recovery-gates-FIRST-FAILED.log.",
            "The 40-card four-seat journey is an evidence-only masked offered-action demonstrator with Jasmine in the command zones, not an exact 100-card starter certificate, product AI or playing-strength proof.",
            "Small AI adapters/heuristic answer handling and native pending buttons were necessary for the new typed choice. No search ranking/budget/provider/model or import/start UI was changed. No new manual native GUI interaction claim.",
            "The focused fixture save/load uses sandbox-marked fixture states and dummy replay decks, not history provenance. The real journey save is non-sandbox and separately replayed.",
            "Other combat assignment remains the disclosed automatic lethal-first simplification; this is one printed exception, not general assignment support.",
            "The no-abilities predicate enumerates currently implemented ability sources; future grants must extend it.",
            "The timed-out run's broad EOL pass damaged two untracked review309 PNG copies. Recovery proved exact transformed-byte equality against the archive's own e6babc6 blobs, restored those exact bytes, and re-read them; restore-archive-images.log is the receipt. Other inherited artifacts/session edits are excluded from publication.",
        ],
        "remaining_ayula_names": ["Dúnedain Rangers", "Evercoat Ursine", "Rampaging Yao Guai", "Ursine Monstrosity"],
        "children_inspected": ["t_26a7a4ff", "t_3c2fe51a"],
        "children_status": "scheduled evaluation wait and unassigned soak backlog; not review children; kept gated",
        "full_issue17_accepted": False, "independent_review_outcome": None,
        "no_merge_master_push_closure_or_unlock": True,
        "next_action": "Concise independent practical review of slice53 only (36eabc3 through published evidence descendant of f7e1d41), then SAME card/writer/history/branch/PR46 for four remaining Ayula cards and exact100-card certification plus separate replays. Do not complete original #17 on increment approval.",
    }
    old_result.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print("Generated RESULT.json:", focused, "focused passed /", len(results), "suites; CI", ci["total"], "success; journey", journey["steps"], "actions")


if __name__ == "__main__":
    main()
