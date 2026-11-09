# OpenKDX Phase 0: Repository and Lab — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A repository that builds, tests, and lints an empty eleven-crate workspace on three OSes, refuses Haxial material, fetches and caches the two challenge tables, and a Linux lab that runs the original KDX programs in sandboxes and records the first protocol captures.

**Architecture:** One Cargo workspace with placeholder crates whose dependency edges are already correct (proto has no I/O, legacy has no UI). `openkdx-table` is the only crate with real code in this phase: download → verify SHA-256 → extract story → cache. Lab scripts are Bash, run as the unprivileged `openkdx` user on the Ubuntu 22.04 box, driving `bwrap` sandboxes with private network namespaces and `dumpcap`.

**Tech Stack:** Rust stable (rust-toolchain.toml pins the channel), tokio, reqwest (rustls), sha2, zip, clap, tempfile. GitHub Actions. Bash, bubblewrap, Wine, dumpcap, Python 3 for CI check scripts.

**Spec:** `docs/superpowers/specs/2026-11-08-openkdx-design.md`

## Global Constraints

- Licence: `MIT OR Apache-2.0` in every `Cargo.toml`; files `LICENSE-MIT` and `LICENSE-APACHE` at repo root (spec §5.3, §9).
- Crate names and kinds exactly as spec §7.1: `openkdx-proto`, `openkdx-codec`, `openkdx-legacy`, `openkdx-table`, `openkdx-client`, `openkdx-server`, `openkdx-tracker`, `openkdx-ui`, `openkdx-app`, `openkdxd`, `openkdx-dissect`.
- Dependency rules: `openkdx-proto` depends on no async runtime, network, or UI crate; `openkdx-legacy` depends on no network or UI crate (spec §7.1).
- No Haxial material in the repository: binaries, archives, the KDX story (SHA-256 `4fca7636e77e85fdde95862a6be66e12e5e031ad61add65c5e975d3daff4b9e7`, 1739 bytes), the GConsole story (SHA-256 `b43f22222971a57a0d8bdb3a5af2747c7ff511ea88f0ba8c24e38e9edc029c36`, 3193 bytes), `.hap` files, icons (spec §5.3).
- Challenge tables are fetched or imported at runtime and cached under the user data directory; tests read them from `OPENKDX_KDX_TABLE` / `OPENKDX_HGC_TABLE` (base64) or the cache and **skip** without them (spec §5.1).
- Story extraction anchors: KDX story begins at the bytes `This program including the following story`; GConsole story begins at `The following story is Copyright 2004` (spec §5.1 via reference handshake.md).
- Smallest known archives holding each story: `KDXTrackerServ1110-Win.zip` (KDX story) and `KDXClient1600-Win.zip` (GConsole story); sources `https://preterhuman.net/download/<name>` and `https://kdx.technowiki.info/downloads/<name>` (spec §10).
- Lab runs as user `openkdx` on Ubuntu 22.04, no sudo after `lab/setup.sh`; runner label `lab` (spec §7.5).
- Rust edition 2024; `cargo clippy -D warnings` and `cargo fmt --check` clean.

## Review Focus

1. A Haxial archive with the right name but tampered bytes: `fetch` must refuse it and leave the cache untouched. (Task 6)
2. `import` given a file that holds neither anchor string: a clear error, exit code 2, no cache write. (Task 6)
3. Partial download or network failure mid-fetch: no half-written `kdx.bin` left behind. (Task 6)
4. A commit that adds a Haxial story under a new filename: the blocklist must match by content hash, not by path. (Task 4)
5. The lab run script invoked when a previous sandbox is still alive: it must fail loudly rather than capture mixed traffic. (Task 9)

---

### Task 1: Rust toolchain on the Mac

**Files:** none in repo.

- [ ] **Step 1: Install rustup with the stable toolchain**

