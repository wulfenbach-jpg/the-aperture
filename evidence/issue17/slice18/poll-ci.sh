#!/usr/bin/env bash
cd /c/AI/aperture-worktrees/issue-17 || exit 1
SHA=a494b78bb0e94b47d856f253c58907f8486e371f
for i in $(seq 1 40); do
  out=$(gh api repos/wulfenbach-jpg/the-aperture/commits/$SHA/check-runs --jq '[.check_runs[]|(.status|tostring)+":"+(.conclusion|tostring)+":"+.name]|join(" | ")')
  gh api repos/wulfenbach-jpg/the-aperture/commits/$SHA/check-runs --jq '{total:.total_count, success:([.check_runs[]|select(.conclusion=="success")]|length), other:([.check_runs[]|select(.conclusion!=null and .conclusion!="success")|{name,conclusion}])}' >> evidence/issue17/slice18/final-ci-poll.jsonl
  running=$(echo "$out" | grep -c "in_progress\|queued")
  echo "$(date -Iseconds) poll $i running=$running $out" >> evidence/issue17/slice18/final-ci-watch.log
  if [ "$running" = "0" ]; then echo "$out" > evidence/issue17/slice18/final-ci-state.txt; break; fi
  sleep 45
done
gh api repos/wulfenbach-jpg/the-aperture/commits/$SHA/check-runs > evidence/issue17/slice18/exact-head-ci.json
