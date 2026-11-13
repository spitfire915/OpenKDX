# Phase 3: Legacy Formats Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `openkdx-legacy` reads and writes every Haxial on-disk format the server and client keep (`Accounts.dat`, `KDXServer.stg`, `KDXTrackerServ.stg`, `News.dat`, `History/`, `News Archives/`, the catalog, `KDX.stg`, `.hap`, sound lists), byte-exact on round trip, documented in `docs/formats/`, with the §5.2 live-store rule (`.orig` once, atomic rewrite in the original format) and lab tests in which the original programs accept what we write.

**Architecture:** One module per file format, each exposing `decode(&[u8]) -> Result<T, LegacyError>` and `encode(&T) -> Vec<u8>` over `openkdx_proto::wire::{Reader, Writer}`, keeping every unknown byte in `ignored_*` fields so re-encoding is exact. Records that already exist on the wire (`Account`, `AccountClassInfo`, `Privileges`, `FolderAccess`, `ServerSettings`, `TrackerSettings`, `NewsgroupInfo`, `NewsMessage`) are reused from `openkdx-proto`, whose private `decode/encode` become public; nothing is modelled twice. `LegacyDir` is the live store: it opens a data directory, makes the one-time `.orig` copies, and saves through temp-file + rename. Formats are reverse-engineered by diffing lab files before and after one controlled change (driven by the `openkdx` CLI or the Windows client under Wine) and, where that fails, by Ghidra headless on the lab (`re/`, landmarks only).

**Tech Stack:** Rust 2024, MSRV 1.88; `openkdx-proto` (`Reader`/`Writer`, `twofish`, `sha1`, `xor_stream`, `AccountKey`), `thiserror`; no other runtime deps (`ci/check-deps.py` forbids tokio/network/UI for this crate). Lab: `lab/kdx-lab`, `lab/lib/scenario.sh`, `lab/lib/ui.sh`, Ghidra 11 headless (lab only).

**Spec:** `docs/superpowers/specs/2026-11-08-openkdx-design.md` §8 item 3, §5.2 (live store, `.orig`), §5.3 (legal), §7.1, §7.3 (`Theme`), §7.5 (differential legacy tests), §9. Reference: scottanderson/kdx `docs/protocol` (pinned `7f82081`; clone at `/private/var/folders/wk/hs0yq57n5q9g5s8kb5ybhh400000gn/T/opencode/kdx-ref`, re-clone with `git clone https://github.com/scottanderson/kdx` if gone) — wire only; `messages/administration.md` "ServerSettings" is the `%KDXSSTG` layout. Our notes: `docs/protocol/README.md`, `docs/formats/README.md`, `docs/formats/inventory.md`.

## Global Constraints

- Rust stable, edition 2024, `rust-version = "1.88"`; `#![forbid(unsafe_code)]`; MIT OR Apache-2.0. `openkdx-legacy` depends only on `openkdx-proto` and `thiserror` (dev: `hex`, `tempfile`, `openkdx-table` for key derivation checks). `python3 ci/check-deps.py` must pass.
- Byte-exact round trip is the acceptance test for every reader/writer: `encode(decode(file)) == file` for every committed fixture and every lab-generated file. Unknown bytes live in `ignored_*: [u8; N]` or `Vec<u8>` fields, never dropped.
- Live store (§5.2): the original files are the store. `LegacyDir::open` copies each present known file to `<name>.orig` **once** and never touches an existing `.orig`. Saves go to `<name>.tmp` then `rename`, preserving the original's permissions (`0600` on the lab). OpenKDX-only files get names the originals ignore (prefix `openkdx-`); none are needed in this phase.
- Legal (§5.3, `ci/check-blocklist.py`): nothing Haxial-made is committed. Committable: files the lab server/tracker/client wrote from our inputs (`fixtures/legacy/`), a `.hap` or sound list whose every bitmap/sound byte we authored. **Not committable:** any `.hap` saved from the Appearance Editor's default document (it carries Haxial's stock bitmaps), the stock `News.dat` welcome post, Haxial's appearance pack `Appearances041205.sit`. Such samples live on the lab under `~/haxial/legacy/samples/`; tests that need them skip (`eprintln!("skipped: …")`) without `OPENKDX_LAB_LEGACY_DIR`. The appearance pack is used only if the user says so when asked in Task 11.
- Evidence: every format claim in `docs/formats/*.md` carries `[O lab <scenario> <file>]` (diff observation), `[S server|client|appedit|sledit FUN_xxxxxxxx]` (Ghidra landmark in `re/landmarks.md`) or `[I]` (inference), as `docs/protocol/README.md` does. Ghidra projects and decompiled output stay on the lab; only scripts and landmarks are committed.
- Lab: `ssh openkdx-lab`; repo `~/openkdx-src` (`git pull` first); cargo in `~/.cargo/bin`; admin `admin` / `openkdx-lab`; everything through `lab/kdx-lab`; take `$RUN/lab.lock` with `flock -n` as `lab/differential.sh` does and wait while the CI job holds it after a push. Flood ban: < 25 connections per server start. Server files are written on each change while the server runs (inventory.md); stop the server (`kdx-lab server stop`) before copying a snapshot anyway.
- Branch `phase3/legacy-formats`, PR #4 (GitHub API with `git credential fill`; no `gh`). Commit after every task. Fresh-context review (subagent, most capable model) after Task 3 and on the whole branch before the PR.
- Carried forward, untouched: `Draw::encode` clamps (Phase 10); Phase 2 minors in PR #3's final message (CLI run streaming/exit codes, `.rsrc` sidecar, unused `lock_lab`).
- Open question settled here: **settings sections are modelled in `openkdx-proto`, not in `openkdx-legacy`.** Whatever Task 3's diffs pin (greeting, icon, allow/deny, trackers, history flag, ports, description) replaces opaque bytes in `openkdx_proto::msg::admin::ServerSettings`; `openkdx-legacy` only adds file I/O and the store. `TrackerSettings` sections stay opaque (Phase 5) but get public decode/encode and a disk round trip (Task 4).