Run: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile default --default-toolchain stable`

- [ ] **Step 2: Verify**

Run: `source ~/.cargo/env && cargo --version && rustfmt --version && cargo clippy --version`
Expected: three version lines, cargo ≥ 1.85.

### Task 2: Licences, README, NOTICE, design doc, setup script

**Files:**
- Rename: `LICENSE` → `LICENSE-APACHE`
- Create: `LICENSE-MIT`, `NOTICE.md`, `.gitignore`
- Modify: `README.md`
- Already staged: `docs/superpowers/specs/2026-11-08-openkdx-design.md`, `lab/setup.sh`

- [ ] **Step 1: `git mv LICENSE LICENSE-APACHE`; write `LICENSE-MIT`** with the standard MIT text, copyright `2026 OpenKDX contributors`.

- [ ] **Step 2: Write `NOTICE.md`** with exactly these three points, one paragraph each: (a) OpenKDX contains no code, binaries, challenge-table text, artwork, or icons from Haxial Pty Ltd; users obtain Haxial's programs themselves and the tables are fetched or imported at runtime; (b) the OpenKDX mark is an original design inspired by KDX's visual vocabulary and is not derived from Haxial's artwork; (c) "KDX" is used descriptively to indicate protocol compatibility and OpenKDX is not affiliated with or endorsed by Haxial.

- [ ] **Step 3: Write `README.md`**: title, one-paragraph description from spec §1, a "Status: Phase 0" line, a table of the three programs (Client, Server, Tracker) with one line each, a Compatibility section naming Server 1.620 / Client 1.600 / Tracker 1.110, a Licence section, and a link to `NOTICE.md` and the design doc.

- [ ] **Step 4: Write `.gitignore`**: `/target`, `**/*.rs.bk`, `.DS_Store`, `/captures/*.pcapng` is **not** ignored (captures are committed), `/lab/run/`, `/lab/bin/`, `*.hap`, `*.lexe`, `*.exe`, `*.sit`, `*.zip`, `kdx.bin`, `hgc.bin`.

- [ ] **Step 4b: Write the logo SVGs** under `assets/logo/`: `mark.svg` (the fused X with the ring, lower half fill as a CSS variable `--role` defaulting to gold), `client.svg`, `server.svg`, `tracker.svg` (full lockups, colours gold `#ffd650`/`#d68c00`, green `#86f542`/`#1aa51a`, purple `#c66cff`/`#7a1fd0`, layout per spec §9 logo line: "OPEN" letterspaced in role colour above chrome "KDX", role word `Client`/`Server`/`Tracker` monospaced in role colour beneath, left-aligned). Hand-written SVG, no raster, no text converted from Haxial artwork. Reference the client lockup at the top of `README.md`.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "docs: add design, licences, NOTICE, lab setup script"
```

### Task 3: Cargo workspace skeleton

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`
- Create for each crate `crates/<name>/Cargo.toml`, `crates/<name>/README.md`, and `src/lib.rs` (libs) or `src/main.rs` (bins); `openkdx-tracker` has both.

**Interfaces:**
- Produces: workspace members and inter-crate dependency edges that later phases extend.

- [ ] **Step 1: Write the test** — `ci/check-deps.py`: reads every `crates/*/Cargo.toml`, asserts `openkdx-proto` has none of `tokio`, `reqwest`, `egui`, `eframe` in `[dependencies]`, and `openkdx-legacy` has none of `tokio`, `reqwest`, `egui`, `eframe`. Exit 1 with the offending crate and dep on failure. Also asserts every crate has `license = "MIT OR Apache-2.0"` and `edition = "2024"`.

- [ ] **Step 2: Run it** — `python3 ci/check-deps.py`. Expected: fails, no `crates/` directory.

- [ ] **Step 3: Write `Cargo.toml`** (workspace, `resolver = "3"`, `members = ["crates/*"]`, `[workspace.package]` with `edition = "2024"`, `license = "MIT OR Apache-2.0"`, `repository`, `rust-version = "1.85"`; `[workspace.dependencies]` for `tokio = { version = "1", features = ["full"] }`, `thiserror = "2"`, `anyhow = "1"`, `clap = { version = "4", features = ["derive"] }`, `sha2 = "0.10"`, `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "blocking"] }`, `zip = { version = "2", default-features = false, features = ["deflate"] }`, `directories = "6"`, `base64 = "0.22"`, `tempfile = "3"`, `hex = "0.4"`). `rust-toolchain.toml`: `channel = "stable"`, components `rustfmt`, `clippy`. `rustfmt.toml`: `edition = "2024"`. `clippy.toml`: empty with a comment.

- [ ] **Step 4: Create the eleven crates.** Each `Cargo.toml` uses `edition.workspace`, `license.workspace`, `repository.workspace`, `rust-version.workspace`, a one-line `description`. Dependency edges now: `openkdx-codec` → `openkdx-proto`; `openkdx-client` → `openkdx-proto`, `openkdx-codec`, `openkdx-table`; `openkdx-server` → `openkdx-proto`, `openkdx-codec`, `openkdx-legacy`, `openkdx-table`; `openkdx-tracker` → `openkdx-proto`, `openkdx-codec`, `openkdx-table`; `openkdx-ui` → `openkdx-legacy`; `openkdx-app` → `openkdx-client`, `openkdx-ui`; `openkdxd` → `openkdx-server`; `openkdx-dissect` → `openkdx-proto`, `openkdx-table`. Every `lib.rs` has `#![forbid(unsafe_code)]` and a doc comment; every `main.rs` prints `"<name> (OpenKDX) — not yet implemented"` and exits 0. Each README: one paragraph from spec §7.1's role column.

