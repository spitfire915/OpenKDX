# Phase 3E: Legacy, CLI, deletion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (one fresh-context session for this phase). Steps use checkbox (`- [ ]`) syntax; the steps themselves live in the master plan.

**Goal:** `kdx-legacy` in the fork writes the 50 fixtures byte for byte; `openkdx-cli` runs on `kdx-client`; `openkdx-proto/-codec/-dissect/-client/-tracker/-server/-legacy` are deleted; docs and CI describe the two-repository workflow; the OpenKDX PR is reviewed and merged.

**Master plan:** `docs/superpowers/plans/2026-11-19-convergence-master.md` — read its header, **Global Constraints**, **Review Focus**, **Type mapping**, **How a session runs**, and the **Ledger** first; then only the task sections named below. Its Global Constraints are this phase's too.

**Tasks (master plan sections):** Task 12 (`kdx-legacy`), Task 13 (`openkdx-cli` on `kdx-client`), Task 14 (delete, docs, pin, review gate, PR, merge).

**Entry condition:** Phase 3D merged in the fork and OpenKDX pinned to it; `lab/legacy.sh` already accepts the kdx checkout.

**Exit condition:** OpenKDX `main` builds only `openkdx-table/-cli/-ui/-app/openkdxd` over the pinned `kdx-*`; all CI green on both repositories; PR #6 merged.

**Context to load (and nothing more):** `crates/openkdx-legacy/**` (port source), `../kdx/crates/kdx-proto/src/msg/{admin,news,server_info,tracker}.rs` (target structs), `crates/openkdx-cli/**`, `CONTRIBUTING.md`, `README.md`, `lab/README.md`, `captures/README.md`, `docs/protocol/README.md`, `ci/check-deps.py`. Load `kdx-client`'s public API by `cargo doc` output, not its source.

**Review:** Task 14 Step 4: fresh-context reviewer on **Opus 5.5** (`github-copilot/claude-opus-5.5`) over `main..phase3b/convergence` and the fork's `main` since the fork point, with the master plan, spec §6.1/§8/§9 and both CONTRIBUTING files in the prompt.

**End of session:** append the ledger row in the master plan (phase, date, tasks, OpenKDX commits, fork PRs, upstream PRs, pinned fork SHA, rulings/carried), commit `docs(plan): convergence ledger — Phase 3E`, push.
