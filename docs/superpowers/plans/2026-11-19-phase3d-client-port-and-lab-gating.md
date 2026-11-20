# Phase 3D: Client port and lab gating — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (one fresh-context session for this phase). Steps use checkbox (`- [ ]`) syntax; the steps themselves live in the master plan.

**Goal:** `kdx-client` has every feature of `openkdx-client` (mock, typed `Notice`, files/chat/news/accounts/folder access/admin/transfers) and the 16-test differential suite; the lab gates both repositories through two runners and `lab/differential.sh <kdx-src>`.

**Master plan:** `docs/superpowers/plans/2026-11-19-convergence-master.md` — read its header, **Global Constraints**, **Review Focus**, **Type mapping**, **How a session runs**, and the **Ledger** first; then only the task sections named below. Its Global Constraints are this phase's too.

**Tasks (master plan sections):** Task 8 (`mock` feature + `Notice`), Task 9 (files, chat, news, accounts/classes, folder access), Task 10 (admin + transfers), Task 11 (differential suite in the fork; `lab/differential.sh`/`legacy.sh` take the kdx checkout; `LAB_WAIT`; `lab/kdx-rev.sh`; `lab.yml` live).

**Entry condition:** Phase 3C merged in the fork; OpenKDX pin bumped to it.

**Exit condition:** `cargo test -p kdx-client --features mock` green; `lab/differential.sh ~/kdx-src` passes 16/16 on the lab; both repositories' lab jobs green; OpenKDX pin bumped.

**Context to load (and nothing more):** `../kdx/crates/kdx-client/src/*`, `../kdx/crates/kdx-codec/src/lib.rs`, `crates/openkdx-client/src/{chat,news,files,accounts,admin,transfer}.rs` and `tests/{common/mock.rs,lab.rs,…}` one area at a time, `lab/differential.sh`, `lab/legacy.sh`, `.github/workflows/ci.yml`, the fork's `lab.yml`. Do not load `openkdx-proto` beyond the mapping table.

**Review:** none within the phase; Review Focus items 3–5 are pinned by the named tests.

**End of session:** append the ledger row in the master plan (phase, date, tasks, OpenKDX commits, fork PRs, upstream PRs, pinned fork SHA, rulings/carried), commit `docs(plan): convergence ledger — Phase 3D`, push.
