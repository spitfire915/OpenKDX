# Phase 3B: Fork and plumbing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (one fresh-context session for this phase). Steps use checkbox (`- [ ]`) syntax; the steps themselves live in the master plan.

**Goal:** The fork `spitfire915/kdx` exists with CI secrets, a lab runner and an inert `lab.yml`; OpenKDX depends on `kdx-proto/-codec/-client` by pinned revision with the `[patch]` workflow documented and a lockfile guard in CI; `openkdx-table` returns `kdx_proto::ChallengeTable`; the 13 deltas are re-diffed against upstream HEAD into `docs/protocol/convergence.md`.

**Master plan:** `docs/superpowers/plans/2026-11-19-convergence-master.md` — read its header, **Global Constraints**, **Review Focus**, **Type mapping**, **How a session runs**, and the **Ledger** first; then only the task sections named below. Its Global Constraints are this phase's too.

**Tasks (master plan sections):** Task 1 (fork, secrets, runner, `lab.yml`, blocklist `--root`), Task 2 (dependency, MSRV 1.96, `openkdx-table`, `--locked` guard), Task 3 (delta re-diff worksheet).

**Entry condition:** Master plan committed on `main`; GitHub token valid (`git credential fill`); lab reachable (`ssh openkdx-lab`).

**Exit condition:** Fork online with `lab.yml` merged and a skipped lab job; OpenKDX branch `phase3b/convergence` builds with the pin and passes hosted CI incl. `cargo metadata --locked`; `docs/protocol/convergence.md` committed.

**Context to load (and nothing more):** `../kdx/{Cargo.toml,AGENTS.md,.github/workflows/ci.yml}`, `Cargo.toml`, `.github/workflows/ci.yml`, `ci/check-deps.py`, `crates/openkdx-table/src/*`, `lab/README.md`, `lab/setup.sh`, `docs/protocol/README.md`, upstream `docs/protocol/**` for Task 3 only.

**Review:** none within the phase.

**End of session:** append the ledger row in the master plan (phase, date, tasks, OpenKDX commits, fork PRs, upstream PRs, pinned fork SHA, rulings/carried), commit `docs(plan): convergence ledger — Phase 3B`, push.
