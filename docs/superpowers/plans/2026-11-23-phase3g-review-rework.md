# Phase 3G: Upstream review rework — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This plan extends `2026-11-19-convergence-master.md` (read its Global Constraints, type mapping and Ledger first); it ends with its own `## Ledger`.

**Goal:** scottanderson/kdx#89 and #90 pass review and the whole `up/*` stack (#89–#96) is rebuilt as small topic commits that pass upstream's new CI rules (#97 frame numbers in capture tags, #98 backticked paths exist, #99 gitlint: the `Co-Authored-By` trailer names the model), with the same fixes merged in the fork and pinned by OpenKDX.

**Architecture:** Fork first. The review fixes land in the fork as two `feat/*` PRs (captures, settings), each a series of topic commits, gated by the fork's CI and lab job; OpenKDX bumps its pin and its docs; then every `up/*` branch is re-cut from `upstream/main` as topic commits taken from the fork's history (the squash commits of kdx#2–#9 split by file group, plus the fix commits folded in), force-pushed, and the PRs retitled. Fork `main` keeps its history; only `up/*` branches are rewritten — that is what they are for, and every SHA OpenKDX ever pinned stays reachable.

**Tech Stack:** as the master plan. New in the fork since #97–#99: `gitlint --commits upstream/main..HEAD` with `.gitlint-rules/co_author.py`, `python3 -I ci/check-docs.py` now checks frame numbers in `[O capture …]` tags and backticked paths in `captures/*.md`.

**Spec:** `docs/superpowers/specs/2026-11-08-openkdx-design.md` §6.1 (amendment of 2026-11-19: upstream every change), §5.3, §7.5. Upstream rules: scottanderson/kdx `AGENTS.md` at `1a6ebd3` (trailer names the model; PR body prose, no trailer; small commits with reasons). The review itself: scottanderson/kdx#89 (general comment + 17 line comments) and #90 (19 line comments); fetch them with `GET /repos/scottanderson/kdx/pulls/N/comments` and `/issues/N/comments` at the start of Tasks 2 and 3 — this plan paraphrases, the comments are the authority.

## Global Constraints

- Everything in the master plan's Global Constraints, plus: in the fork **every commit's trailer names the model**: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` for code written in this plan's sessions (the session model), `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` only on a commit whose code an Opus 5.5 review session wrote. The bare `Co-Authored-By: Claude <…>` fails gitlint. OpenKDX adopts the same form (Task 4 changes CONTRIBUTING).
- **No "OpenKDX" in the fork** outside `captures/README.md`'s one-line provenance row ("captured in a sandboxed lab") — not in notes, test names, doc comments, READMEs, fixture names or commit subjects. The lab password `openkdx-lab` is a value, not a name; it stays, and each capture note states it.
- **Capture names** (user's ruling 2026-11-23: `cli-<scenario>`; extended to upstream's convention of naming the client that produced the traffic): driven by our CLI → `cli-<scenario>`; driven by KDX Client 1.600 under Wine → `win-<scenario>`. Files, fixtures (`<name>-<port>.c2s.bin`), test files (`cli_<scenario>.rs`) and every tag follow.
- **Evidence discipline:** an `[O capture frames]` tag cites frames that show the claim (open the capture with `kdx-dissect … --inventory` and read them); what a frame does not show is `[I]` with "probably", or `[S FUN_…]` when the maintainer named the function. Never keep a tag the implementer has not verified against the frames.
- **Byte-exactness** of every fixture and capture after each decoder change: `cargo test -p kdx-proto -p kdx-codec -p kdx-dissect -p kdx-legacy` green, every capture `Same` under `kdx-dissect --audit`.
- **Public API changes** in `kdx-proto` are named in the commit body ("Public API: …").
- **Lab budget:** Task 2 and Task 3 each trigger the fork's lab job (~25 min); Task 4 triggers OpenKDX's. Nothing else runs on the lab. Never `pkill lab/differential.sh` there.
- **The user posts upstream.** The PAT is fork-only: replies on #89/#90, retitles and body edits of #89–#96 are text the plan hands the user (a `gh`-free world: the plan gives the exact text; the user pastes it). Force-pushes to `spitfire915:up/*` are ours.

## Review Focus

1. A `ServerSettings` blob laid out differently from Server 1.620's (sections out of table order, an unpadded trailing icon, a non-zero 0x54, an allow/deny flag of 7) must decode and re-encode byte for byte, not fail. Tests: Task 3 `decodes_sections_in_any_table_order`, `unpadded_trailing_icon_round_trips`, `ignored_54_and_allow_flag_take_any_value`.
2. A chat, tracker or allow/deny record whose string exceeds 255 bytes must fail `encode` with `EncodeError`, not be truncated or dropped. Tests: Task 3 `overlong_strings_fail_to_encode`.
3. Renaming 15 captures must leave no dangling path: `check-docs.py` (#98) finds backticked paths that do not exist; `cargo test -p kdx-codec --test fixtures` loads every fixture by name. Pinned by Task 2's verification steps.
4. Rewriting eight `up/*` branches must not lose a commit that fork `main` has: Task 5 diffs each rebuilt branch's tree against the fork's post-fix tree restricted to the branch's files (`git diff --stat <branch> main -- <paths>` empty).
5. The OpenKDX pin must advance to a fork SHA whose `Cargo.lock` entries CI accepts: Task 4's `cargo metadata --locked` and `python3 ci/check-deps.py`.

---

## File structure

Fork (`../kdx`):
- `captures/{cli,win}-<scenario>.{pcapng,md}` (renamed from `openkdx-*`), `captures/README.md` (table rows in words, with links, valid starting capture; the OpenKDX paragraph removed)
- `crates/kdx-proto/tests/{cli,win}_<scenario>.rs` (renamed), `crates/kdx-proto/tests/common/mod.rs` (gains `pub const LAB_PASSWORD: &[u8] = b"openkdx-lab";`), `crates/kdx-proto/tests/fixtures/{cli,win}-<scenario>[-<port>].{c2s,s2c}.bin` + `README.md`
- `crates/kdx-codec/tests/fixtures.rs` (name mapping for the renamed fixtures, one-line comment), `crates/kdx-dissect/tests/{captures,inventory}.rs`
- `docs/protocol/{handshake,sequences}.md`, `docs/protocol/messages/{accounts,administration,chat,files,folder-access,news}.md`
- `crates/kdx-proto/src/msg/{chat,server_info}.rs`
- `crates/kdx-client/tests/{lab,mock,admin}.rs`, `crates/kdx-client/tests/common/mod.rs`, `crates/kdx-client/README.md`, `crates/kdx-legacy/**` (OpenKDX mentions only; Task 5 sweeps them when the branch carrying them is rebuilt)

OpenKDX:
- `Cargo.toml`, `Cargo.lock` (pin), `CONTRIBUTING.md` (trailer form, capture names), `docs/protocol/README.md` (evidence column names `cli-*`/`win-*` upstream), `captures/README.md` (if it names the fork's copies), this plan's ledger and the master plan's upstream table.

---

### Task 1: Groundwork — upstream merged into the fork, the trailer rule adopted, the reply posted

**Files:**
- Fork: `main` (merge commit), `.gitlint`/`.gitlint-rules/co_author.py`/`.github/workflows/ci.yml` arrive with the merge
- OpenKDX: `CONTRIBUTING.md` (lines 16–24 and 49–57: the trailer names the model, both repositories)

**Interfaces:**
- Produces: fork `main` at a SHA containing `1a6ebd3` (Tasks 2–3 branch from it); the reply text for #89.

- [ ] **Step 1: Merge upstream into fork `main`**

```bash
cd ../kdx && git checkout main && git pull -q && git fetch -q upstream
git merge --no-ff upstream/main -m "Merge upstream main: CI rules for frame tags, note paths and the co-author trailer

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
python3 -I ci/check-docs.py; codespell; gitlint --commits upstream/main..HEAD
```
Expected: `check-docs` and codespell may now **fail** on `captures/openkdx-*.md` (#97/#98 rules); note each failure, they are Task 2's work. gitlint passes on the merge commit. Push `main`. Watch the fork's `check`/`docs` job: red is expected until Task 2 merges; record that in the ledger.

- [ ] **Step 2: OpenKDX CONTRIBUTING adopts the model-named trailer**

Edit `CONTRIBUTING.md`: every `Co-Authored-By: Claude <noreply@anthropic.com>` example becomes `Co-Authored-By: Claude <model> <noreply@anthropic.com>, naming the model that wrote the commit, e.g. Claude Fable 5.1` — in the OpenKDX section and the fork section; add one sentence: "upstream lints this (gitlint, UC1)". Commit on `main`: `docs(contributing): the Co-Authored-By trailer names the model`, trailer `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. Push.

- [ ] **Step 3: Hand the user the reply for #89** (they post it; stop for the answer only if it changes the naming — otherwise continue):

> Thanks — working through all of it. Two things to confirm before I force-push the stack: (1) naming: our captures were produced by two clients, so I plan `win-<scenario>` for the seven driven by KDX Client 1.600 under Wine (as `win-view-display`) and `cli-<scenario>` for the eight driven by our command-line client — fine? (2) the stack #89 → #90 → #91 … #96 will be rebuilt as small topic commits on current `main`, with model-named trailers and no external project names, after the same fixes land in our fork. I'll retitle #89 without the `(#2)`.

- [ ] **Step 4: Ledger row** in this plan (`3G-1`): merge SHA, CI state, reply posted. Commit `docs(plan): phase 3g ledger — groundwork`.

---

### Task 2: Fix #89 in the fork — renames, notes, evidence, tests (`feat/review-captures`)

**Files:**
- Fork, branch `feat/review-captures` from `main`: everything under "File structure → Fork" except `server_info.rs`.

**Interfaces:**
- Consumes: Task 1's fork `main`.
- Produces: fork `main` SHA after merge (Task 4 pins it; Task 5 takes the topic commits).

Fetch the 17 line comments of #89 first (`GET /repos/scottanderson/kdx/pulls/89/comments`) and tick each off against the commits below. One commit per bullet group; subjects imperative, bodies give the reason; each trailer names the model.

- [ ] **Step 1 — Rename by scenario.** `git mv` for each of the 15 captures and notes, the fixtures, the five test files; update `include_bytes!` paths, `kdx-codec/tests/fixtures.rs` name matching (`"cli-disconnect-ban-54332" => EMPTY_PASSWORD`, `"cli-upload-resume-36680" => "cli-upload-resume-36678"`, `_ if name.starts_with("cli-") || name.starts_with("win-")`), `kdx-dissect/tests/captures.rs`. Mapping:
  - `win-`: chat-basic, files-basic, first-connect, guest-login, news-basic, pm-basic, tracker-basic
  - `cli-`: accounts-classes, broadcast, connection-monitor, disconnect-ban, folder-access, server-history, server-settings, upload-resume
  Verify: `cargo test -p kdx-proto -p kdx-codec -p kdx-dissect` → all `ok`; `rg -l "openkdx-(accounts|broadcast|chat|connection|disconnect|files|first|folder|guest|news|pm|server|tracker|upload)" .` → empty. Commit: `Rename the lab captures by the client that produced them`.

- [ ] **Step 2 — Capture notes.** For each of the 15 notes: remove every "OpenKDX" (the client is "the command-line client of the lab" or "KDX Client 1.600 under Wine 6.0.3"); state the password (`openkdx-lab`, or "wrong-password then openkdx-lab" for disconnect-ban); `Decoded by` names the note's own capture; delete "checked by `crates/kdx-dissect/tests/inventory.rs`" (the file comes with #91 — Task 5 adds the sentence back on that branch only); where a note lists no steps (the eight CLI-driven ones), write them as the Wine notes do, from the scenario scripts in OpenKDX `lab/scenarios/`. `captures/README.md`: drop the OpenKDX paragraph; rewrite our rows in words with a link per message and a valid "start with" capture. Verify: `python3 -I ci/check-docs.py` → clean; `codespell` → clean. Commit: `Give each lab capture note its steps, password and decoder line`.

- [ ] **Step 3 — Protocol facts re-tagged**, one commit per file, each cell checked against the frames (`cargo run -p kdx-dissect -- captures/<name>.pcapng --password openkdx-lab --inventory`):
  - `chat.md`: the action marker is "probably a `u16` at 0x08" `[I]`, observed bytes `00 01 00 00` `[O win-chat-basic 24, 25]`.
  - `handshake.md`: status `1` failed gets its own `[O cli-disconnect-ban 9]` beside the `[S]`.
  - `files.md`: the 401 `kind` "probably a type table" `[I]`; resume offset = the size of the `.prt` on disk (0x20000 after a 150000-byte cut — rewrite the "rounded down to 0x1000" sentence, drop the 20000-byte cut and "large chunks" or tag them `[I]`), `[O cli-upload-resume 61, 88]` — confirm those frame numbers.
  - `accounts.md`: 510 returns the last editor `[O cli-accounts-classes <frame>]`; the 0x1000d trap on echoing it back is `[I]`; add the 504/507/511 padding Status claims here in that order, tagged `[I]` unless a frame shows 0x1000a; wrap at 80, no "Note that".
  - `folder-access.md`: keep only what `cli-folder-access` shows (success statuses); the padding rule stays as the reference had it or becomes `[I]`.
  - `sequences.md`: leave the `[O]` as supporting evidence, as he allowed.
  Verify after each: `python3 -I ci/check-docs.py` clean.

- [ ] **Step 4 — `chat.rs`.** Move `FLAG_ACTION` and `is_action` after the constants with blank lines; body: "Public API: `ChatText::ignored` split into `flags: u16` and `ignored_0a`". Verify `cargo test -p kdx-proto`. Commit: `Place ChatText's action flag after its constants`.

- [ ] **Step 5 — Tests.** `tests/common/mod.rs` gains `pub const LAB_PASSWORD`; the five renamed tests use it; `cli_connection_monitor.rs` asserts the actual path (read it from the inventory, e.g. `b"Uploads/tiny.txt"` — use the real value); `cli_upload_resume.rs`: replace the tautological offset test with `prt_size_is_the_resume_offset`: the 416 offset equals the byte count of 414 data frames the server acknowledged before the cut (sum the first transfer's data payload lengths from the fixture; expect `0x20000`); `kdx-codec/tests/fixtures.rs` comment one line, no project name; `fixtures/README.md` wrapped at 80, no project name. Verify: `cargo test -p kdx-proto -p kdx-codec` ok; `cargo +1.96 clippy --workspace --all-targets -- -D warnings`; `cargo fmt --check`. Commit: `Share the lab password between capture tests and assert real values`.

- [ ] **Step 6 — Fork PR.** `python3 ci/check-blocklist.py --root ../kdx`; `gitlint --commits main..HEAD`; push; open fork PR `feat/review-captures` → `main` titled "Rework the lab captures for upstream review" (body: the #89 comments addressed, as a list). Wait for `check`, `docs`, `lab` green. Squash-merge. Record kdx#11 and its SHA.

- [ ] **Step 7 — Ledger row** `3G-2`. Commit in OpenKDX: `docs(plan): phase 3g ledger — captures`.

---

### Task 3: Fix #90 in the fork — a lenient, byte-exact `ServerSettings` (`feat/review-settings`)

**Files:**
- Fork, branch `feat/review-settings` from `main` (after Task 2 merged): `crates/kdx-proto/src/msg/server_info.rs`, `crates/kdx-proto/tests/cli_server_settings.rs`, `docs/protocol/messages/administration.md`; callers unchanged unless the API step requires (`kdx-client/src/admin.rs`, `kdx-legacy/src/server_stg.rs`, `kdx-dissect/src/{audit,main}.rs`).

**Interfaces:**
- Consumes: Task 2's renamed capture `cli-server-settings` and fixture `cli-server-settings-43622`.
- Produces: `ServerSettings` as below; `kdx-legacy` `decode_server_stg`/`encode_server_stg` keep their signatures.

Fetch the 19 line comments of #90 first. Decisions this task makes (ledger them as rulings):
- `ServerSettings` gains `pub layout: SettingsLayout` — `pub struct SettingsLayout { pub order: Vec<usize>, pub gaps: [Vec<u8>; SECTIONS], pub trailing_pad: Option<Vec<u8>> }` — not stored in the wire sense: `decode` records the order sections appeared in, the bytes between them, and whether the blob ended padded; `encode` replays it; `Default` is Server 1.620's canonical layout (table order, zero padding to 4, padded end). This is what makes "accept any layout" and "re-encode byte for byte" both true.
- `section(&self, i) -> Vec<u8>` stays (declared in the body as a public API change, with `sections()` as a method); `FLAGS_AT`, `SPEED_LIMIT_AT`, `NAME_AT`, `ServerSettings::GREETING_ENABLED` removed; `pub const OTHER: [usize; 5] = [4, 6, 8, 9, 10]` replaces the literal list.
- `encode` returns `Result<Vec<u8>, EncodeError>` (chat records and the 255-byte limits can fail); callers add `?`/`map_err` — `kdx-legacy::encode_server_stg` becomes `Result<Vec<u8>>`, `kdx-client::set_server_settings` propagates.

- [ ] **Step 1 — Failing tests** in `tests/cli_server_settings.rs` (keep the two byte-exact tests; drop `name_length_byte_0x52_is_the_name_length`):

```rust
#[test] fn decodes_sections_in_any_table_order()      // build a blob with greeting before trackers; decode ok; encode == input
#[test] fn unpadded_trailing_icon_round_trips()        // icon of 5 bytes, blob ends at the icon's last byte; encode == input
#[test] fn ignored_54_and_allow_flag_take_any_value()  // byte 0x54 = 0x7f, AddressRule u16 = 7: decode ok, re-encode exact, rule.allow_raw == 7
#[test] fn overlong_strings_fail_to_encode()           // MainSettings.name of 256 bytes → Err(EncodeError::…len_u8…); same for TrackerEntry.name, ChatRecord
#[test] fn tracker_entry_strings_follow_the_twelve_lengths() // synthetic record per FUN_08061c10: lengths at 0x14..0x20, strings in that order; decode then encode exact
#[test] fn renamed_172_length_byte_matches_the_servers_105() // frame 20's 172 byte 0x52 == len of the name in frame 38's 105
```
Run: `cargo test -p kdx-proto --test cli_server_settings` → the new tests FAIL (compile errors for `layout`/`allow_raw` count).

- [ ] **Step 2 — Implement** in `server_info.rs`: `SettingsLayout`; `decode` takes each section at its table offset in any order (reject only overlap or out-of-range), records gaps, accepts an unpadded end; `unpinned_zero` removed (0x54 kept in `ignored_54`); `AddressRule { allow_raw: u16 }` with `pub fn allow(&self) -> bool { self.allow_raw == 1 }`; `TrackerEntry` with twelve `u8` lengths at 0x14–0x1f and strings in that order (fields: `name, comments, address, login, password?, groups, …` — name them from FUN_08061c10's order as the maintainer's comment lists; what the comment does not name stays `str_N`); `str255` → `len_u8` error; chat records' length from `ChatRecord::decode` returning `(Self, usize)` (add that if missing, or a `decoded_len`); `encode -> Result`. Run Step 1's tests → PASS; then `cargo test -p kdx-proto -p kdx-legacy -p kdx-client -p kdx-dissect` → ok (the `.stg` fixtures with non-empty chat sections are the chat-record guard). Commit per topic: `Decode settings sections at their table offsets in any order`; `Keep every settings byte the server does not check`; `Return an error for a settings string past 255 bytes`; `Lay out a tracker record as the server reads it`.

- [ ] **Step 3 — `administration.md`.** Split the long `main` row: a short row, then a table "Sections" (index, Rust field, contents, evidence). Tags: `tracker_count` `[S FUN_0805f754, FUN_08060268]`; `TrackerEntry` `[S FUN_08061c10, FUN_0805f754]`; `AddressRule` `[S FUN_08060268, FUN_0805f754]` (0x1b counted); chat section `[I]` "probably `ChatRecord`s; empty in every 171 captured"; `name_len` cites `[O cli-server-settings 20, 23, 38]` only for what those frames show (the 172 sets it, 105 reads it back) and the `OpenKDX La` truncation moves out (it was a lab observation with no capture: `[I]` or drop); the "one-setting-at-a-time diffs" sentence goes — fields beyond the captured ones are `[I]`. Verify `python3 -I ci/check-docs.py`, `codespell`. Commit: `Document the settings sections in a table and tag each layout by its evidence`.

- [ ] **Step 4 — Fork PR.** clippy 1.96, fmt, blocklist, gitlint; push; fork PR "Rework ServerSettings for upstream review" → wait green (lab job) → squash-merge; record kdx#12 and SHA.

- [ ] **Step 5 — Ledger row** `3G-3` with the rulings above. Commit `docs(plan): phase 3g ledger — settings`.

---

### Task 4: OpenKDX pin bump and docs

**Files:**
- OpenKDX: `Cargo.toml` (four `rev =`), `Cargo.lock`, `docs/protocol/README.md` (evidence names of the fork's copies, if any row names `openkdx-*`), `CONTRIBUTING.md` (one line: the fork's captures are `cli-*`/`win-*`), `captures/README.md` (same, if it names them), master plan upstream table (states unchanged until Task 5).

- [ ] **Step 1:** `.cargo/config.toml` absent; edit the four `rev =` to Task 3's merge SHA, then `cargo update -p kdx-proto -p kdx-codec -p kdx-client -p kdx-legacy` (as the 3C–3E pin bumps did); `cargo metadata --locked`; `cargo test -q --workspace`; `python3 ci/check-deps.py`. Expected: green, lock current.
- [ ] **Step 2:** Branch `phase3g/pin`, commits `build: pin kdx-* at <sha>` and `docs: fork capture names and the trailer form`; PR #7 → `main` ("Phase 3g: pin after the upstream review rework"); wait for the hosted jobs and the lab job (LAB_WAIT shares the lock with the fork's); squash-merge; `ssh openkdx-lab 'cd ~/openkdx-src && git pull'`.
- [ ] **Step 3:** Ledger row `3G-4`. (Ruling to record: a PR, because the ruleset asks for one and the bypass in 3F was for docs alone.)

---

### Task 5: Rebuild the `up/*` stack as topic commits

**Files:**
- Fork branches, names kept (GitHub ties each PR to its branch; only the titles change): `up/openkdx-captures`, `up/typed-server-settings`, `up/dissect-inventory`, `up/client-mock-notice`, `up/client-features`, `up/client-admin-transfers`, `up/lab-suite`, `up/kdx-legacy`.

**Interfaces:**
- Consumes: fork `main` after Task 3 (tree T).

Procedure, in stack order; each branch starts from its parent's new tip (the first from `upstream/main`):

- [ ] **Step 1 — Cut the branch's file set.** From T, the files the original squash commit (kdx#2 … kdx#9) touched, plus their Task 2/3 fixes: `git diff --name-only upstream/main <old up branch>` gives the list; take each file's content from T. Group into topic commits (3–8 per branch; e.g. #89: "Add the Wine-client captures and their fixtures", "Add the command-line client captures and their fixtures", "Record the chat action flag", "Record the proof status 1 of a failed login", "Record the resume offset and the .prt partial", "Record the 510 editor field", "Add the capture tests"). Each body states the reason and any public API change; trailer names the model.
- [ ] **Step 2 — Sweep** the branch for "OpenKDX"/"openkdx-" (`rg -i openkdx` → only the password value and the README's provenance row). On `up/dissect-inventory`, restore the notes' "checked by `crates/kdx-dissect/tests/inventory.rs`" sentence.
- [ ] **Step 3 — Verify per branch:** `gitlint --commits upstream/main..HEAD`; `python3 -I ci/check-docs.py`; `codespell`; `cargo +1.96 clippy --workspace --all-targets -- -D warnings`; `cargo fmt --check`; `cargo test --workspace` (lab tests skip without the env); `git diff --stat HEAD main -- <branch's files>` → empty (Review Focus 4). `python3 ci/check-blocklist.py --root ../kdx`.
- [ ] **Step 4 — Force-push** `git push --force-with-lease origin <branch>`. Then hand the user, per PR: the new title (#89 without `(#2)`: "Add the lab captures and the facts they pin") and a prose body that cites `spitfire915/kdx#N` for the fork PR; a reply on #89 and #90 listing each comment and the commit that answers it.
- [ ] **Step 5 — Verify upstream CI** on each PR via `GET /repos/scottanderson/kdx/commits/<sha>/check-runs` → all `success`.
- [ ] **Step 6 — Ledger:** final row `3G-5` with the eight branches' new tips and PR states; update the master plan's upstream table (same states, "reworked <date>"). Commit `docs(plan): phase 3g ledger — stack rebuilt`. Phase 3G ends here; Phase 4-0 may start.

---

## Open points

- The exact field order of `TrackerEntry` beyond the five names we had: taken from the #90 comment (FUN_08061c10); if the comment does not name a string, the field is `str_<offset>`, tagged `[S]`.
- Whether upstream wants `win-` for Wine-driven captures: asked in Task 1's reply; if he prefers all `cli-`, Task 2 Step 1's mapping changes before any rename.

## Ledger

| Session | Date | Tasks | Fork PRs | OpenKDX commits | Pinned fork SHA | Upstream | Rulings / carried |
|---|---|---|---|---|---|---|---|
| 3G-1 | 2026-11-23 | 1 | none (fork main merge `968c47f` = upstream `1a6ebd3` merged --no-ff) | `98e4b5d` CONTRIBUTING: trailer names the model | `6fb82b4` (unchanged) | #89 reply handed to the user (win-/cli- naming, stack rebuild) | Fork `check`/`docs` red on main until Task 2 (check-docs: 16 `lab/scenarios/*.sh` paths, 1 handshake path, 4 frameless tags in `captures/openkdx-*.md`); 10 pre-merge fork commits fail UC1 (history, kept). Ruling: trailers name the model that wrote the commit (Sonnet 5.5 implementers, Fable 5.1 controller, Opus 5.5 reviews). |