## Review Focus

1. A corrupt, truncated or foreign file in the data directory (wrong magic, version ≠ expected, table offset past EOF): `decode` returns `LegacyError::{BadMagic, UnsupportedVersion, Decode}` and `LegacyDir` neither panics nor writes anything — the operator can still roll back. Tests: Task 2 `stg_truncated_is_error`, `stg_wrong_magic_is_bad_magic`, `stg_version_2_is_unsupported`; Task 1 `open_does_not_write_when_nothing_to_back_up`.
2. A crash during save must not leave a half-written live file: temp + rename, and a stale `<name>.tmp` from a previous crash is replaced, not appended to. Test: Task 1 `save_is_atomic_and_replaces_stale_tmp`.
3. Bytes we do not understand (non-zero `ignored_*`, sections 1, 2, 4, 6, 8–10, trailing bytes after the last section) survive a decode/encode; the original server must read our output. Tests: Task 3 `stg_random_ignored_bytes_round_trip` (proptest-free: a fixed set of mutated fixtures), Task 13 `lab_server_starts_on_our_rewritten_files`.
4. Boundary names: login of 31 bytes, server name of 63 bytes, empty description, bytes ≥ 0x80 in names — length bytes, padding and the NUL in `server_name` must be computed by the writer, not copied from the input. Tests: Task 7 `account_with_31_byte_login_round_trips`, Task 3 `set_name_recomputes_length_and_nul`.
5. The scrambled files (`Accounts.dat`, `KDX.stg`): if the original picks a fresh key per write, our writer must too, and the original must accept a file we wrote with a key it never produced. Tests: Task 6 `unscramble_known_plaintext_finds_admin_key`, Task 7 `lab_original_server_logs_in_account_we_wrote`.

---

## File structure

```
crates/openkdx-legacy/
  Cargo.toml                  + openkdx-proto, thiserror; dev: hex, tempfile, openkdx-table
  README.md
  src/lib.rs                  pub mod list + re-exports
  src/error.rs                LegacyError, Result
  src/store.rs                LegacyDir: open, .orig backups, atomic save, file names
  src/bases.rs                Bases/ folder-name conventions ([UL], [DB], [item])
  src/server_stg.rs           KDXServer.stg = proto ServerSettings + typed sections
  src/tracker_stg.rs          KDXTrackerServ.stg = proto TrackerSettings
  src/news.rs                 News.dat (%KDXNUUZ): NewsStore
  src/scramble.rs             the Accounts.dat / KDX.stg obfuscation
  src/accounts.rs             Accounts.dat: AccountsFile
  src/history.rs              History/ tab-separated logs: read, append
  src/news_archive.rs         News Archives/ text: append
  src/catalog.rs              catalog file: Catalog
  src/client_stg.rs           KDX.stg: ClientSettings
  src/hap.rs                  .hap: Theme
  src/soundlist.rs            sound lists: SoundList
  examples/legacytool.rs      dump / add-account / set-name / scramble, no deps
  tests/fixtures.rs           round trips over fixtures/legacy/**
  tests/store.rs              LegacyDir behaviour on a tempdir
  tests/lab.rs                skip unless OPENKDX_LAB_LEGACY_DIR
crates/openkdx-proto/src/msg/{admin,accounts,news,tracker,misc}.rs   decode/encode made pub; new section structs
crates/openkdx-cli/src/args.rs, cmd/                               + files catalog, news create-group --size-kib, server set-history
fixtures/legacy/lab-2026-11-1x/{server,tracker,client}/…  + <set>.md (steps that produced it); inventory.md rows
lab/legacy.sh                 lock, generate the sample set, snapshot, run tests/lab.rs, write-back check
lab/scenarios/legacy-samples.sh   deterministic sample generation (CLI + Windows client UI)
lab/scenarios/{hap-samples,soundlist-samples}.sh   editors under Wine, lab-only output
re/README.md  re/setup-ghidra.sh  re/scripts/{strings_xrefs.py,landmarks.py}  re/landmarks.md
docs/formats/{accounts-dat,server-stg,tracker-stg,news-dat,news-archives,history,catalog,bases,client-stg,hap,soundlist}.md
docs/formats/README.md (statuses), inventory.md (new sets)
.github/workflows/ci.yml      lab job: + lab/legacy.sh
```

---

### Task 1: Crate scaffolding, errors, live store, Bases/ conventions

