#!/usr/bin/env bash
# Poll exact-head CI for slice 41 with a bounded number of capture attempts.
set -u
cd "$(dirname "$0")/../../.." || exit 1
sha=ddd99f0
log=evidence/issue17/slice41/exact-head-ci-$sha.log
for attempt in $(seq 1 20); do
  python evidence/issue17/slice35/capture-ci.py "$sha" >> "$log" 2>&1
  status=$?
  echo "attempt=$attempt capture_exit=$status" >> "$log"
  if [ "$status" -eq 0 ]; then
    echo "CI_GREEN_AT_ATTEMPT=$attempt" >> "$log"
    exit 0
  fi
  sleep 45
done
echo "CI_NOT_GREEN_AFTER=20" >> "$log"
exit 1
