# Phase 3C: Upstream the evidence — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (one fresh-context session for this phase). Steps use checkbox (`- [ ]`) syntax; the steps themselves live in the master plan.

**Goal:** Our 15 captures and the 13 protocol deltas live in the fork: `kdx-dissect` audits the captures byte-exact, `kdx-proto` and `docs/protocol` carry the facts with evidence tags, `ServerSettings` is typed, and `kdx-dissect --inventory` feeds OpenKDX's scenario notes.

**Master plan:** `docs/superpowers/plans/2026-11-19-convergence-master.md` — read its header, **Global Constraints**, **Review Focus**, **Type mapping**, **How a session runs**, and the **Ledger** first; then only the task sections named below. Its Global Constraints are this phase's too.

**Tasks (master plan sections):** Task 4 (captures byte-exact under `kdx-dissect`), Task 5 (protocol facts: `/me`, proof status 1, padding phase, `.prt`, 171 byte 0x52, 511 editor, Status replies), Task 6 (typed `ServerSettings`), Task 7 (`--inventory`, scenario notes).

**Entry condition:** Phase 3B done: fork exists with secrets, runner and `lab.yml`; OpenKDX pins the fork; `docs/protocol/convergence.md` lists each delta's action.

**Exit condition:** Fork `main` holds the four `feat/*` merges with the lab job skipped-or-green; four `up/*` PRs open upstream; `cargo test` in the fork green; OpenKDX scenario inventories regenerated.

**Context to load (and nothing more):** `../kdx/crates/kdx-proto/src/msg/{chat,server_info,admin,status}.rs`, `../kdx/crates/kdx-dissect/{src/main.rs,tests/captures.rs}`, `../kdx/AGENTS.md`, `docs/protocol/convergence.md`, our `captures/*.md` as needed, `crates/openkdx-proto/src/msg/admin.rs:450-1100` (the typed settings to port). Do not load `openkdx-client` or `openkdx-legacy`.

**Review:** none within the phase; the fork PRs' CI is the gate.

**End of session:** append the ledger row in the master plan (phase, date, tasks, OpenKDX commits, fork PRs, upstream PRs, pinned fork SHA, rulings/carried), commit `docs(plan): convergence ledger — Phase 3C`, push.