**Files:**
- Modify: `crates/openkdx-legacy/Cargo.toml`, `src/lib.rs`
- Create: `src/error.rs`, `src/store.rs`, `src/bases.rs`, `tests/store.rs`, `tests/bases.rs`, `docs/formats/bases.md`, `docs/superpowers/plans/2026-11-13-phase3-legacy-formats.md` (this plan, copied in)

**Interfaces:**
- Produces:
  - `pub enum LegacyError { Io { path: PathBuf, source: std::io::Error }, Decode(DecodeError), BadMagic { expected: [u8; 8], found: [u8; 8] }, UnsupportedVersion { file: &'static str, version: u16 }, Invalid(&'static str), NotScrambled }` (thiserror; `From<DecodeError>`); `pub type Result<T> = std::result::Result<T, LegacyError>`.
  - `pub enum ServerFile { Accounts, Settings, News, Tracker }` with `fn name(self) -> &'static str` (`Accounts.dat`, `KDXServer.stg`, `News.dat`, `KDXTrackerServ.stg`).
  - `pub struct LegacyDir { root: PathBuf }`: `pub fn open(root: impl Into<PathBuf>) -> Result<LegacyDir>` (root must exist; for each `ServerFile` present without `<name>.orig`, `fs::copy` to it, preserving permissions; never overwrites an existing `.orig`); `pub fn root(&self) -> &Path`; `pub fn path(&self, f: ServerFile) -> PathBuf`; `pub fn read(&self, f: ServerFile) -> Result<Option<Vec<u8>>>` (None if absent); `pub fn save(&self, f: ServerFile, bytes: &[u8]) -> Result<()>` (write `<name>.tmp` truncating any stale one, copy the live file's permissions or `0600` on unix when none, `rename` over `<name>`); `pub fn history_dir(&self) -> PathBuf` (`History`), `pub fn news_archives_dir(&self) -> PathBuf` (`News Archives`), `pub fn bases_dir(&self) -> PathBuf`.
  - `pub enum BaseTag { Uploads, DropBox, Item(Vec<u8>) }`; `pub struct BaseName { pub display: Vec<u8>, pub tag: Option<BaseTag> }`; `pub fn parse_base_name(name: &[u8]) -> BaseName` (trailing ` [UL]` → Uploads, ` [DB]` → DropBox, ` [x]` → Item(x), else None); `pub fn format_base_name(&BaseName) -> Vec<u8>` (inverse).

- [ ] **Step 1: Write failing tests** `tests/store.rs`: `open_makes_orig_once_and_never_replaces_it` (write `Accounts.dat` = `b"v1"`, open → `.orig` = `v1`; change live to `v2`, open again → `.orig` still `v1`), `open_does_not_write_when_nothing_to_back_up` (empty dir → no new entries), `save_is_atomic_and_replaces_stale_tmp` (pre-create `KDXServer.stg.tmp` with junk; save; live file == bytes; no `.tmp` left), `save_keeps_mode_0600` (unix only). `tests/bases.rs`: `parse_ul_db_and_item_suffixes` (`Uploads [UL]`, `Admin Dropbox [DB]`, `Pics [Pictures]`, `Plain`), `format_is_inverse`.
- [ ] **Step 2:** `cargo test -p openkdx-legacy` → FAIL (unresolved).
- [ ] **Step 3:** Implement `error.rs`, `store.rs`, `bases.rs`; `lib.rs` re-exports `LegacyDir, ServerFile, LegacyError, Result, BaseName, BaseTag, parse_base_name, format_base_name`. Write `docs/formats/bases.md` from the lab directory listing in `inventory.md` (`[O lab first-run]`), status `read`.
- [ ] **Step 4:** Tests PASS; `cargo clippy -p openkdx-legacy --all-targets -- -D warnings`; `python3 ci/check-deps.py`.
- [ ] **Step 5:** Commit `feat(legacy): errors, live store with .orig backups and atomic saves, Bases/ names`.

---

### Task 2: Ghidra headless on the lab and `re/`

**Files:**
- Create: `re/README.md`, `re/setup-ghidra.sh`, `re/scripts/strings_xrefs.py`, `re/scripts/landmarks.py`, `re/landmarks.md`

**Interfaces:**
- Produces: `re/setup-ghidra.sh` (lab only; downloads Ghidra 11.x and a JDK 21 into `~/ghidra/`, verifies SHA-256 from the release page, creates project `~/ghidra/projects/openkdx`; imports `KDXServer.lexe`, `KDXTrackerServ.lexe`, `KDXClient.exe`, `AppearanceEdit.exe`, `KDXSoundListEdit.exe` with `analyzeHeadless … -import … -postScript`). `strings_xrefs.py NEEDLE…`: prints each defined string containing a needle and the functions that reference it (`FUN_xxxxxxxx`, call-site address). `landmarks.py`: prints `FUN_… entry size` for a list of functions, for pasting into `re/landmarks.md`. `re/landmarks.md`: table `Binary | Function | What | Evidence used in` — the only RE artefact in the repo.
- [ ] **Step 1:** On the lab, run `re/setup-ghidra.sh`; `analyzeHeadless` of `KDXServer.lexe` completes; `strings_xrefs.py Accounts.dat KDXServer.stg News.dat History "News Archives"` lists at least one function per needle. Record them in `re/landmarks.md`.
- [ ] **Step 2:** `re/README.md`: what is committed, what is not (projects, decompiled output), how to add a landmark, the evidence tag format. Add `~/ghidra` to `lab/README.md`'s machine inventory.
- [ ] **Step 3:** Commit `re: ghidra headless setup and string-xref scripts; first landmarks`.

---

### Task 3: `KDXServer.stg` — read/write, typed sections, first format landed

**Files:**
- Modify: `crates/openkdx-proto/src/msg/admin.rs` (`ServerSettings::decode(bytes: &[u8]) -> Result<Self, DecodeError>` and `to_bytes(&self) -> Vec<u8>` public; sections typed as diffs reveal them), `crates/openkdx-client/src/admin.rs` (`set_server_name` adjusts for the new fields), `crates/openkdx-cli` (`server set-history on|off`, `server set-description`, `server set-greeting`, if the fields are pinned)
- Create: `crates/openkdx-legacy/src/server_stg.rs`, `tests/fixtures.rs`, `lab/scenarios/legacy-samples.sh` (first part), `fixtures/legacy/lab-2026-11-1x/server/KDXServer.stg` variants, `docs/formats/server-stg.md`

**Interfaces:**
- Consumes: `openkdx_proto::msg::admin::{ServerSettings, ServerSettingsMain}`, `ServerGreeting` (107 layout for section 3), `ServerIcon` (106 bytes for section 5).
- Produces (proto): `pub struct ServerSettings { version, ignored, table: [(u32,u32); 11], value_64, body }` unchanged in shape plus `pub fn decode(&[u8])`, `pub fn to_bytes(&self)`, `pub fn set_section(&mut self, i: usize, bytes: &[u8])` (rewrites body and table offsets at multiples of 4, entry 7 = end-of-blob as in a 171), `pub fn greeting(&self) -> Option<Vec<u8>>`, `pub fn icon(&self) -> Option<Vec<u8>>`; `ServerSettingsMain` gains every field pinned by the diffs (expected from the §5.2 table: description, address, TCP port(s), speed limit confirmed, history-logging flag, allow/deny list, tracker registrations — each either a named field or left `ignored_*` with the diff recorded in the doc). Produces (legacy): `pub fn decode_server_stg(bytes: &[u8]) -> Result<ServerSettings>` (magic `%KDXSSTG` → `BadMagic`, version ≠ 1 → `UnsupportedVersion`), `pub fn encode_server_stg(&ServerSettings) -> Vec<u8>`; `impl LegacyDir { pub fn load_settings(&self) -> Result<Option<ServerSettings>>; pub fn save_settings(&self, &ServerSettings) -> Result<()> }`.
- [ ] **Step 1: Lab diffs.** `lab/scenarios/legacy-samples.sh` part 1: `scenario_begin legacy-samples; use_cli; start_server`; snapshot `KDXServer.stg` as `00-fresh`; then one change per snapshot through `cli`: `server set-name "Legacy Lab"` → `01-name`; through the Windows client's Server Settings window (`lab/lib/ui.sh`, offsets recorded in the scenario): description → `02-description`, greeting on with text → `03-greeting`, history logging on → `04-history`, speed limit 20 KiB/s → `05-speed`, a deny entry with expiry → `06-deny`, tracker `127.0.0.1` → `07-tracker`, port 10701 → `08-port`. `cmp -l` each against its predecessor; write the offsets and values into `docs/formats/server-stg.md` with `[O lab legacy-samples 0N]` tags. Copy the snapshots to `fixtures/legacy/lab-2026-11-1x/server/stg/` with a `legacy-samples.md` listing the steps.
- [ ] **Step 2: Write failing tests** `tests/fixtures.rs`: `every_server_stg_fixture_round_trips` (glob `fixtures/legacy/**/KDXServer*.stg` and `stg/*.stg`; `encode(decode(b)) == b`), `stg_wrong_magic_is_bad_magic`, `stg_truncated_is_error` (cut at 0x40 → `Decode(Truncated)`), `stg_version_2_is_unsupported`, `set_name_recomputes_length_and_nul` (name of 63 bytes → byte 0x52 = 63, NUL after name, decode gives the name back), `stg_history_flag_reads_true_on_04_history` (and false on `00-fresh`), `stg_random_ignored_bytes_round_trip` (for each fixture, xor every `ignored_*` byte with 0x5a via the typed struct, encode, decode, encode again → equal).
- [ ] **Step 3:** Run → FAIL. Implement proto changes (public decode/encode, `set_section`, named fields with doc comments citing the tags) and `server_stg.rs`. Keep `ServerSettingsMain::to_bytes` byte-exact for the Phase 1 captures (`cargo test -p openkdx-dissect` still passes).
- [ ] **Step 4:** Tests PASS workspace-wide; clippy clean. Update `docs/formats/README.md` row to `read/write`.
- [ ] **Step 5:** Commit `feat(legacy,proto): KDXServer.stg read/write; settings sections pinned by lab diffs`.
- [ ] **Step 6: Review gate 1.** Dispatch a fresh-context reviewer (most capable model) with: the spec §5.2/§5.3, this plan's Global Constraints and Tasks 1–3, the diff `main..HEAD`. Fix what it finds before Task 4; note dismissed items in the PR description.

---

### Task 4: `KDXTrackerServ.stg`

**Files:**
- Modify: `crates/openkdx-proto/src/msg/tracker.rs` (`TrackerSettings::decode(&[u8])`, `to_bytes()` public)
- Create: `src/tracker_stg.rs`, `docs/formats/tracker-stg.md`, fixture `fixtures/legacy/lab-2026-11-1x/tracker/KDXTrackerServ.stg` (fresh + one group added via the client's tracker UI or left fresh if the UI route costs more than an hour — then the doc says so)

**Interfaces:**
- Produces: `pub fn decode_tracker_stg(&[u8]) -> Result<TrackerSettings>`, `pub fn encode_tracker_stg(&TrackerSettings) -> Vec<u8>`; `LegacyDir::{load_tracker_settings, save_tracker_settings}`. Sections stay opaque (Phase 5).
- [ ] **Step 1:** Test `every_tracker_stg_fixture_round_trips`, `tracker_stg_wrong_magic`. FAIL → implement → PASS.
- [ ] **Step 2:** Commit `feat(legacy): KDXTrackerServ.stg read/write (sections opaque until Phase 5)`.

---

### Task 5: `News.dat` — the newsgroup store

**Files:**
- Modify: `crates/openkdx-proto/src/msg/news.rs` (`NewsgroupInfo`, `NewsMessage` `decode(&mut Reader)`/`encode(&mut Writer)` public), `crates/openkdx-cli` (`news create-group --size-kib`)
- Create: `src/news.rs`, `docs/formats/news-dat.md`, fixtures `server/news/{10-empty-after-delete,11-one-post,12-two-groups,13-deleted-post}.dat`

**Interfaces:**
- Produces: `pub struct NewsStore { pub version: (u16, u16), pub groups: Vec<NewsGroupRecord>, pub ignored_header: Vec<u8> }`, `pub struct NewsGroupRecord { pub info: NewsgroupInfo (or a disk twin if the diff shows a different header), pub access: NewsAccess, pub messages: Vec<NewsMessage>, pub ignored: … }` — final field list follows the diffs; `decode_news(&[u8]) -> Result<NewsStore>` (magic `%KDXNUUZ`, `00 02 00 01`), `encode_news(&NewsStore) -> Vec<u8>`; `LegacyDir::{load_news, save_news}`; `NewsStore::next_message_id(&self, group) -> u32`.
- [ ] **Step 1: Lab diffs** (scenario part 2): delete the stock welcome post (`news delete`), snapshot `10-empty-after-delete` (now free of Haxial prose — verify with `strings`); `news post General\ News --subject A --body B` → `11`; `news create-group Second --description D --size-kib 8` → `12`; `news delete` the post → `13` (does the file shrink or mark?). Also a 0-byte `News.dat` (first-run state) must decode to an empty store.
- [ ] **Step 2: Tests** `every_news_fixture_round_trips`, `empty_news_dat_is_empty_store`, `news_11_has_one_post_with_subject_a`, `news_12_second_group_size_8`, `post_appended_by_us_decodes` (append a `NewsMessage` with `next_message_id`, encode, decode). FAIL → implement → PASS.
- [ ] **Step 3:** `docs/formats/news-dat.md`; README status `read/write`. Commit `feat(legacy): News.dat read/write`.

---

### Task 6: The scrambling of `Accounts.dat` and `KDX.stg`

**Files:**
- Create: `src/scramble.rs`, `tests/scramble.rs`, `examples/legacytool.rs` (`scramble decode <file>` dumps plaintext to stdout for the lab work), `re/landmarks.md` rows

**Interfaces:**
- Produces: `pub fn unscramble(bytes: &[u8]) -> Result<Vec<u8>>` and `pub fn scramble(plain: &[u8], key: u32 /* or [u8;N] as found */) -> Vec<u8>` with `pub fn fresh_key() -> …` using no RNG dep (time-seeded FNV is fine; the key is not a secret — document why). Known-plaintext check: `AccountKey::from_password(b"openkdx-lab").as_bytes()` and `b"admin"` appear in `unscramble(Accounts.dat)`.
- [ ] **Step 1: Differential on the lab.** From `00-fresh` `Accounts.dat` (2008 B): `accounts create u1 --new-password p1` → `A`; `accounts set u1 --name X` → `B`; `accounts set u1 --name Y` → `C` (same length). `cmp -l B C | wc -l`: a handful of bytes → stream cipher without chaining, key fixed per file or in the first word; everything → chaining or per-write key. Record in `docs/formats/accounts-dat.md`.
- [ ] **Step 2: Candidates, in order, each a test in `tests/scramble.rs` that currently fails:** (a) `xor_stream` seeded by the first u32 over bytes 4.. and 0.. (already excluded on the Mac: no), other seeds (file length, constant), other constants than `0x4878` found near the file-write function; (b) Twofish-256 ECB/CBC with `AccountKey::GUEST`, zero key, `from_password(b"")`, a 32-byte constant from the binary; (c) Ghidra: `strings_xrefs.py Accounts.dat` → writer → the transform; landmark it `[S server FUN_…]`. Stop at the first candidate whose output contains the admin key and `admin`.
- [ ] **Step 3:** `unscramble_known_plaintext_finds_admin_key`, `scramble_then_unscramble_is_identity`, `unscramble_rejects_short_input` PASS. `examples/legacytool.rs` `scramble decode` works on the fixture. Commit `feat(legacy): Accounts.dat/KDX.stg scrambling undone and redone`.

---

### Task 7: `Accounts.dat` — accounts, classes, folder access

**Files:**
- Modify: `crates/openkdx-proto/src/msg/accounts.rs` (`Privileges`, `Account`, `AccountClassInfo` decode/encode public), `msg/misc.rs` (`FolderAccess` likewise, if it lives here)
- Create: `src/accounts.rs`, `docs/formats/accounts-dat.md`, fixtures `server/accounts/{00-fresh,20-u1,21-class1-renamed,22-fa-item,23-u1-deleted}.dat`, lab tests in `tests/lab.rs`, `lab/legacy.sh`

**Interfaces:**
- Produces: `pub struct AccountsFile { pub header: AccountsHeader /* ignored bytes + counts */, pub classes: Vec<AccountClassInfo>, pub accounts: Vec<Account>, pub folder_access: Vec<FolderAccess>, pub trailer: Vec<u8> }` (if folder access turns out to live in the `.stg`, it moves to Task 3's struct and this field goes); `decode_accounts(&[u8]) -> Result<AccountsFile>` (unscramble, then parse), `encode_accounts(&AccountsFile, key) -> Vec<u8>`; `AccountsFile::{find(login) -> Option<&Account>, upsert(Account), remove(login) -> bool}`; `LegacyDir::{load_accounts, save_accounts}`. `legacytool add-account <dir> <login> <password> [--class N]` builds an `Account` with `openkdx_client::accounts::new_account`'s field choices (copy them; legacy must not depend on client) and saves.
- [ ] **Step 1: Lab diffs** (scenario part 3): `00-fresh` → `accounts create u1` → `20`; `classes set 1 --name Renamed` → `21`; `folder-access create Pics --icon pictures` → `22`; `accounts delete u1` → `23` (record removed or tombstoned?). Diff the unscrambled files; locate record boundaries (expected ≈152 B per account: the wire `Account` 0x80 header + strings + pad4). Document.
- [ ] **Step 2: Tests** `every_accounts_fixture_round_trips`, `fresh_accounts_has_admin_and_guest_and_12_classes` (or the count found), `account_20_u1_has_class_1_and_key_of_p1`, `account_with_31_byte_login_round_trips`, `upsert_then_remove_restores_fresh_bytes_modulo_key` (unscrambled plaintext equal). FAIL → implement → PASS.
- [ ] **Step 3: Lab verification** `lab/legacy.sh`: lock; `reset server`; `server start`; part 3 changes; `server stop`; snapshot to `$RUN/legacy/snapshot/`; `cargo test -p openkdx-legacy --test lab` with `OPENKDX_LAB_LEGACY_DIR=$RUN/legacy/snapshot` (tests: `lab_accounts_match_cli_changes`, `lab_every_file_round_trips`); then `legacytool add-account $RUN/server legacyuser secret`; `server start`; `cli --as legacyuser secret login` succeeds and `accounts show legacyuser` matches → `lab_original_server_logs_in_account_we_wrote` is the shell assertion, logged in the script. Server log must not say "licen" or report a damaged file.
- [ ] **Step 4:** `docs/formats/accounts-dat.md`, README `verified`. Commit `feat(legacy): Accounts.dat read/write; original server accepts our file`.

---

### Task 8: `History/` and `News Archives/`

**Files:**
- Create: `src/history.rs`, `src/news_archive.rs`, `docs/formats/history.md`, `docs/formats/news-archives.md`, fixtures `server/History/*`, `server/News Archives/*`

**Interfaces:**
- Consumes: `openkdx_proto::msg::admin::{HistoryRecord, HistoryKind}`, `Time1900`.
- Produces: `pub struct HistoryLine { pub time: Time1900 /* parsed from the original's text form */, pub kind: HistoryKind, pub address: Vec<u8>, pub nick: Vec<u8>, pub login: Vec<u8>, pub detail: Vec<u8> }` with the exact column set the files show; `pub fn parse_history(bytes: &[u8]) -> Result<Vec<HistoryLine>>`, `pub fn format_history_line(&HistoryLine) -> Vec<u8>` (inverse, byte-exact including line ending), `pub fn history_file_name(day: Time1900) -> String` (the original's naming, one file per day or per start — as observed); `LegacyDir::append_history(&self, &HistoryLine) -> Result<()>`. `pub fn format_news_archive(group: &[u8], msg: &NewsMessage) -> Vec<u8>` and `LegacyDir::append_news_archive(group, msg)`; `archive_file_name(group) -> String`.
- [ ] **Step 1: Lab** (scenario part 4): with history on since `04-history`, the logins/posts above produce `History/`; set `General News` size to 4 KiB and post two 3 KB bodies so the archive fills; copy both directories into the fixture set (check with `strings` that no Haxial prose came along).
- [ ] **Step 2: Tests** `history_fixture_lines_round_trip_byte_exact`, `history_login_line_has_kind_login_and_admin`, `news_archive_fixture_matches_formatter` (re-format the archived messages from `News.dat` snapshot `11` → identical bytes). FAIL → implement → PASS.
- [ ] **Step 3:** Docs, README `read/write` (append-only per §5.2). Commit `feat(legacy): History/ and News Archives/ readers and append-only writers`.

---

### Task 9: The catalog

**Files:**
- Modify: `crates/openkdx-cli` (`files catalog`: send 450, print 451 statuses until done — check `openkdx-client` for an existing call first)
- Create: `src/catalog.rs`, `docs/formats/catalog.md`, fixtures `server/<catalog file>` (fresh and after one file added), `Bases/Default/…` listing in the `.md`

**Interfaces:**
- Produces: `pub struct Catalog { pub header: …, pub entries: Vec<CatalogEntry> }`, `pub struct CatalogEntry { pub path: FilePath or Vec<Vec<u8>>, pub size: u64, pub modified: Time1900, pub ignored: … }` per the bytes; `decode_catalog`, `encode_catalog`; `LegacyDir::{load_catalog, save_catalog}`; `pub const CATALOG_FILE: &str` (the name the server uses — found by `ls` after generation).
- [ ] **Step 1: Lab** (part 5): `files put` a small file of ours, `files catalog`, snapshot; `files put` a second, regenerate, snapshot; `files search` confirms the server reads it. Diff.
- [ ] **Step 2: Tests** `catalog_fixtures_round_trip`, `catalog_lists_our_two_files`. FAIL → implement → PASS. Commit `feat(legacy): catalog read/write`.

---

### Task 10: `KDX.stg` — the client's settings

**Files:**
- Create: `src/client_stg.rs`, `docs/formats/client-stg.md`, fixtures `client/{30-fresh,31-nick,32-address-book,33-colour}.stg` (lab-written by Client 1.600 under Wine; clean exit each time)

**Interfaces:**
- Produces: `pub struct ClientSettings { pub identity: ClientIdentity { name, description, text_color: Color, background_color: Color, icon: Vec<u8> }, pub address_book: Vec<AddressBookEntry { name, address, login, password: Vec<u8>, … }>, pub sections: … opaque, pub ignored… }`, `decode_client_stg(&[u8]) -> Result<ClientSettings>` (via `scramble::unscramble`), `encode_client_stg(&ClientSettings, key) -> Vec<u8>`. Window positions and registration stay opaque bytes, documented as such (Phase 7 owns them).
- [ ] **Step 1: Lab** (part 6, `lab/lib/ui.sh`): start the Windows client, exit cleanly → `30`; set nick `legacynick` → `31`; add address-book entry `Lab` / `127.0.0.1` / `admin` / `openkdx-lab` → `32`; change the background colour → `33`. Diff unscrambled.
- [ ] **Step 2: Tests** `client_stg_fixtures_round_trip`, `client_31_nick_is_legacynick`, `client_32_address_book_has_lab_entry_with_password`. Lab check in `lab/legacy.sh`: write `34` with nick `openkdxwrote` via `legacytool set-nick`, start the client, `ui_shot` the Settings window, assert via the window title or the `User List` nick after `connect_to`. FAIL → implement → PASS.
- [ ] **Step 3:** Docs; README `verified` or `read/write` per the check's outcome. Commit `feat(legacy): KDX.stg read/write`.

---

### Task 11: `.hap` appearance files → `Theme`

**Files:**
- Create: `src/hap.rs`, `docs/formats/hap.md` (with the "Not supported" section §7.3 requires, initially listing what Task 11 does not decode), `lab/scenarios/hap-samples.sh`, `tests/hap.rs`

**Interfaces:**
- Produces (§7.3): `pub struct Theme { pub palette: Palette { chrome, panes, highlight_frame, title_bar, text, … : Color }, pub fonts: Vec<FontSpec>, pub bitmaps: Vec<NamedBitmap { id: u16 or name, width, height, depth, pixels: Vec<u8> }>, pub ignored: … }`, `decode_hap(&[u8]) -> Result<Theme>`, `encode_hap(&Theme) -> Vec<u8>`, `pub fn bitmap_to_rgba(&NamedBitmap) -> Vec<u8>` for the one pixel format the samples show.
- [ ] **Step 0: Ask the user** whether Haxial's appearance pack may be used as lab-only read-input (never committed). Proceed with self-made samples either way.
- [ ] **Step 1: Lab:** `hap-samples.sh` runs Appearance Editor 1.240 under Wine (`kdx-lab x run`), saves the untouched default as `h00.hap` (lab-only), then one change per save: one palette colour → `h01`, a second → `h02`, a font size → `h03`, replace one bitmap with our own 16×16 PNG of a plain pattern → `h04`. Outputs go to `~/haxial/legacy/samples/hap/`. Only a `.hap` whose every bitmap we replaced is copied into `fixtures/legacy/…/client/Appearances/openkdx-test.hap` — if replacing them all is impractical, none is committed and `tests/hap.rs` uses `encode_hap` of a `Theme` built in the test plus the lab-only samples under `OPENKDX_LAB_LEGACY_DIR`.
- [ ] **Step 2: Tests** `hap_samples_round_trip` (lab-only, skips), `hap_h01_differs_from_h00_in_one_palette_entry` (lab-only), `theme_encode_decode_is_identity` (always), `hap_wrong_magic_is_bad_magic`. FAIL → implement → PASS.
- [ ] **Step 3:** `docs/formats/hap.md` with the evidence tags and "Not supported"; README `read` or `read/write`. Commit `feat(legacy): .hap appearance files → Theme`.

---

### Task 12: Sound lists

**Files:**
- Create: `src/soundlist.rs`, `docs/formats/soundlist.md`, `lab/scenarios/soundlist-samples.sh`, `tests/soundlist.rs`, fixture `client/Sound Lists/openkdx-test` (only if every embedded sound is ours; our WAVs are generated by a Python one-liner in the scenario)

**Interfaces:**
- Produces: `pub struct SoundList { pub entries: Vec<SoundEntry { event: SoundEvent or u16, data: Vec<u8> /* or file reference */ }>, pub ignored }`, `decode_soundlist`, `encode_soundlist`.
- [ ] **Step 1: Lab:** KDXSoundListEdit 1.000 under Wine: empty list → `s00`; assign our 0.1 s WAV to one event → `s01`; a second event → `s02`. Diff.
- [ ] **Step 2: Tests** `soundlist_samples_round_trip`, `soundlist_s01_has_one_entry_with_our_wav_bytes`, `empty_soundlist_round_trips`. FAIL → implement → PASS. Commit `feat(legacy): sound lists`.

---

### Task 13: Lab entry point, CI, docs, PR

**Files:**
- Modify: `.github/workflows/ci.yml` (lab job: `- name: Legacy formats against the original programs` → `run: lab/legacy.sh`), `lab/README.md`, `docs/formats/README.md` (final statuses), `docs/formats/inventory.md` (new sets, regenerated with `lab/inventory.sh`), `crates/openkdx-legacy/README.md` (modules, store rule, how samples are made, what is not committed and why), `CONTRIBUTING.md` (section "Format code": byte-exact rule, evidence tags, fixture policy), `docs/protocol/README.md` (new confirmations from the settings diffs)
- Create/finish: `lab/legacy.sh` (whole flow: lock, `legacy-samples.sh`, snapshot, `cargo test -p openkdx-legacy --test lab`, write-back checks for Accounts.dat and KDX.stg, `lab_server_starts_on_our_rewritten_files`: rewrite all four server files via `legacytool rewrite <dir>`, start the server, `cli login` + `news groups` + `server settings` equal the snapshot)
- [ ] **Step 1:** `lab/legacy.sh` runs green on the lab twice in a row (idempotent; `.orig` files present after the first run and unchanged after the second).
- [ ] **Step 2:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `python3 ci/check-deps.py`, `python3 ci/check-blocklist.py --range main HEAD`, `python3 -m unittest discover ci` all pass locally; `strings` over every new fixture shows no Haxial prose.
- [ ] **Step 3: Review gate 2.** Fresh-context reviewer over the whole branch (`main..HEAD`), with the spec, this plan and `docs/formats/`. Fix or record.
- [ ] **Step 4:** Push; open PR #4 via the API (`POST /repos/spitfire915/OpenKDX/pulls`, body: summary, format status table, what is lab-only, deferred minors); wait for the four hosted jobs and the lab job; squash-merge as #2/#3.
- [ ] **Step 5 (on main after merge):** docs follow-up commit: README status line (Phase 3 complete, formats verified), crate table row split (`openkdx-legacy` done), spec §8 `**Phase 3 complete:** 2026-11-xx, branch …(PR #4). …`, CONTRIBUTING, `docs/formats/README.md` index, `lab/README.md`.

---

## Self-review notes

- Spec coverage: §5.2 table — every row has a task (Accounts.dat 6–7, KDXServer.stg 3, Bases/ 1, news store 5, News Archives 8, History 8, catalog 9, KDX.stg 10, .hap 11, sound lists 12); §5.2 rule (`.orig`, rewrite in original format, append-only logs) Task 1 + 7/8/13; §7.3 `Theme` Task 11; §7.5 legacy differential tests Task 7/10/13; §5.3 Tasks 2 and the fixture policy; §8 docs Task 13. `KDXTrackerServ.stg` is not in §8 item 3 but is in `docs/formats/README.md`'s table and costs one task (4).
- Deliberately open until the bytes decide: exact field lists of `NewsStore`, `AccountsFile`, `Catalog`, `ClientSettings`, `Theme`, `SoundList`; whether folder access lives in `Accounts.dat` or the `.stg`; whether `.hap` samples can be committed. Each has its decision rule written in its task.
- Risk: Task 6 is the one task with no guaranteed finish time. Budget: differential (1 h) → candidates (2 h) → Ghidra (half a day). If the transform is still unknown after that, land Tasks 8–9 and 11–12 first (they do not depend on it), then return; the PR does not merge without Accounts.dat.
- Proto changes are additive (visibility, `set_section`, named fields replacing `ignored_*` ranges of equal total size); every Phase 1/2 capture test must stay byte-exact — the workspace test run in each task's Step 4 is the guard.

## Execution notes (2026-11-13)

Executed natively with `executing-plans`; the ledger's rulings are summarised
in PR #4. Order became 1, 2, 3 (review gate 1), 6, 7, 4, 5, 8, 9, 10, 13:
`KDXTrackerServ.stg` turned out scrambled, so Task 4 followed the scramble;
the scramble itself fell to lab diffs alone (an LCG XOR stream) with no
Ghidra needed. Tasks 11 and 12 (`.hap`, sound lists) are deferred: the
editors' Haxial-drawn save dialog lists no volumes under Wine, so no sample
can be made from their defaults; the appearance pack (manual download,
lab-only, agreed with the user) is the way in.