- [ ] **Step 5: Verify** — `python3 ci/check-deps.py && cargo build --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`. Expected: all succeed.

- [ ] **Step 6: Commit** — `git commit -am "build: eleven-crate workspace skeleton"` (after `git add -A`).

### Task 4: CI with the Haxial blocklist

**Files:**
- Create: `ci/haxial-blocklist.txt`, `ci/check-blocklist.py`, `ci/check-blocklist_test.py`, `.github/workflows/ci.yml`

- [ ] **Step 1: Write the failing test** `ci/check-blocklist_test.py` (plain `unittest`): creates a temp git repo, commits a file whose contents hash to a value present in the blocklist (generate: write bytes `b"x"`, compute its SHA-256, write a temporary blocklist containing that hash with label `test`), asserts `check_blocklist(repo, blocklist)` returns the path and label; a second case with a clean repo returns `[]`; a third case where the matching bytes are embedded at offset 100 inside a larger file also returns a hit (content scan, not whole-file hash only).

- [ ] **Step 2: Run** — `python3 -m unittest ci/check-blocklist_test.py`. Expected: ImportError.

- [ ] **Step 3: Write `ci/haxial-blocklist.txt`** — one entry per line, `sha256  length  label`: the KDX story and GConsole story hashes/lengths from Global Constraints; plus the SHA-256 of each archive in `vendor/MANIFEST.md` of scottanderson/kdx that we can verify (fetch the manifest, copy the hashes with label = archive filename). Comment lines start with `#`.

- [ ] **Step 4: Implement `check_blocklist(repo_root: Path, blocklist: Path) -> list[tuple[Path, str]]`** in `ci/check-blocklist.py`: for every file in `git ls-files`, compute whole-file SHA-256 and compare; additionally, for each blocklist entry with a length ≤ 1 MiB, slide a window of that length over the file (step 1 byte is too slow — use the first 32 bytes of the known story as an anchor instead: store `anchor_hex` as a fourth column, search for the anchor, hash `length` bytes from each hit). `main()` prints hits and exits 1 if any.

- [ ] **Step 5: Run test** — Expected: PASS.

