# Phase 3H: Evidence rebase — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This plan continues `2026-11-23-phase3g-review-rework.md` and `2026-11-19-convergence-master.md` (Global Constraints, type mapping, ledgers). **Do not execute until the user has reviewed it.**

**Goal:** Our upstream PRs to scottanderson/kdx rest on *his* captures and rules: #89 becomes a docs-only PR that cites frames already on `main`; the three details his captures lack (`/me` action flag, a ban, an upload resume) and one trimmed frame (the server's startup registration) each arrive as their own small PR with a capture recorded by the native Client 1.620 beta 5 under his lab tool; #90–#96 are unstacked so he can review them in parallel; fork `main` mirrors what upstream will hold.

**Architecture:** Fork first, as before. The fork drops the 15 `win-*`/`cli-*` captures with their fixtures, tests and inventories and re-points the kept facts at his frames; OpenKDX reverts the fork-copy naming and bumps the pin. Upstream branches are re-cut on `upstream/main` (#100–#105 in): `#89 → up/protocol-facts` (docs only), `#90`, `#91` independent, `#92–#96` a chain among themselves only. The recaptures run on the lab box with **his** `lab/kdx-lab` from the kdx checkout (`~/kdx-src`), the native Linux client driven by a new xdotool helper in OpenKDX (`lab/lib/ui-linux.sh`), under the lab lock; each capture gets a #101 header, `kdx-dissect --export` fixtures, an entry in `crates/kdx-dissect/tests/captures.rs`, and the doc rows it pins.

**Tech Stack:** as the master plan, plus upstream's lab (`lab/kdx-lab up|server|client|run|down|reset`, `vendor/fetch`, `/tmp/.X11-unix/X0` → Xvfb on the lab box), `kdx` and `kdx-probe` (`crates/kdx-cli/src/bin/`), `kdx-dissect --export`, `editcap`, `tc`, `ip`. Shared passwords: `lab-only-pw!1` (admin), empty (guest), `lab-peer-pw!1`, `lab-tracker-pw!1`, `lab-tracker-pw!2`.

**Spec:** `docs/superpowers/specs/2026-11-08-openkdx-design.md` §5.3, §6.1 (upstream every change), §7.5 (the lab, captures with evidence). Upstream rules at `upstream/main` 4be3391: `AGENTS.md`, `CONTRIBUTING.md`, `captures/README.md` (ground truth, header, Passwords), `lab/README.md`. The review itself: scottanderson/kdx#89 general comment of 2026-11-25 and its per-note inline comments (fetch with `GET /repos/scottanderson/kdx/pulls/89/comments`), plus the still-standing line comments on #89 and #90 (`.superpowers/sdd/…/pr89-comments.md`, `pr90-comments.md` were deleted with the 3G workspace: refetch).

## Global Constraints

- Master plan constraints and Phase 3G's: trailer names the model that wrote the commit; no external project name in the fork outside `.github/workflows/lab.yml`, `tests/lab.rs`'s header and kdx-client's README lab sentence (and captured data); check-docs, codespell, clippy 1.96, fmt, gitlint clean.
- **Captures are ground truth; tool traffic is synthetic (#100).** A fact tagged `[O capture frames]` is shown by a *Haxial program's* frames. What `kdx`/`kdx-probe`/our CLI sends is not evidence; the server's reply to it is. Every tool command appears in full in the note (`lab/kdx-lab run target/debug/kdx …`), built from the tree of the same PR.
- **Note header (#101)**: `lab/kdx-lab up <scenario>` writes it; fields in order Date (UTC minutes first→last frame), Server/Tracker/Client (name as in `vendor/MANIFEST.md` + full sha256), Tools, Server state (each login and password), Decode (the `kdx-dissect` command with every password). `ci/check-docs.py` checks it.
- **Passwords (#103)**: only the shared ones; a new one takes the next number and goes into `captures/README.md` Passwords, `crates/kdx-dissect/tests/captures.rs`'s list, and every note/test that decodes it.
- **Default client is Client 1.620 beta 5 for Linux (#104)**; Wine only for Client 1.600 facts.
- **One PR per capture**, each with note, fixtures, `captures.rs` entry and the doc rows it pins; PR bodies are the squash message: prose, no hard wrap, no trailer, `spitfire915/kdx#N` for fork PRs.
- **No Messages inventories in capture PRs**; they come with `--inventory` in #91.
- **Unstacked**: #90 and #91 on `upstream/main` alone; #92–#96 a chain on `upstream/main`; nothing depends on #89.
- **The lab lock** (`~/lab/run/lab.lock`, `flock`) is held for every recapture session; `LAB_WAIT=1` waits behind CI. Never `pkill` a CI script. His namespace (`kdx-lab up`) and ours (`lab/kdx-lab`) must not run at once: his `up` creates its own netns; the lock is the arbiter.
- **Nothing of Haxial's leaves the lab** (`python3 ci/check-blocklist.py --root ../kdx` before each fork push).
- **The user posts upstream** (titles, bodies, replies, retitle of #89); we force-push `spitfire915:up/*` only.

## Review Focus

1. **A fact re-tagged onto his frames must actually be shown there.** Pinned by each docs commit's "verified" line in the body (`kdx-dissect <capture> --inventory` frames read) and by the Task 3 reviewer decoding each cited frame.
2. **The recapture scripts must drive the *client*, not the server, and leave the server's replies intact** — a scripted client that sends a malformed request produces evidence of nothing. Pinned by each note's steps naming the client action per frame and by `kdx-dissect --audit` showing no mismatch.
3. **The `/me` width**: a real client's 207 settles u8@0x09 vs u16@0x08. Pinned by `linux-chat-action` frames and `ChatText` test `action_flag_from_a_real_client` (Task 5).
4. **Ban semantics**: `ban_seconds` non-zero in 150, flag 2 in 151, who 151 names, the refusal from the banned address, loopback exemption. Pinned by `linux-chat-ban` and `kdx-probe` from both addresses (Task 5).
5. **Resume arithmetic from a real client**: 416's offset equals the `.prt` size listed by 401 after the cancel; the resumed file compares equal (`cmp`). Pinned by `linux-files-resume` and the fixture test `prt_size_is_the_resume_offset` re-pointed at it (Task 5).

---

## Rulings written into this plan (from the planning discussion of 2026-11-25)

- **R1 — #89 is retitled, not closed:** it becomes the docs-only PR (`up/protocol-facts`), keeping his review thread; his inline comments describe exactly its content. Cost if wrong: he prefers a fresh PR — one retitle.
- **R2 — `ChatText::flags` moves to #92's branch** (Notice needs `is_action`), tagged `[I]` against his plain-line frames (`linux-chat-admin` 71, 74) until `linux-chat-action` lands and re-tags it `[O]`. #92 does not wait on lab work. Cost: one follow-up commit on the chat-action PR.
- **R3 — fork `main` mirrors upstream content:** the 15 fork captures, notes, fixtures, tests and inventories are deleted in the fork; OpenKDX keeps its own `captures/` and lab. Cost: the fork loses files nobody downstream cites (OpenKDX notes stop naming fork copies in Task 1).
- **R4 — the Linux-client helper lives in OpenKDX (`lab/lib/ui-linux.sh`)**, our lab, our runner; his `lab/kdx-lab` is invoked as is from `~/kdx-src`, never wrapped or patched, because the note must show his tool's commands. Cost: two lab tools on one box, serialised by our lock.
- **R5 — the GUI is scripted with xdotool** (user's choice), with a "connect + one chat line works" exit gate before any recapture; a step that resists scripting after a bounded effort (Task 4 budget) falls back to the user over VNC for that step only, recorded as a ruling.
- **R6 — order:** docs PR + unstacking first (Tasks 1–3), lab second (Tasks 4–6).
- **R7 — our 15 OpenKDX captures stay OpenKDX's record**; nothing in this plan edits them beyond the Decoded-by lines.
- **R8 — `win-tracker-basic` frame 1 is trimmed with `editcap -r … 1` into `linux-tracker-startup`** as he asked, even though the client in that capture was the Wine client (frame 1 is server→tracker; the client never appears in it), with the #101 header written by hand in `kdx-lab up`'s form.

---

## File structure

Fork (`../kdx`):
- Delete: `captures/{win,cli}-*.{pcapng,md}`, `crates/kdx-proto/tests/{win,cli}_*.rs`, `crates/kdx-proto/tests/fixtures/{win,cli}-*`, their rows in `captures/README.md`, the `cli-`/`win-` key rule in `crates/kdx-codec/tests/fixtures.rs`, their names in `crates/kdx-dissect/tests/{captures,inventory}.rs`
- Modify: `docs/protocol/{handshake,sequences}.md`, `docs/protocol/messages/{accounts,administration,chat,files,folder-access,news}.md` (tags onto his frames), `crates/kdx-proto/tests/common/mod.rs` (`LAB_PASSWORD` goes if unused), `crates/kdx-proto/src/msg/chat.rs` (flags stay; tag `[I]`), new `crates/kdx-proto/tests/server_settings_rename.rs` over `linux-server-settings` (frames 41, 80) replacing `cli_server_settings.rs`'s capture-bound tests (the synthetic-blob tests move to a `server_settings_layout.rs`)
- Create (Tasks 5–6): `captures/linux-chat-action.{pcapng,md}`, `captures/linux-chat-ban.{pcapng,md}`, `captures/linux-files-resume.{pcapng,md}`, `captures/linux-tracker-startup.{pcapng,md}`; fixtures `crates/kdx-proto/tests/fixtures/linux-chat-action-<port>.{c2s,s2c}.bin` etc. via `kdx-dissect --export`; tests `crates/kdx-proto/tests/linux_chat_action.rs`, `linux_chat_ban.rs`, `linux_files_resume.rs`, `linux_tracker_startup.rs`; entries in `crates/kdx-dissect/tests/captures.rs`

OpenKDX:
- Modify: `captures/*.md` (Decoded-by lines name the OpenKDX capture only), `lab/lib/scenario.sh` (remove `FORK_CAPTURE_PREFIX`), `CONTRIBUTING.md` ("What goes where": the fork's captures are his lab's; ours stay here), `Cargo.toml`/`Cargo.lock` (pin), `docs/protocol/README.md` (Upstream column: the new PR numbers)
- Create: `lab/lib/ui-linux.sh` (xdotool helper for Client 1.620b5), `lab/recapture/{chat-action,chat-ban,files-resume}.sh` (drive his lab tool + our helper, under the lock), `lab/recapture/README.md`
- This plan's `## Ledger`

---

### Task 1: Fork mirrors upstream; OpenKDX stops naming fork copies

**Files:** fork branch `feat/evidence-rebase` from `main`; OpenKDX branch `phase3h/pin` (PR #9).

**Interfaces:**
- Produces: fork `main` SHA M1 (Tasks 2–3 cut from `upstream/main`, but M1 is the tree the topic commits are taken from); the re-tag table (Step 2) that Task 3 reuses verbatim.

- [ ] **Step 1 — Merge upstream.** `git merge --no-ff upstream/main` (4be3391: #100–#105). Resolve conflicts keeping both sides; run `python3 -I ci/check-docs.py` — expect failures on every `captures/{win,cli}-*.md` (#101 header) and possibly `[O]` tags; they vanish in Step 3. Commit the merge; push `main` (as 3G Task 1 did).
- [ ] **Step 2 — Re-tag the kept facts onto his frames** (one commit per doc file; each body ends with `Verified: kdx-dissect captures/<name>.pcapng --password … --inventory, frames <list>` after the implementer has read those frames):

| Fact | Old tag | New evidence (his inline comments) |
|---|---|---|
| 510 `editor` is the last editor; 511 echo asks a rename | `cli-accounts-classes` | `kdx-accounts` 411, 412, 431 (`[O]` for editor; the 0x1000d rename stays `[I]`) |
| 401 `kind` empty for `.txt`, 410 says `text/plain` | `win-files-basic` | `linux-files` 56, 82 (empty), 87 (410 `text/plain`), 93 (listed with type after rename) — rewrite the cell: the type comes from the file, not a table (`[I]` on any "table" claim) |
| Padding phase of the payload word | `win-first-connect` | drop: `reencode_byte_for_byte` in `kdx-codec/tests/fixtures.rs` over `first-connect.pcapng` already pins it; the doc keeps the sentence with `[O first-connect …]` only if main's tag exists, else no tag |
| 414 data padding | `win-files-basic 30` | drop: main cites `linux-files` 69, 113 |
| Proof status 1 (wrong password) | `cli-disconnect-ban 9` | `linux-first-connect` 13 |
| Disconnect without ban (150/151) | `cli-disconnect-ban` | `linux-chat-admin` 159, 161; the ban facts wait for Task 5 |
| 154 broadcast: no reply; 155 to sender and others | `cli-broadcast` | `linux-chat-admin` 129 (154), 130, 132 (155) |
| 143 transfer record (transfer_id ≠ 0, path) | `cli-connection-monitor` | `linux-catalog-volumes` from 104 |
| 0x44c status (folder access) | `cli-folder-access` | `linux-files` 234; the unpadded-record 0x1000a claim becomes `[I]` or goes |
| 180–183 history records, refused request record | `cli-server-history` | `linux-server-settings` (94: refused 400), `linux-chat-admin` |
| 172 rename; 105 shows the new name; 0x52 length byte | `cli-server-settings` 17/20/23/38 | `linux-server-settings` 41 (173), 80 (105) — note: the "`OpenKDX La` truncation" fact has no capture on main: `[I]` or drop |
| 600/604 answered with a success Status | `[O lab legacy-samples 33, 36]` | no capture on main shows it: `[I]` ("the lab's server answered a success Status; a capture is wanted") — or record it in Task 5's ban session with `kdx` (`kdx` creates a newsgroup as admin → the server's 1 Status is evidence) |
| 1100 length check 0x1000a | lab test | `[I]` unless a frame shows 0x1000a |
| `/me` action flag | `win-chat-basic` 24, 25 | `[I]` with `linux-chat-admin` 71, 74 (plain line) until Task 5 (R2) |
| 703 empty group → 0-byte payload | lab | already in the reference; no tag needed |
| Resume offset = `.prt` size | `cli-upload-resume` 61, 88 | wait for Task 5; meanwhile `[I]`, text as the review asked |

- [ ] **Step 3 — Delete** the 15 captures, notes, README rows, fixtures, tests, the `LAB_PASSWORD` const if unused, the `cli-`/`win-` key rule, their names in kdx-dissect tests; replace `cli_server_settings.rs` by `server_settings_layout.rs` (synthetic-blob tests: any order, unpadded end, 0x54, allow u16, overlong strings, twelve lengths, gaps, trailing pad, tracker count, unpadded last record) and `server_settings_rename.rs` (`linux-server-settings` frames 41, 80: the 172's byte 0x52 equals the later 105's name length; exported fixture for that connection, `kdx-dissect captures/linux-server-settings.pcapng --export …` — check which connection carries 172 and 105 and whether a fixture for it already exists on main). `administration.md`'s `name_len` row cites 41/80. Commit per topic. Verify: `cargo test -p kdx-proto -p kdx-codec -p kdx-dissect -p kdx-legacy -p kdx-client` ok; check-docs, codespell, clippy 1.96, fmt, gitlint, blocklist clean; `git grep -il openkdx` → only the allowed files and data.
- [ ] **Step 4 — Fork PR** "Rest the protocol facts on the captures already here" → CI (lab) → squash-merge → M1.
- [ ] **Step 5 — OpenKDX `phase3h/pin`:** fifteen `captures/*.md` Decoded-by lines name only `captures/<scenario>.pcapng` here (no fork copy); `lab/lib/scenario.sh` loses `FORK_CAPTURE_PREFIX` and writes that line; `CONTRIBUTING.md` "What goes where": our captures are OpenKDX's record; what goes upstream is recorded with his lab (`lab/kdx-lab`) by the native client and lives only there; pin `rev = M1`, `cargo update -p kdx-*`, `cargo metadata --locked`, tests, check-deps. Commits `docs(captures): notes name only this repository's captures`, `build: pin kdx-* at <M1_7>`. PR #9, CI (lab), squash-merge, lab box `git pull`.
- [ ] **Step 6 — Ledger row 3H-1.**

---

### Task 2: Unstack #90–#96 on `upstream/main`

**Files:** fork branches `up/typed-server-settings`, `up/dissect-inventory`, `up/client-mock-notice`, `up/client-features`, `up/client-admin-transfers`, `up/lab-suite`, `up/kdx-legacy` (names kept).

**Interfaces:**
- Consumes: M1's tree (Task 1).
- Produces: new tips T90…T96; the ownership table (per branch, the files it carries) reused by Task 6.

- [ ] **Step 1 — Ownership.** From Phase 3G's Task 5 report method (`git diff --name-only <parent> <tip>` of the current branches, then content from M1), minus everything Task 1 deleted. `up/typed-server-settings` now carries `server_info.rs`, `administration.md`, `server_settings_layout.rs`, `server_settings_rename.rs` (+ its fixture if new), `ChatRecord::decode_with_len` — and nothing of #89. `up/client-mock-notice` carries `chat.rs`'s `flags`/`FLAG_ACTION`/`is_action` with the `chat.md` row tagged `[I]` (R2). `up/dissect-inventory` carries `--inventory`, its test, `docs/dissect`, and **the inventories of his existing notes only if #91 already did that** (it did not: the inventories were of our notes — so #91 carries the feature, test and doc; no note changes).
- [ ] **Step 2 — Cut:** #90 and #91 from `upstream/main`; #92 from `upstream/main`; #93 from T92; #94 from T93; #95 from T94; #96 from T95. Topic commits as in 3G (3–8 per branch, Opus-trailered by whoever writes them, bodies with reasons and "Public API: …", 80 columns). kdx-legacy's `server_stg.rs` depends on #90's types: #96 therefore depends on #90 too — **ruling needed at execution**: either #96 is cut on T90+T95 (a merge base of two chains, which GitHub shows as "based on #95 and #90"), or #96 carries a copy of #90's `server_info.rs` change (no: duplicate). Record: cut #96 from T95 with T90 merged in (`git merge --no-ff T90` as the first commit of #96's branch is ugly; prefer: cut #92–#96 from **T90**, since `set_server_name` in #94 needs typed settings anyway — then only #90 is a shared base, which is what he can review first). Decide, ledger it.
- [ ] **Step 3 — Verify per branch** (gitlint over `upstream/main..HEAD`, check-docs, codespell, clippy 1.96, fmt, `cargo test --workspace`, blocklist); `--force-with-lease`. Tree check: merging all tips reproduces M1 minus nothing (`git diff --stat <octopus of tips> main` empty).
- [ ] **Step 4 — Texts** for the user: each PR's body rewritten (prose, no hard wrap, `spitfire915/kdx#N`, "based on main" / "based on #90"); a reply on #90 that lists his comments and the commits, noting the capture-bound test now cites `linux-server-settings` 41/80.
- [ ] **Step 5 — Ledger row 3H-2.**

---

### Task 3: #89 → `up/protocol-facts` (docs only)

**Files:** fork branch `up/openkdx-captures` (name kept; PR retitled "Record what the captures on main already show") from `upstream/main`.

- [ ] **Step 1 — Cut** from `upstream/main` the Task 1 Step 2 doc commits only (no code, no captures, no fixtures): one commit per doc file, bodies carrying the "Verified: … frames …" line. Nothing of `chat.rs` (R2).
- [ ] **Step 2 — Verify:** check-docs, codespell, gitlint; `git diff --stat upstream/main HEAD` touches only `docs/protocol/**`.
- [ ] **Step 3 — Force-push; texts:** new title and body (no hard wrap; says the captures were withdrawn and which PRs will bring the three new ones and the startup frame); a reply that walks his inline comments: for each withdrawn capture "cited <his frames> instead", for the three recaptures "coming as <PR>", for the inventories "with #91".
- [ ] **Step 4 — Ledger row 3H-3.** *Tasks 1–3 end the docs half; the user may pause here.*

---

### Task 4: His lab tool on our box; the Linux-client helper

**Files:** OpenKDX `lab/lib/ui-linux.sh` (new), `lab/recapture/README.md` (new), `lab/README.md` (one paragraph), lab box state.

**Interfaces:**
- Produces: `ui_lx_start`, `ui_lx_connect ADDR LOGIN PASSWORD`, `ui_lx_chat TEXT`, `ui_lx_upload FILE FOLDER`, `ui_lx_cancel_transfer`, `ui_lx_ban_user NICK`, `ui_lx_quit`, `ui_lx_shot NAME`; `recapture_begin SCENARIO` / `recapture_end` (lock + `lab/kdx-lab up/down` + Xvfb :0 + `tc`/`ip` cleanup).

- [ ] **Step 1 — Prerequisites on the lab box** (as `openkdx`, under the lock, `LAB_WAIT=1`): `cd ~/kdx-src && git checkout M1 && cargo build -p kdx-cli` (gives `target/debug/kdx`, `kdx-probe`), `vendor/fetch` (binaries already under `~/haxial/bin`; symlink or let it fetch — it verifies sha256; nothing new leaves the box), `lab/kdx-lab setup` (i386 libstdc++.so.5), `Xvfb :0 -screen 0 1280x1024x24` so `/tmp/.X11-unix/X0` exists (his tool binds exactly that socket). Verify: `lab/kdx-lab up probe-smoke && lab/kdx-lab server -d && lab/kdx-lab run target/debug/kdx-probe --addr 127.0.0.1:10700 && lab/kdx-lab down`. Expected: a pcapng in `~/kdx-src/captures/probe-smoke.pcapng` (delete it; never commit).
- [ ] **Step 2 — Window map.** `lab/kdx-lab client` under `DISPLAY=:0`; `xdotool search --name '' ` + `xwininfo -root -tree` to list the client's windows (connect dialog, main window, Public Chat, Files, Users, transfer window); `lab/kdx-lab x shot`-equivalent (`xwd`/`import`) per state; write the map into `lab/recapture/README.md` (titles, child geometry, which keys the dialogs accept). Budget: this step and Step 3 together one session; if the connect dialog cannot be driven after that, R5's fallback.
- [ ] **Step 3 — `ui-linux.sh` exit gate:** `ui_lx_connect 127.0.0.1 admin lab-only-pw!1` then `ui_lx_chat 'hello'` against a server started with `lab/kdx-lab server` — the server log shows the login and `kdx listen` (run via `lab/kdx-lab run`) prints the chat line. `bash -n`; the smoke capture is deleted. Commit `lab: drive Client 1.620 beta 5 with xdotool (ui-linux.sh)`.
- [ ] **Step 4 — Ledger row 3H-4** (window map facts, what resisted).

---

### Task 5: The three recaptures

**Files:** OpenKDX `lab/recapture/{chat-action,chat-ban,files-resume}.sh`; fork branch `feat/linux-captures` → three fork PRs? **No:** one fork PR per capture mirrors upstream's one-PR-per-capture (kdx#N+1..N+3), each: `captures/<name>.{pcapng,md}`, fixtures via `kdx-dissect --export`, `tests/<name>.rs`, `captures.rs` entry, the doc rows.

Each scenario follows his inline recipe verbatim (the note's steps quote the commands in full):

- [ ] **5a `linux-chat-action`:** `lab/kdx-lab reset server; lab/kdx-lab up linux-chat-action; lab/kdx-lab server` (setup: Enter, `admin`, `lab-only-pw!1` ×2, `yes`); `lab/kdx-lab run target/debug/kdx listen --nick listener`; client: connect as admin, Public Chat, `/me waves`, Return; quit; Ctrl-C listener; `down`. Facts: 207 from a real client with the action marker — settle width (RF3); 208 to the listener. Test `linux_chat_action.rs::action_flag_from_a_real_client` over the exported fixture; `chat.md` row re-tagged `[O linux-chat-action N, M]` on this branch and a follow-up to #92 (R2).
- [ ] **5b `linux-chat-ban`:** `up linux-chat-ban`; `lab/kdx-lab run ip addr add 10.9.0.1/32 dev lo`; client 1 (admin → 127.0.0.1), client 2 (`client linux2`, guest → 10.9.0.1); admin disconnects the guest with ban; client 2 reconnects to 10.9.0.1 (refused — how?); `kdx-probe --addr 10.9.0.1:10700` (refused) and `--addr 127.0.0.1:10700` (gets in); quit; `down`. Facts: 150 `ban_seconds`, 151 flag 2 and whom it names, the refusal form, loopback exemption (RF4). Test `linux_chat_ban.rs`; `administration.md` 150/151 rows and `sequences.md` ban paragraph re-tagged. Optional in the same session, if cheap: `kdx` as admin creates a newsgroup → the 1 Status for 600 (Task 1's `[I]`) — only if the note stays readable; else its own capture later.
- [ ] **5c `linux-files-resume`:** `mkdir -p lab/run/client-linux && head -c 2000000 /dev/urandom > lab/run/client-linux/resume.bin`; `up linux-files-resume`; server; `lab/kdx-lab run ip link set lo mtu 1500; lab/kdx-lab run tc qdisc add dev lo root tbf rate 800kbit burst 16kb latency 2s`; client: connect, upload `resume.bin` into `Uploads [UL]`, cancel partway (`ui_lx_cancel_transfer` after ~8 s); `lab/kdx-lab run target/debug/kdx ls 'Uploads [UL]' --login admin --password 'lab-only-pw!1'` (401 lists `.prt`); client uploads again → resume; `kdx get 'Uploads [UL]/resume.bin' -o resume-check.bin …; cmp resume-check.bin lab/run/client-linux/resume.bin`; quit; `down`. Facts: 415 from a real client, 416 offset = `.prt` size (RF5), 417 header form. Test `linux_files_resume.rs::prt_size_is_the_resume_offset`; `files.md` 415/416 rows re-tagged.
- [ ] For each: `kdx-dissect captures/<name>.pcapng --password 'lab-only-pw!1' --audit` → no mismatch; `--export` fixtures; `captures.rs` expected problems (likely none); note header as `up` wrote it, Date from `capinfos -a -e`, sha256s from `vendor/MANIFEST.md`; steps with frames, ports, user ids; `python3 ci/check-blocklist.py --root ../kdx`; fork PR, CI (lab), squash-merge; pin bump at the end of the three (one OpenKDX PR #10).
- [ ] **Ledger row 3H-5** per capture (what the frames settled: width, ban fields, offset).

---

### Task 6: `linux-tracker-startup`, the four upstream capture PRs, hand-off

- [ ] **Step 1 — Trim:** from OpenKDX's own `captures/tracker-basic.pcapng` (same bytes as the deleted fork copy): `editcap -r captures/tracker-basic.pcapng ../kdx/captures/linux-tracker-startup.pcapng 1`; note with the #101 header by hand (Server 1.620 sha256, Tracker 1.110 sha256, Client `none`, Server state: fresh with admin / `openkdx-lab` — **the password is ours, not shared; frame 1 needs no password** so Decode has none; say so), one step, the fact (registration at startup carries no password). `captures.rs` entry; fixture only if the UDP frame exports; `tracker.md`/`udp.md` row. Fork PR → merge.
- [ ] **Step 2 — Upstream PRs:** four branches from `upstream/main`: `up/linux-chat-action`, `up/linux-chat-ban`, `up/linux-files-resume`, `up/linux-tracker-startup`, each the topic commits of its fork PR; verify (gitlint, check-docs — the header check, codespell, tests incl. the new fixture tests, blocklist); push; texts (title, prose body with the recipe summarised and the facts, "replaces part of #89").
- [ ] **Step 3 — #92 follow-up:** `chat.md` action row → `[O linux-chat-action N, M]`, the `[I]` sentence removed; one commit appended to `up/client-mock-notice` (and the chain above it rebased, `--force-with-lease`). Or, if the chat-action PR merges first, the commit goes there and #92 rebases onto `upstream/main`.
- [ ] **Step 4 — Final ledger row 3H-6**; master plan upstream table updated (#89 docs-only, #90/#91 independent, #92–#96 chain on #90, four capture PRs with numbers once the user opens them). **Phase 3H ends here.**

---

## Open points (decide at execution, ledger the ruling)

- Task 2 Step 2: the base of #92–#96 (T90 vs `upstream/main` + duplicated types). Plan's preference: T90.
- Whether the 600/604 success-Status fact gets its own capture (`linux-news-status`) or rides in `linux-chat-ban`.
- `linux-tracker-startup`'s Server state names a non-shared password (`openkdx-lab`): acceptable because no frame uses it, or re-record with his lab (`up linux-tracker-startup`, server + tracker start, no client) — the latter is five minutes once Task 4 works; prefer re-recording.
- Xvfb vs a real display: his tool hard-codes `/tmp/.X11-unix/X0`; Xvfb `:0` satisfies it. If the lab box already runs an X server on :0 for our Wine scenarios, reuse it.

## Ledger

| Session | Date | Tasks | Fork PRs | OpenKDX commits | Pinned fork SHA | Upstream | Rulings / carried |
|---|---|---|---|---|---|---|---|
