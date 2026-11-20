#!/usr/bin/env bash
# Two lab runs that start together: the second waits for the lock instead of
# failing (Review Focus 5). Runs on the lab machine. Each run of
# `differential.sh --lock-only` holds the lock 5 s; both must succeed, and
# the pair must take at least 5 s longer than one alone.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
export LAB_WAIT=1

start=$(date +%s)
"$HERE/differential.sh" --lock-only > /tmp/lock-test-a.out 2>&1 &
a=$!
sleep 0.5
"$HERE/differential.sh" --lock-only > /tmp/lock-test-b.out 2>&1 &
b=$!
wait "$a" || { echo "lock-test: first run failed"; cat /tmp/lock-test-a.out; exit 1; }
wait "$b" || { echo "lock-test: second run failed"; cat /tmp/lock-test-b.out; exit 1; }
elapsed=$(( $(date +%s) - start ))
if (( elapsed < 10 )); then
  echo "lock-test: both runs finished in ${elapsed}s; the second did not wait" >&2
  exit 1
fi
echo "lock-test: ok (both ran, ${elapsed}s in total; the second waited)"