- [ ] **Step 6: Write `.github/workflows/ci.yml`**: jobs `check` (ubuntu: `python3 ci/check-deps.py`, `python3 ci/check-blocklist.py`, `python3 -m unittest discover -s ci -p '*_test.py'`), `rust` (matrix ubuntu-latest, macos-latest, windows-latest: `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, fmt check, clippy `-D warnings`, `cargo test --workspace`), `lab` (`runs-on: [self-hosted, lab]`, `if: github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository`, runs `lab/ci-smoke.sh` — created in Task 9; until then the job runs `true`).

- [ ] **Step 7: Commit and push; confirm `check` and `rust` jobs are green on GitHub** (the `lab` job will queue until a runner exists; that is expected).

```bash
git add -A && git commit -m "ci: cross-platform build and Haxial content blocklist" && git push
```

### Task 5: `openkdx-table` — story extraction and cache

**Files:**
- Create: `crates/openkdx-table/src/lib.rs`, `src/story.rs`, `src/cache.rs`, `tests/story.rs`, `tests/cache.rs`

**Interfaces:**
- Produces:
  - `pub enum Story { Kdx, Gconsole }` with `fn anchor(&self) -> &'static [u8]`, `fn len(&self) -> usize` (1739 / 3193), `fn sha256(&self) -> [u8; 32]`, `fn file_name(&self) -> &'static str` (`kdx.bin` / `hgc.bin`).
  - `pub fn extract(story: Story, haystack: &[u8]) -> Result<Vec<u8>, TableError>` — finds anchor, takes `len` bytes, verifies SHA-256.
  - `pub struct Cache { dir: PathBuf }` with `Cache::default_dir() -> PathBuf` (`directories::ProjectDirs::from("org","openkdx","openkdx").cache_dir()/table`), `Cache::new(dir)`, `fn get(&self, story) -> Result<Option<Vec<u8>>, TableError>` (returns `None` if absent or hash mismatch), `fn put(&self, story, bytes: &[u8]) -> Result<(), TableError>` (verifies hash, writes to temp file in same dir then renames).
  - `pub fn from_env(story) -> Result<Option<Vec<u8>>, TableError>` reading `OPENKDX_KDX_TABLE` / `OPENKDX_HGC_TABLE` as base64.
  - `pub enum TableError { AnchorNotFound, HashMismatch { expected, actual }, Io(std::io::Error), Base64(..), Http(..), Zip(..) }` via `thiserror`.

- [ ] **Step 1: Write failing tests** in `tests/story.rs`: `extract_rejects_missing_anchor` (random bytes → `AnchorNotFound`); `extract_rejects_wrong_hash` (anchor followed by zeros → `HashMismatch`); `extract_accepts_real_story` — gated: reads the real table via a helper `fn real_table(Story) -> Option<Vec<u8>>` that calls `from_env` then `Cache::new(Cache::default_dir()).get`, returns early with `eprintln!("skipped: no table")` if `None`; asserts `extract(story, &[b"junk", table, b"junk"].concat()) == Ok(table)`. In `tests/cache.rs`: `put_then_get_roundtrip` using `tempfile::tempdir()` and a synthetic `Story`-shaped fixture — since `Story` hashes are fixed, test the cache with a `#[cfg(test)]`-only story variant is awkward; instead expose `Cache::put_unchecked` under `#[doc(hidden)]` for tests, and test `get` returns `None` after corrupting the file.

- [ ] **Step 2: Run** — `cargo test -p openkdx-table`. Expected: compile errors, symbols missing.

- [ ] **Step 3: Implement** `story.rs`, `cache.rs`, `lib.rs` per Interfaces. Dependencies: `sha2`, `thiserror`, `directories`, `base64`, `tempfile` (dev).

- [ ] **Step 4: Run** — Expected: all pass; the gated test prints `skipped` on a machine without tables.

- [ ] **Step 5: Commit** — `git commit -m "feat(table): story extraction, verification and cache"`.

### Task 6: `openkdx-table` — fetch, import, CLI

**Files:**
- Create: `crates/openkdx-table/src/fetch.rs`, `src/import.rs`, `src/bin/openkdx-table.rs`, `tests/fetch.rs`, `tests/import.rs`

**Interfaces:**
- Consumes: Task 5.
- Produces:
  - `pub struct Source { pub file_name: &'static str, pub urls: &'static [&'static str], pub sha256: [u8; 32], pub story: Story }` and `pub const SOURCES: &[Source]` (two entries; hashes taken from scottanderson/kdx `vendor/MANIFEST.md`, URLs: preterhuman then technowiki mirror).
  - `pub fn fetch(story: Story, cache: &Cache, client: &reqwest::blocking::Client) -> Result<Vec<u8>, TableError>` — tries each URL, downloads to memory, verifies archive SHA-256, opens zip, scans every entry with `extract`, `cache.put`.
  - `pub fn import(path: &Path, cache: &Cache) -> Result<Vec<Story>, TableError>` — if zip, scan entries; else scan the file; put every story found; returns which.
  - CLI `openkdx-table` with subcommands `fetch [--force] [kdx|hgc]`, `import <FILE>`, `path`, `verify`. Exit codes: 0 ok, 1 network/IO, 2 nothing found or hash mismatch.

- [ ] **Step 1: Write failing tests**: `tests/import.rs` — `import_from_zip_finds_story` (build a zip in a tempdir containing a fake entry; gated on real table like Task 5, otherwise asserts `Err(AnchorNotFound)` on a zip of junk); `import_junk_file_errors_without_writing_cache` (asserts cache dir stays empty — Review Focus 2). `tests/fetch.rs` — uses a local `tiny_http` or `std::net::TcpListener` serving a fixed body: `fetch_rejects_tampered_archive` (serve bytes whose hash ≠ `Source.sha256`, assert `HashMismatch` and cache empty — Review Focus 1); `fetch_leaves_no_partial_file_on_disconnect` (server closes after headers; assert no `kdx.bin*` in cache dir — Review Focus 3). Override `SOURCES` for tests via `fetch_from(sources: &[Source], ...)` with `fetch` as a thin wrapper.

- [ ] **Step 2: Run** — Expected: compile failures.

- [ ] **Step 3: Implement** `fetch.rs`, `import.rs`, the binary with `clap`.

- [ ] **Step 4: Run tests** — Expected: PASS. Then a manual check: `cargo run -p openkdx-table -- fetch` on the Mac. Expected: downloads two archives, prints `kdx.bin: ok (1739 bytes)` and `hgc.bin: ok (3193 bytes)`, and `cargo test -p openkdx-table` now runs the gated tests instead of skipping.

- [ ] **Step 5: Document** `docs/setup/challenge-tables.md`: why the tables exist, the three ways to get them, the env vars for CI, the exit codes.

- [ ] **Step 6: Commit** — `git commit -m "feat(table): fetch from mirrors, import from files, CLI"`.

### Task 7: Lab access and binaries on the Linux box

**Files:**
- Create: `lab/README.md`, `lab/fetch-binaries.sh`, `lab/MANIFEST.txt`

**Interfaces:**
- Produces: `~openkdx/haxial/bin/` populated and verified; `~openkdx/haxial/tables/{kdx.bin,hgc.bin}`.

- [ ] **Step 1: User action** — run `lab/setup.sh openkdx` as root on the box; generate an SSH key pair on the Mac (`ssh-keygen -t ed25519 -f ~/.ssh/openkdx_lab -C openkdx-lab`); user adds the public key to `~openkdx/.ssh/authorized_keys`. Record host alias `openkdx-lab` in `~/.ssh/config`.

- [ ] **Step 2: Verify** — `ssh openkdx-lab 'id; dumpcap -D | head -3; unshare -rn ip link show lo; wine --version; bwrap --version; cargo --version'`. Expected: uid for `openkdx`, interface list, `lo` shown, Wine ≥ 6, bwrap version, cargo version.

- [ ] **Step 3: Write `lab/MANIFEST.txt`** — `sha256  filename  url` for: `KDXServer1620-Lnx.zip`, `KDXTrackerServ1110-Lnx.zip`, `KDXClient-Lnx-beta5.zip`/`.tar.gz`, `KDXClient1600-Win.zip`, `KDXServer1600-Win.zip`, `KDXTrackerServ1110-Win.zip`, `AppearanceEdit1240-Win.zip`, `KDXSoundListEdit1000-Win.zip`. Hashes: compute from a fresh download on the Mac, cross-check the first six against scottanderson/kdx `vendor/MANIFEST.md`.

- [ ] **Step 4: Write `lab/fetch-binaries.sh`** — for each manifest line: skip if the file exists with the right hash; else `curl -fL` to a temp name, verify, move into `~/haxial/bin/`, unzip into `~/haxial/bin/<stem>/`; `chmod +x` any `*.lexe`. Fail on first mismatch. Then run `openkdx-table import` on `KDXTrackerServ1110-Win.zip` and `KDXClient1600-Win.zip` with `--cache ~/haxial/tables`.

- [ ] **Step 5: Run it on the box** — `ssh openkdx-lab 'cd ~/openkdx-src && git pull && lab/fetch-binaries.sh'` (first clone the repo there). Expected: eight `ok` lines, two table lines.

- [ ] **Step 6: Write `lab/README.md`** — the layout from `setup.sh`, how to reach the box, what is and isn't committed, the admin login used in the lab (`admin` / a password stored only in `~openkdx/haxial/lab.env`).

- [ ] **Step 7: Commit** — `git commit -m "lab: binary manifest and fetch script"`.

### Task 8: Sandbox runner for the original programs

**Files:**
- Create: `lab/kdx-lab` (Bash), `lab/lib/sandbox.sh`, `lab/lib/capture.sh`

**Interfaces:**
- Produces CLI: `lab/kdx-lab server start|stop|status`, `tracker start|stop`, `client start|stop [--win]`, `capture start <scenario>|stop`, `shell <server|tracker|client>`; state under `~/lab/run/<component>/`, captures to `~/captures/<scenario>.pcapng`.

- [ ] **Step 1: Write the check** `lab/kdx-lab selftest`: starts a throwaway `bwrap` with `--unshare-net`, asserts only `lo` is visible inside, asserts the host's `ip link` is unchanged, asserts `dumpcap -i lo -c 1 -w /tmp/x.pcapng` inside the namespace exits 0 within 5 s when a packet is sent to `127.0.0.1`. Also asserts `start` refuses when a pidfile exists and the process is alive (Review Focus 5).

- [ ] **Step 2: Run** — `ssh openkdx-lab 'lab/kdx-lab selftest'`. Expected: command not found.

- [ ] **Step 3: Implement.** One network namespace per lab session (`unshare -rn` + `ip link set lo up`), all components joined into it via `nsenter`-equivalent (`bwrap --unshare-net` cannot share; so create the netns once with `unshare -rnp --fork --mount-proc sleep infinity`, record its PID, and start each component with `nsenter -t $PID -n -U --preserve-credentials bwrap --ro-bind / / --bind ~/lab/run/$c /work --chdir /work ...`). Server runs `KDXServer.lexe` non-interactively after a first-run `expect` script answers the setup prompts from `lab.env`. Windows client under `xvfb-run wine KDXClient.exe`. Capture: `dumpcap -i lo -w ~/captures/$scenario.pcapng` inside the namespace, pidfile; `stop` sends SIGINT and waits.

- [ ] **Step 4: Run selftest** — Expected: `selftest: ok`.

- [ ] **Step 5: First real run** — `kdx-lab server start`; check `~/lab/run/server/server.log` contains `KDX Server started on 127.0.0.1:10700`. `kdx-lab server stop`.

- [ ] **Step 6: Commit** — `git commit -m "lab: sandboxed runner for the original KDX programs"`.

### Task 9: First captures and the CI smoke job

**Files:**
- Create: `captures/README.md`, `captures/<scenario>.pcapng` + `.md` for each scenario below, `lab/ci-smoke.sh`, `lab/scenarios/*.sh`

**Interfaces:**
- Produces: fixtures for Phase 1 (`captures/first-connect.pcapng` etc.).

- [ ] **Step 1: Write `lab/ci-smoke.sh`** — the test for the lab job: `kdx-lab selftest`, `kdx-lab server start`, wait for the `started on` log line, open a TCP connection to 127.0.0.1:10700 and send 8 junk bytes, assert the server log gains `The network protocol is unknown. (ID 4/113 @ TRN941)` within 5 s, `kdx-lab server stop`. Exit non-zero on any failure. Wire it into the `lab` job in `ci.yml`.

- [ ] **Step 2: Run on the box** — Expected: `smoke: ok`. Push; confirm the `lab` job goes green on GitHub.

- [ ] **Step 3: Scripted scenarios** in `lab/scenarios/`, each a Bash script that starts capture, runs components, performs steps (Linux client steps by `xdotool` where needed; otherwise documented manual GUI steps), stops capture, and writes `captures/<name>.md` with the steps and the frame count:
  - `first-connect`: Win client 1.600 logs in as admin to Server 1.620.
  - `guest-login`: empty login/password.
  - `chat-basic`: join public chat, send two lines, set topic.
  - `pm-basic`: two clients, one private message each way.
  - `news-basic`: list newsgroups, read one, post one.
  - `files-basic`: list base, download a 3-byte file, upload a 3-byte file.
  - `tracker-basic`: server registers with Tracker 1.110; client browses groups and servers.

- [ ] **Step 4: Run all seven; copy results into `captures/`.** Verify each `.pcapng` opens with `tshark -r` and has ≥ 10 frames; verify by hand that `first-connect` frame with the client HELLO contains `%TXP%KDX` after XOR (the key is the first 4 bytes; decode per spec §5.1) — write the decoded hello bytes into `captures/first-connect.md`.

- [ ] **Step 5: Write `captures/README.md`** — index table: scenario, programs and versions, date, frame count, what it shows.

- [ ] **Step 6: Commit** — `git commit -m "captures: first seven lab scenarios; lab smoke job"`.

### Task 10: Legacy data intake and docs scaffolding

**Files:**
- Create: `docs/formats/README.md`, `docs/formats/inventory.md`, `docs/protocol/README.md`, `CONTRIBUTING.md`, `lab/inventory.sh`

- [ ] **Step 1: User action** — copy old server dir, client dir, `.hap` files, sound lists to `~openkdx/haxial/legacy/` on the box.

- [ ] **Step 2: Write `lab/inventory.sh`** — walks `~/haxial/legacy`, prints `relative-path  size  sha256  first-8-bytes-hex` sorted, excluding the contents of `Bases/` (file names only, no hashes — they are the user's files). Output pasted into `docs/formats/inventory.md` with a header noting what each top-level directory is.

- [ ] **Step 3: Run it; write `inventory.md`.**

- [ ] **Step 4: Write `docs/protocol/README.md`** — states the reference spec URL and commit hash pinned today, how we tag evidence (`[O capture frames]`, `[S binary function]`, `[I]`), and an empty "Deltas from the reference" table. `docs/formats/README.md` — list of formats from spec §5.2 with status `unknown` for each and a link to the editors in `lab/MANIFEST.txt`.

- [ ] **Step 5: Write `CONTRIBUTING.md`** — the checks (`cargo fmt`, `clippy -D warnings`, `cargo test`, `python3 ci/check-*.py`), commit message convention (`type(scope): summary`), evidence-tag rule, and the legal rules: never commit Haxial material; Ghidra projects stay local; derived notes and addresses are fine.

- [ ] **Step 6: Commit and push** — `git commit -m "docs: format inventory, protocol deltas scaffold, contributing"`.

### Task 11: Phase 0 exit check

- [ ] **Step 1: Fresh-machine check** — in a clean temp dir on the Mac: `git clone`, `cargo test --workspace` (gated tests skip), `cargo run -p openkdx-table -- fetch`, `cargo test -p openkdx-table` (gated tests run). Expected: all green.
- [ ] **Step 2: CI check** — latest `main` has `check`, `rust` ×3, and `lab` all green.
- [ ] **Step 3: Record** — add a "Phase 0 complete: <date>, <commit>" line to `docs/superpowers/specs/2026-11-08-openkdx-design.md` §8 and commit.
