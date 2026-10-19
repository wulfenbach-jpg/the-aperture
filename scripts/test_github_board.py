import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("github_board", Path(__file__).with_name("github-board.py"))
board = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(board)


class BoardPublicationTests(unittest.TestCase):
    def test_bootstrap_retries_delayed_issue_list_visibility(self):
        issue = {"number": 1, "title": "T", "body": "<!-- aperture:roadmap:k -->\n- [ ] proof", "state": "open", "milestone": {"title": "M"}, "labels": [{"name": "status:ready"}]}
        manifest = {"milestones": [{"title": "M", "description": "D"}], "issues": [{"key": "k", "title": "T", "milestone": "M", "area": "engine", "priority": "P0", "status": "ready", "depends_on": [], "acceptance": ["proof"]}]}
        def fake_pages(endpoint):
            return [{"title": "M", "number": 1}] if "/milestones" in endpoint else []
        with patch.object(board, "gh", return_value={"private": False}), patch.object(board, "pages", side_effect=fake_pages), patch.object(board, "all_issues", side_effect=[[issue], [], [issue]]), patch.object(board, "snapshot", return_value={"issues": [{"number": 1}]}):
            result = board.bootstrap("owner/repo", manifest)
        self.assertEqual(result["verified_roadmap_issues"], 1)
        self.assertEqual(result["created_keys"], [])

    def test_private_repository_cannot_become_a_public_snapshot(self):
        with patch.object(board, "gh", return_value={"private": True}):
            with self.assertRaisesRegex(RuntimeError, "private repository"):
                board.snapshot("owner/private")

    def test_status_change_preserves_non_status_labels_and_verifies_target(self):
        before = {"state": "open", "labels": [{"name": "area:engine"}, {"name": "priority:P0"}, {"name": "status:ready"}]}
        after = {"state": "open", "labels": [{"name": "area:engine"}, {"name": "priority:P0"}, {"name": "status:in-progress"}]}
        with patch.object(board, "gh", side_effect=[before, {}, after]) as remote:
            self.assertTrue(board.change_status("owner/repo", 6, "in-progress")["verified"])
        payload = remote.call_args_list[1].kwargs["payload"]
        self.assertEqual(set(payload["labels"]), {"area:engine", "priority:P0", "status:in-progress"})
        self.assertEqual(remote.call_args_list[2].args, ("api", "repos/owner/repo/issues/6"))

    def test_unverified_status_write_fails_closed(self):
        before = {"state": "open", "labels": [{"name": "status:ready"}]}
        with patch.object(board, "gh", side_effect=[before, {}, before]):
            with self.assertRaisesRegex(RuntimeError, "read-back mismatch"):
                board.change_status("owner/repo", 6, "review")

    def test_closed_issues_and_pull_requests_cannot_be_moved(self):
        for issue in [{"state": "closed"}, {"state": "open", "pull_request": {}}]:
            with self.subTest(issue=issue), patch.object(board, "gh", return_value=issue):
                with self.assertRaisesRegex(RuntimeError, "open issue"):
                    board.change_status("owner/repo", 6, "ready")



if __name__ == "__main__":
    unittest.main()
