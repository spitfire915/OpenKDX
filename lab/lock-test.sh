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

# legacy.sh keeps the lock for its whole run, including the scenarios it
# calls: while `legacy.sh --lock-only` sleeps, a fresh flock -n must fail, and
# a scenario started under OPENKDX_LAB_LOCK_HELD=1 must not try to take it.
"$HERE/legacy.sh" --lock-only > /tmp/lock-test-c.out 2>&1 &
c=$!
sleep 1
RUN="${OPENKDX_LAB_RUN:-$HOME/lab/run}"
if flock -n "$RUN/lab.lock" true; then
  echo "lock-test: legacy.sh --lock-only did not hold the lock" >&2; kill "$c"; exit 1
fi
wait "$c" || { echo "lock-test: legacy.sh --lock-only failed"; cat /tmp/lock-test-c.out; exit 1; }
echo "lock-test: ok (legacy.sh holds the lock while it runs)"
