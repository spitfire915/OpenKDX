# Phase 1: Protocol Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `openkdx-proto`, `openkdx-codec`, and `openkdx-dissect` decode every byte of the seven lab captures and re-encode them byte-exact, with every message type 1–1107 and 4000–4201 typed; plus the two deferred Phase 0 minors (MSRV 1.88, feature-gated fetching in `openkdx-table`).

**Architecture:** `openkdx-proto` is pure, no-I/O Rust: a byte `Reader`/`Writer`, the handshake (XOR stream, `Hello`, `Proof`, `account_key`, challenge tables), the session layer (`Header`, scramble, Twofish-256 ECB `SessionCipher`, LCG padding), the UDP tracker `Registration`, and one typed struct per spec message implementing a `Payload` trait, gathered in a `Message` enum. `openkdx-codec` wraps the session layer in `tokio_util::codec` `Encoder`/`Decoder`s, including the streaming mode for 414/417 transfer data. `openkdx-dissect` is a library (pcap reading, TCP reassembly, connection decoding, byte-exact audit) plus a CLI; its integration tests over `captures/` are Phase 1's acceptance test.

**Tech Stack:** Rust 2024, MSRV 1.88. `twofish` 0.8, `sha1` 0.10 (RustCrypto), `thiserror` 2; `tokio-util` 0.7 (`codec`), `bytes` 1; `pcap-file` 2, `etherparse` 0.21, `clap` 4, `anyhow` 1, `hex` 0.4.

**Spec:** `docs/superpowers/specs/2026-11-08-openkdx-design.md` (§5.1, §7.1, §7.5, §8 item 1). **Reference protocol spec:** https://github.com/scottanderson/kdx/tree/main/docs/protocol, pinned at commit `db2c1481648a57e51aa16e353824fcae3a1d402e` (see `docs/protocol/README.md`). A local clone is useful: `git clone --depth 1 https://github.com/scottanderson/kdx /private/var/folders/wk/hs0yq57n5q9g5s8kb5ybhh400000gn/T/opencode/kdx-ref` (already present on this machine). Strip the evidence comments when reading: `sed -e 's/<!--[^>]*-->//g' <page>`.

## Global Constraints

- Rust stable, edition 2024, one Cargo workspace; `rust-version = "1.88"` after Task 1.
- `openkdx-proto` has **no I/O** and must not depend, directly or transitively, on tokio, reqwest, hyper, mio, `openkdx-table`, or any UI crate (`ci/check-deps.py` enforces this). Challenge tables reach it as `&[u8]`.
- Crypto from RustCrypto only (`twofish`, `sha1`); no OpenSSL.
- Every crate: `license = "MIT OR Apache-2.0"`, `#![forbid(unsafe_code)]`, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt` clean.
- **Never commit Haxial material:** no challenge-table bytes, not even 20-byte excerpts, in tests, fixtures, or docs. Tests needing a table take it from `openkdx_table::from_env(Story::Kdx)` or the cache and **skip (not fail)** when absent. `ci/check-blocklist.py` runs on every push.
- Captures are the ground truth; the reference spec is authoritative unless a capture contradicts it. Each discrepancy goes in the "Deltas" table of `docs/protocol/README.md` with an evidence tag `[O <capture> <frames>]`.
- Lab: `ssh openkdx-lab`; the Phase 0 scripts are in `~/openkdx-src` there (`git pull` first). Lab admin login `admin`, password `openkdx-lab`. The lab is sandboxed; never touch the user's own lab session (`lab/ci-smoke.sh` shows the lock protocol).
- Wire numbers are big-endian; `str` fields are raw bytes (`Vec<u8>`), never `String`; lengths count bytes.
- Byte-exact rule: every decoded payload re-encodes to exactly the bytes it came from, including `ignored_*`, `padding`, and `extra` bytes, which are kept as fields. Decoding fails on trailing bytes the struct does not account for (`DecodeError::Trailing`). This is how we find spec errors.
- Commit after every task with a conventional message; push to the branch `phase1/protocol-core`; open PR #2 with the openchamber `session.link` when the first commit is pushed.

## Review Focus

1. **Truncated payloads.** A client/server (or a bad capture) can send a payload shorter than the struct's fixed part; every `decode` must return `DecodeError::Truncated`, never panic or slice out of bounds. Pinned in Task 7 (prefix test over every captured payload) and Task 19.
2. **Oversized lengths.** A header whose `len` exceeds the connection cap (0x20000 server-side, 0x100000 client-side) must be rejected before any allocation of `len` bytes; bits 32–47 non-zero is also a rejection on a normal message. Pinned in Task 5 and Task 18.
3. **Zero txid or kind.** The original gives 0x40074; our `Header::decode` must reject them too, or the codec would accept garbage after a key mismatch. Pinned in Task 5.
4. **Non-UTF-8 text.** File names like `Erik\xd5s` are legal on the wire; decode must keep them as bytes and `Display` must be lossy, never fail. Pinned in Task 2 (`Reader::str`) and Task 12.
5. **Hello edge cases.** `pad_len`/`skipped_len` non-zero and `name_len > 31` must decode as the original does (skip the bytes, keep 31 bytes of name); a hello over 0x800 bytes is rejected. Pinned in Task 4.

---

## File structure

```
Cargo.toml                                  MSRV, new workspace deps, openkdx-dissect in workspace.dependencies
.github/workflows/ci.yml                    msrv job; lab job runs the capture tests
crates/openkdx-table/
  Cargo.toml                                [features] fetch = ["dep:reqwest"]; default = ["fetch"]
  src/lib.rs, src/fetch.rs, src/main.rs     cfg(feature = "fetch")
crates/openkdx-proto/src/
  lib.rs                                    pub mod wire, account, xor, handshake, challenge, session, registration, message_type, msg; re-exports
  wire.rs                                   Reader, Writer, DecodeError, pad4 helpers
  account.rs                                AccountKey, account_key(password)
  xor.rs                                    the 0x4878 handshake XOR stream
  challenge.rs                              ChallengeTable (default 31-byte table or a story), response(offset)
  handshake.rs                              Hello, Proof, proof_auth(), ProofStatus, SubVersion/Role
  session.rs                                Header, scramble/unscramble, SessionCipher (seal/open, data blocks, padding LCG)
  registration.rs                           UDP Registration: LCG XOR, CRC-32 variant, encode/decode
  message_type.rs                           MessageType(u32) with name table from messages/README.md
  msg/mod.rs                                Payload trait, Message enum, decode_message(kind, bytes), Status + error codes
  msg/common.rs                             Color, FilePath, PathName, DelimitedFilePath, UserRecord, ChatRecord, Time
  msg/session.rs    100–118                 msg/chat.rs 200–223, 300–301      msg/news.rs 600–614
  msg/files.rs      400–453                 msg/tracker.rs 700–721            msg/accounts.rs 502–551 + privileges
  msg/admin.rs      140–191                 msg/misc.rs 800–905, 1000–1003, 1100–1107   msg/gconsole.rs 4000–4201
crates/openkdx-codec/src/
  lib.rs, handshake.rs (HandshakeCodec), session.rs (SessionCodec, Event, Item)
crates/openkdx-dissect/src/
  lib.rs                                    pub mod capture, connection, audit, text
  capture.rs                                read pcap/pcapng → Vec<Frame>; link types; UDP datagrams and TCP flows (reassembly by seq)
  connection.rs                             classify by port; decode handshake; choose key; decode session into Vec<Record>
  audit.rs                                  re-encode each payload; Audit::Exact | Mismatch{offset} | Undecoded(DecodeError)
  text.rs                                   summaries: Display for records, hexdump, quoted bytes, time_1900
  main.rs                                   clap CLI
crates/openkdx-dissect/tests/captures.rs    the seven-capture acceptance test
captures/*.md, captures/README.md           re-recorded notes; "Messages" inventory per capture
docs/protocol/README.md                     deltas + confirmed claims from this phase
docs/superpowers/plans/2026-11-09-phase1-protocol-core.md   this plan
```

---

### Task 1: Workspace housekeeping — MSRV 1.88, dependencies, feature-gated fetching

**Files:**
- Create: `docs/superpowers/plans/2026-11-09-phase1-protocol-core.md` (copy of this file)
- Modify: `Cargo.toml` (workspace), `.github/workflows/ci.yml`
- Modify: `crates/openkdx-table/Cargo.toml`, `src/lib.rs`, `src/fetch.rs`, `src/main.rs`, `README.md`
- Modify: `crates/openkdx-dissect/Cargo.toml` (depend on table with `default-features = false`)

**Interfaces:**
- Produces: `openkdx-table` feature `fetch` (default on); without it, no `reqwest` in the dependency graph; `TableError::Http`/`HttpStatus` and `fetch`, `fetch_from`, `sources`, `Source` exist only with the feature.

- [ ] **Step 1: Branch and copy the plan**

```bash
git checkout -b phase1/protocol-core
mkdir -p docs/superpowers/plans && cp ~/.opencode/plan/2026-11-09-phase1-protocol-core.md docs/superpowers/plans/
```

- [ ] **Step 2: Write the failing feature-gate check**

Add to `ci/check-deps.py` a rule: `openkdx-table` built with `--no-default-features` must not resolve `reqwest`. Implement by running `cargo metadata --no-default-features --filter-platform ... -p openkdx-table`? `cargo metadata` has no per-package feature flags, so use `cargo tree -p openkdx-table --no-default-features -e normal --prefix none` and assert `reqwest` is absent from the output. Add the matching unit test to `ci/check-deps_test.py` (mock the subprocess output: one with `reqwest`, one without).

- [ ] **Step 3: Run it to see it fail**

Run: `python3 ci/check-deps.py`
Expected: exit 1, "openkdx-table without default features still depends on reqwest".

- [ ] **Step 4: Gate fetching**

In `crates/openkdx-table/Cargo.toml`: `reqwest = { workspace = true, optional = true }`; `[features] default = ["fetch"]`, `fetch = ["dep:reqwest"]`; `[[bin]] name = "openkdx-table", path = "src/main.rs", required-features = ["fetch"]`. In `lib.rs`: `#[cfg(feature = "fetch")] mod fetch;` and the matching `pub use`; `#[cfg(feature = "fetch")]` on the `HttpStatus` and `Http` variants. Move `tests/fetch.rs` behind `#![cfg(feature = "fetch")]`. Document the feature in the crate README. `openkdx-dissect/Cargo.toml`: `openkdx-table = { workspace = true, default-features = false }`.

- [ ] **Step 5: Raise the MSRV and add the dependencies**

`Cargo.toml`: `rust-version = "1.88"`; add to `[workspace.dependencies]`: `openkdx-dissect = { path = "crates/openkdx-dissect" }`, `openkdx-tracker`, `openkdxd`, `openkdx-app` paths for completeness, `twofish = "0.8"`, `sha1 = "0.10"`, `tokio-util = { version = "0.7", features = ["codec"] }`, `bytes = "1"`, `futures-util = "0.3"`, `pcap-file = "2"`, `etherparse = "0.21"`. In `ci.yml` add a job `msrv` (ubuntu, `dtolnay/rust-toolchain@1.88`, `cargo check --workspace --all-targets`).

- [ ] **Step 6: Verify**

Run: `python3 ci/check-deps.py && cargo build -p openkdx-table --no-default-features && cargo test -p openkdx-table && cargo clippy --workspace --all-targets -- -D warnings && for t in ci/*_test.py; do python3 -m unittest "$t"; done`
Expected: all pass; the no-default-features build has no `reqwest` in `cargo tree`.

- [ ] **Step 7: Commit and open the PR**

```bash
git add -A && git commit -m "chore: MSRV 1.88, feature-gate fetching in openkdx-table, Phase 1 deps and plan"
git push -u origin phase1/protocol-core
gh pr create --title "Phase 1: protocol core" --body "Implements docs/superpowers/plans/2026-11-09-phase1-protocol-core.md" --draft
```
Then link the PR to this session (`openchamber session.link`, kind `change`).

---

### Task 2: Re-record the four captures that start after login (lab)

Finding during planning: `chat-basic`, `news-basic`, `pm-basic`, and `files-basic` call `capture_begin` **after** `connect_to`, so they hold no handshake, and `files-basic`'s transfer connection cannot be decrypted at all: an attached connection's key is the 32-byte `session_key` at offset 0 of the parent's **105 Server Info** (reference `transfers.md`), which the capture missed. Moving `capture_begin` before the first `connect_to` makes every capture self-contained. Same scenarios, same seven files.

**Files:**
- Modify: `lab/scenarios/chat-basic.sh`, `news-basic.sh`, `pm-basic.sh`, `files-basic.sh` (move `capture_begin` above the first `connect_to`)
- Replace: `captures/{chat,news,pm,files}-basic.{pcapng,md}`; update frame counts in `captures/README.md`

- [ ] **Step 1: Edit the four scripts locally, commit, push; on the lab pull and run them**

```bash
ssh openkdx-lab 'cd ~/openkdx-src && git fetch && git checkout phase1/protocol-core && git pull && for s in chat-basic news-basic pm-basic files-basic; do lab/scenarios/$s.sh || exit 1; done'
```
Respect the lab lock as `lab/ci-smoke.sh` does; the scenario framework writes `captures/<name>.{pcapng,md}` under `~/openkdx-src/captures/`.

- [ ] **Step 2: Verify each new capture decodes fully with the reference dissector**

Run (in the kdx-ref clone, after `cargo build -p kdx-dissect`): `./target/debug/kdx-dissect -p openkdx-lab --audit captures/<name>.pcapng` for each new file copied back with `scp`.
Expected: no `stopped:` line; `files-basic` shows both connections, the second with name `"\x00\x00\x00\x00\x01"`-style attach login and `key: 105 Server Info of …`.

- [ ] **Step 3: Copy back, run the blocklist, commit**

```bash
scp 'openkdx-lab:~/openkdx-src/captures/*-basic.*' captures/
python3 ci/check-blocklist.py
git add lab/scenarios captures && git commit -m "captures: record the handshake in chat, news, pm, and files scenarios"
```
Update the `Frames` column of `captures/README.md` from the new `.md` files in the same commit.

---

### Task 3: `openkdx-proto` wire primitives

**Files:**
- Create: `crates/openkdx-proto/src/wire.rs`; Modify: `src/lib.rs`, `Cargo.toml` (deps: `thiserror`, `twofish`, `sha1`)

**Interfaces (Produces):**
```rust
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("truncated: needed {needed} more bytes at offset {offset}")] Truncated { offset: usize, needed: usize },
    #[error("{0}")] Invalid(&'static str),
    #[error("{0} trailing bytes")] Trailing(usize),
}
pub struct Reader<'a> { /* bytes, pos */ }
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self;
    pub fn pos(&self) -> usize;  pub fn remaining(&self) -> usize;  pub fn is_empty(&self) -> bool;
    pub fn u8(&mut self) -> Result<u8, DecodeError>;  pub fn u16(&mut self) -> Result<u16, _>;  pub fn u32(&mut self) -> Result<u32, _>;
    pub fn u48(&mut self) -> Result<u64, _>;  pub fn u64(&mut self) -> Result<u64, _>;
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], _>;
    pub fn bytes(&mut self, n: usize) -> Result<Vec<u8>, _>;   // also used for str fields
    pub fn rest(&mut self) -> Vec<u8>;
    pub fn pad4(&mut self, struct_start: usize) -> Result<Vec<u8>, _>;  // 0–3 bytes up to a multiple of 4 from struct_start; fewer if the input ends (keeps what is there)
    pub fn finish(self) -> Result<(), DecodeError>;  // Trailing(n) if bytes remain
}
pub struct Writer(Vec<u8>);  // u8/u16/u32/u48/u64/bytes/pad4(struct_start, &[u8]) mirror Reader; into_inner()
pub fn pad4_len(len: usize) -> usize;
```

- [ ] **Step 1: Write failing tests in `wire.rs` `#[cfg(test)]`**: `u48_reads_six_bytes_big_endian` (`00 00 00 01 00 02` → 0x0001_0000_0002), `truncated_reports_offset_and_need` (`Reader::new(&[1,2]).u32()` → `Truncated{offset:0, needed:2}`), `pad4_from_struct_start` (struct starting at 2 with 5 bytes read → pad of 1), `pad4_at_end_of_input_is_short` (input ends: returns what exists, no error), `finish_reports_trailing`, `writer_roundtrip_all_widths`.
- [ ] **Step 2: Run** `cargo test -p openkdx-proto` — expected: compile failure (module missing).
- [ ] **Step 3: Implement `wire.rs`** per the interface above.
- [ ] **Step 4: Run** `cargo test -p openkdx-proto` — expected: PASS.
- [ ] **Step 5: Commit** `feat(proto): wire reader and writer`.

---

### Task 4: `openkdx-proto` handshake — account key, XOR stream, challenge table, Hello, Proof

**Files:**
- Create: `src/account.rs`, `src/xor.rs`, `src/challenge.rs`, `src/handshake.rs`; Modify `src/lib.rs`
- Test: `crates/openkdx-proto/tests/handshake.rs`

**Interfaces (Produces):**
```rust
// account.rs
#[derive(Clone, Copy, PartialEq, Eq, Hash)] pub struct AccountKey(pub [u8; 32]);
impl AccountKey { pub const GUEST: AccountKey; pub fn from_password(password: &[u8]) -> Self; pub fn as_bytes(&self) -> &[u8; 32]; }
// xor.rs
pub fn xor_stream(key: u32, body: &mut [u8]);   // body.len() % 4 == 0; k = (k << 1).wrapping_add(0x4878) per word, word ^= k
// challenge.rs
pub const RESPONSE_LEN: usize = 20;
pub struct ChallengeTable { bytes: Vec<u8> }
impl ChallengeTable {
    pub const DEFAULT: &'static [u8] = b"All your base are belong to us!";   // 31 bytes, not Haxial prose
    pub fn new(bytes: Vec<u8>) -> Self;  pub fn default_table() -> Self;
    pub fn len(&self) -> u16;  pub fn max_offset(&self) -> u16;            // len - 0x14
    pub fn response(&self, offset: u16) -> Option<[u8; RESPONSE_LEN]>;    // None when offset + 0x14 > len
    pub fn verify(&self, offset: u16, response: &[u8; RESPONSE_LEN]) -> bool;
}
// handshake.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Role { Client = 0x10, Server = 0x20, Tracker = 0x30, GConsole = 0x40 }
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum SubProtocol { Kdx, Hgc }   // "%KDX", "%HGC"
pub struct Hello { pub sub_protocol: SubProtocol, pub version: u8, pub sub_version: u8, pub challenge: u16, pub nonce: [u8; 12], pub name: Vec<u8>,
                   pub pad_len: u8, pub skipped_len: u16, pub ignored: [u8; 4], pub padding: Vec<u8> }
impl Hello {
    pub const MAX_LEN: usize = 0x800;
    pub fn decode(bytes: &[u8]) -> Result<(Hello, usize), HandshakeError>;  // returns bytes consumed; needs the 4-byte key + 0x20 header to know the length; HandshakeError::Incomplete{needed} if short
    pub fn encode(&self, key: u32) -> Vec<u8>;
    pub fn role(&self) -> Option<Role>;
    pub fn wire_len(&self) -> usize;   // (name_len + pad_len + skipped_len + 0x27) & !3
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum ProofStatus { Ok = 0x10, UnknownProtocol = 2, TooOld = 3, TooNew = 4 }
pub struct Proof { pub auth: [u8; 20], pub response: [u8; 20], pub status: ProofStatus, pub ignored: [u8; 3] }
impl Proof {
    pub const LEN: usize = 44;
    pub fn decode(bytes: &[u8; 44]) -> Result<Proof, HandshakeError>;  pub fn encode(&self) -> [u8; 44];
    pub fn compute(key: &AccountKey, peer_nonce: &[u8; 12], table: &ChallengeTable, peer_challenge: u16) -> Option<Proof>;
    pub fn failure(peer_nonce: &[u8; 12], status: ProofStatus) -> Proof;   // auth zero except key word = peer_nonce[4..8] ^ 0x201f7790
}
pub fn proof_auth(key: &AccountKey, peer_nonce: &[u8; 12]) -> [u8; 20];
#[derive(Debug, thiserror::Error)] pub enum HandshakeError { Incomplete{needed: usize}, UnknownProtocol, TooOld, TooNew, TooLong, InvalidStatus(u8), Decode(#[from] DecodeError) }
```
Error codes as associated consts on `HandshakeError`: 0x40070 auth, 0x40071, 0x40072, 0x40073, 0x40074 (self-connect), 0x40075 (too long).

- [ ] **Step 1: Write failing tests** (`tests/handshake.rs`), with these vectors from `captures/first-connect.pcapng`, `guest-login.pcapng`, `tracker-basic.pcapng` (decoded during planning with the reference dissector; hex is the captured value):

```rust
#[test] fn account_key_of_lab_password() {
    assert_eq!(hex::encode(AccountKey::from_password(b"openkdx-lab").as_bytes()),
        "7720015b5397c4e1910d0edf6e98f7aec7cbdbed7720015b5397c4e177390992");
}
#[test] fn guest_key_is_the_fixed_constant() {
    assert_eq!(hex::encode(AccountKey::from_password(b"").as_bytes()),
        "da39a3eecaaebe6e5e6b4b0dba8e29ba3255bfef18a2b5c495601890afd80709");
    assert_eq!(AccountKey::from_password(b""), AccountKey::GUEST);
}
#[test] fn first_connect_client_proof_auth() {   // client proof uses the SERVER's nonce (frame 8)
    let key = AccountKey::from_password(b"openkdx-lab");
    let server_nonce = hex!("d51503a414c2987f0d832176");
    assert_eq!(hex::encode(proof_auth(&key, &server_nonce)), "040f3e1528378f64484b05f0e31595c1861ca170");
}
#[test] fn first_connect_server_proof_auth() {   // frame 9, client nonce 49f403a414c2980df0150354
    ... == "4d9fa57bbf71ff0f6a6c579a3cffcaacdfad97a9"
}
#[test] fn guest_login_proofs() { // key GUEST; client auth with server nonce 004203a414c343df30a44f86 == "b92d57c73ef822232ee4a944367d55676b1bf90f"; server auth with client nonce 58b203a414c343a9425e0150 == "4e1e98512e3e97bd88ecb5f6aec01ff0379a5cb7"
}
#[test] fn tracker_proofs_use_guest_key() { // server nonce 9baa03a414c8a6a6043cb0e6 → "1545f70da2502a2f3d8e22735c9cf5cd920c03d7"; client nonce 33ab03a414c8a6d9c2c5bc7d → "cd8334dca3f8cfdde2b634469ffceee61fd5ccf0"
}
#[test] fn xor_stream_round_trips_and_matches_first_word() { /* key k, one word w: w ^ ((k<<1)+0x4878) */ }
#[test] fn hello_encode_decode_round_trip_first_connect_values() {
    // sub KDX, version 1, sub_version 0x10, challenge 1037, nonce 49f403a414c2980df0150354, name "admin" → wire_len 44; server hello (0x20, challenge 883, nonce d51503a414c2987f0d832176, "") → 36
}
#[test] fn hello_skips_pad_and_truncates_name_to_31() { /* pad_len 3, skipped_len 2, name 40 bytes: decoded name is 31 bytes, consumed == wire_len */ }
#[test] fn hello_rejects_wrong_protocol_version_and_length() { /* "%TXQ" → UnknownProtocol; version 0 → TooOld; 2 → TooNew; wire_len > 0x800 → TooLong */ }
#[test] fn hello_role_from_sub_version() { /* 0x10 Client, 0x20 Server, 0x30 Tracker, HGC 0x40 GConsole, 0x50 None */ }
#[test] fn proof_round_trip_and_failure_shape() { /* encode/decode; failure(): auth[0..4] == nonce[4..8] ^ 0x201f7790 BE, rest zero */ }
#[test] fn challenge_table_bounds() { /* default table len 31: response(11) Some, response(12) None; verify */ }
#[test] fn challenge_response_matches_capture_when_table_available() {
    // skip unless openkdx_table::from_env(Story::Kdx) or Cache default has kdx.bin; then compare the
    // SHA-1 of table.response(883) and table.response(1037) with the digests of the first-connect
    // responses (frames 8, 9). No excerpt of the table goes in the repository, not even 20 bytes.
}
```
`openkdx-table` is allowed as a **dev**-dependency of proto for the skip-if-absent helper (check-deps looks at normal/build edges only); `hex` as dev-dep; a tiny `hex!` via `hex::decode` is fine.

- [ ] **Step 2: Run** `cargo test -p openkdx-proto --test handshake` — expected: compile failure.
- [ ] **Step 3: Implement** the four modules. `proof_auth`: input = `s[0..4] n[0..4] s[4..12] n[4..8] s[12..20] n[8..12] s[20..32]` (44 bytes), `d = SHA-1`, `auth = Twofish256(key).encrypt(d[0..16]) || d[16..20]`. `account_key`: `d = SHA-1(pw); d || d[0..8] || BE32(fnv1)` with `h = 1; h = (h * 0x01000193) ^ b` per byte; empty password → the fixed constant. `Hello::decode`: key = first u32; XOR the rest (whole words) with `xor_stream(key)`; parse; total length formula; `Hello::encode(key)` inverse (`pad_len`/`skipped_len` bytes written as zeros).
- [ ] **Step 4: Run** `cargo test -p openkdx-proto` — expected: PASS (the table test prints `skipped` if no table).
- [ ] **Step 5: Commit** `feat(proto): handshake, account key, challenge tables`.

---

### Task 5: `openkdx-proto` session layer — Header, scramble, SessionCipher

**Files:** Create `src/session.rs`; Modify `src/lib.rs`. Tests inline + `tests/session.rs`.

**Interfaces (Produces):**
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header { pub txid: u32, pub kind: u32, pub reply: bool, pub len: u64 /* u48 */, pub reserved: u8, pub flags_rest: u8 /* flags & !1 */ }
impl Header {
    pub const LEN: usize = 16;
    pub fn request(txid: u32, kind: u32, len: u64) -> Self;  pub fn reply(txid: u32, kind: u32, len: u64) -> Self;
    pub fn to_bytes(&self) -> [u8; 16];
    pub fn from_bytes(b: &[u8; 16], max_len: u64) -> Result<Header, SessionError>;   // SessionError::ZeroTxidOrKind (0x40074), LengthTooLong (0x40075) if len > max_len or bits 32..48 set
    pub fn from_bytes_unbounded(b: &[u8; 16]) -> Result<Header, SessionError>;      // transfer data headers: only the zero check
    pub fn padded_len(&self) -> usize;   // ceil(len/16)*16
}
pub fn scramble(h: &mut [u8; 16]);  pub fn unscramble(h: &mut [u8; 16]);   // the four rotate/XOR constants from framing.md
pub fn padding_word(txid: u32) -> u32;   // txid.wrapping_mul(0x41c64e6d).wrapping_add(0x3039)
pub struct SessionCipher { /* twofish::Twofish */ }
impl SessionCipher {
    pub fn new(key: &AccountKey) -> Self;
    pub fn seal(&self, header: Header, payload: &[u8]) -> Vec<u8>;       // header block (scrambled, encrypted) + payload blocks, last block padded with padding_word(txid) as repeated BE u32 words
    pub fn seal_header(&self, header: Header) -> [u8; 16];
    pub fn open_header(&self, block: &[u8; 16], max_len: u64) -> Result<Header, SessionError>;
    pub fn open_header_unbounded(&self, block: &[u8; 16]) -> Result<Header, SessionError>;
    pub fn open_payload(&self, header: &Header, blocks: &[u8]) -> Result<Vec<u8>, SessionError>;  // blocks.len() == header.padded_len(); returns len bytes
    pub fn decrypt_blocks(&self, data: &mut [u8]);  pub fn encrypt_blocks(&self, data: &mut [u8]);   // ECB, len % 16 == 0
}
pub const SERVER_RECV_CAP: u64 = 0x20000;  pub const CLIENT_RECV_CAP: u64 = 0x100000;  pub const TRACKER_RECV_CAP: u64 = 0x10000;
pub const TRANSFER_RECV_CAP: u64 = 0x8000; pub const DCC_RECV_CAP: u64 = 0x80000; pub const VIEW_DISPLAY_RECV_CAP: u64 = 0x1000000;
#[derive(Debug, thiserror::Error, PartialEq, Eq)] pub enum SessionError { ZeroTxidOrKind, LengthTooLong{len: u64, cap: u64}, BadBlockLength, ReservedNotZero }
```

- [ ] **Step 1: Write failing tests**: `scramble_then_unscramble_is_identity` (random words), `scramble_matches_formula_on_known_words` (h = [1,2,3,4] → w0 = rotl(2,13)^0x6e7dfd34 …), `header_round_trip`, `header_rejects_zero_txid_and_kind`, `header_rejects_len_over_cap_and_high_bits` (len 0x20001 with cap 0x20000; len 1<<32 with unbounded is Ok), `seal_pads_last_block_with_txid_lcg` (payload of 5 bytes, txid 7: decrypt the sealed payload block and check bytes 5..16 are `padding_word(7)` BE repeated, aligned to word boundaries from block start), `seal_open_round_trip_with_lab_key`, `open_payload_rejects_wrong_block_count`.
- [ ] **Step 2: Run** — expected: compile failure.
- [ ] **Step 3: Implement** `session.rs` using `twofish::Twofish` with `cipher::{KeyInit, BlockEncrypt, BlockDecrypt}`.
- [ ] **Step 4: Run** `cargo test -p openkdx-proto` — PASS.
- [ ] **Step 5: Commit** `feat(proto): session header, scramble, and Twofish framing`.

---

### Task 6: `openkdx-proto` UDP tracker registration

**Files:** Create `src/registration.rs`; test `tests/registration.rs`.

**Interfaces (Produces):**
```rust
pub struct Registration { pub flags: u16, pub server_id: [u8; 12], pub server_port: u32, pub users: u16,
    pub string0: Vec<u8>, pub login: Vec<u8>, pub name: Vec<u8>, pub description: Vec<u8>, pub string4: Vec<u8>,
    pub groups: [Vec<u8>; 6], pub string11: Vec<u8>, pub string12: Vec<u8>,
    pub ignored_1c: [u8; 8], pub ignored_26: [u8; 13], pub padding: Vec<u8> }
impl Registration {
    pub fn decode(datagram: &[u8]) -> Result<Registration, RegistrationError>;  // decrypts, checks len > 0x40, "%KDX", version 0x0010, lengths fit, checksum
    pub fn encode(&self) -> Vec<u8>;   // pads to 4 with self.padding (zeros if shorter), sets placeholder ca d3 1e cc, computes CRC, encrypts
}
pub fn udp_xor(data: &mut [u8]);     // s = 0xa5a16c4a; w ^= s; s = s*0x41d28485 + 0x322b; whole words only
pub fn udp_crc(data: &[u8]) -> u32;  // CRC-32 poly 0x04c11db7 MSB-first, init 0xdeadbeef, no final XOR
pub enum RegistrationError { TooShort, BadProtocol, BadVersion, BadLengths, BadChecksum{expected: u32, actual: u32}, Decode(DecodeError) }
```

- [ ] **Step 1: Failing tests**: `udp_crc_table_entry_one_is_polynomial` (CRC of a single 0x80 bit stream — simpler: build the table and assert `table[1] == 0x04c11db7`; expose `fn crc_table()` `pub(crate)` or test via a known vector computed by hand in Python during implementation and pinned), `udp_xor_is_an_involution`, `registration_round_trip_synthetic` (port 10700, 1 user, login `trk-login`, name `Lab Server`, group0 `Lab, Lab2` → encode → decode equal; encoded length 92), `registration_rejects_bad_checksum_and_short_datagram`.
- [ ] **Step 2–4**: fail, implement, pass.
- [ ] **Step 5: Commit** `feat(proto): tracker UDP registration`.

The byte-exact check against `tracker-basic` frame 1 (84 bytes) lands in Task 8's capture test.

---

### Task 7: `openkdx-proto` message framework — `MessageType`, `Payload`, `Message`, `Status`

**Files:** Create `src/message_type.rs`, `src/msg/mod.rs`, `src/msg/common.rs`; Modify `src/lib.rs`.

**Interfaces (Produces):**
```rust
// message_type.rs
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct MessageType(pub u32);
impl MessageType { pub const STATUS: MessageType = MessageType(1); /* one const per type, SCREAMING_SNAKE of the spec name, e.g. SET_IDENTITY = 100, DOWNLOAD_FILE_DATA = 414 */
    pub fn name(self) -> Option<&'static str>;  // exact names from messages/README.md, e.g. "Set Identity", "Download File (D)", "(unnamed; license report)"
    pub fn is_known(self) -> bool; }
impl fmt::Display for MessageType  // "100 Set Identity" or "9999 (unknown)"
// msg/mod.rs
pub trait Payload: Sized + fmt::Debug {
    const KIND: MessageType;
    fn decode(bytes: &[u8]) -> Result<Self, DecodeError>;   // must consume everything (Reader::finish)
    fn encode(&self, w: &mut Writer);
    fn to_bytes(&self) -> Vec<u8> { … }
}
pub struct Status { pub code: u32, pub ignored: Vec<u8> /* bytes after the code, if any */ }   // + pub fn error_name(code) -> Option<&'static str> from status.md's table
#[derive(Debug)] pub enum Message { Status(Status), /* one variant per typed payload, added by Tasks 9–17 */ Unknown { kind: MessageType, bytes: Vec<u8> } }
impl Message {
    pub fn decode(kind: MessageType, bytes: &[u8]) -> Result<Message, DecodeError>;   // unknown kinds → Ok(Unknown)
    pub fn kind(&self) -> MessageType;  pub fn encode(&self) -> Vec<u8>;
}
// msg/common.rs
pub struct Color { pub r: u8, pub g: u8, pub b: u8 }
pub struct PathName { pub ignored: u16, pub name: Vec<u8> }
pub struct FilePath { pub names: Vec<PathName> }  // encode: empty payload for root; decode(bytes) ; decode_delimited(&mut Reader) for DelimitedFilePath (root = 00 00)
pub struct Time1900(pub u64);  // ms since 1900-01-01; fn to_unix_ms(&self) -> i64
pub struct UserRecord { … every field of encodings.md's table, including ignored_05: [u8;5], ignored_1b: u8, extra: Vec<u8>, padding: Vec<u8> }  // decode(&mut Reader) / encode(&mut Writer) since it is embedded in lists
pub struct ChatRecord { … }  // same style
```
`Message::decode` is a `match kind.0 { 1 => Status, …, _ => Unknown }`; each later task adds its arms. Keep each struct's field order = wire order of lengths, then strings; every `ignored_*`/`padding` from the spec table is a field so re-encoding is exact.

- [ ] **Step 1: Failing tests** (`tests/msg_framework.rs`): `every_listed_type_has_a_name` (iterate a literal list of all type numbers from messages/README.md, 1…4201, assert `name().is_some()` and distinct names), `status_round_trip_and_error_name` (`00 04 00 71` → code 0x40071, name "unknown protocol" per status.md), `unknown_kind_round_trips_bytes`, `file_path_root_is_empty_and_delimited_root_is_two_zero_bytes`, `file_path_example_from_spec` (`00 01 00 00 0c "Uploads [UL]"`), `user_record_from_first_connect_frame_12_round_trips` — bytes: the 560-byte 101 payload; obtain them with the reference dissector's hexdump (`kdx-dissect -p openkdx-lab captures/first-connect.pcapng`) or via Task 8 once it exists; put them in `crates/openkdx-proto/tests/fixtures/first-connect-101.bin` (lab traffic, no Haxial material: the icon is the client's default 2-bpp bitmap drawn by Haxial's client — **check with the user before committing the icon bytes**; if in doubt, assert on the fields and lengths only and keep the fixture out). Decision default: keep fixture bytes out of proto; byte-exact tests happen in Task 8+ straight from the pcapng.
- [ ] **Step 2–4**: fail, implement, pass. Error-code table: transcribe `status.md#error-codes` fully.
- [ ] **Step 5: Commit** `feat(proto): message types, payload trait, status, common records`.

---

### Task 8: `openkdx-dissect` library and CLI — captures, flows, handshake, session records

**Files:** Create `crates/openkdx-dissect/src/{lib.rs,capture.rs,connection.rs,audit.rs,text.rs}`; rewrite `src/main.rs`; `Cargo.toml` deps: `openkdx-proto`, `openkdx-table` (no default features), `pcap-file`, `etherparse`, `clap`, `anyhow`, `hex`. Test: `tests/captures.rs`.

**Interfaces (Produces):**
```rust
// capture.rs
pub struct Frame { pub number: usize, pub ts: Option<Duration>, pub data: Vec<u8>, pub link: LinkType }
pub enum Transport { Tcp { seq: u32, flags: TcpFlags, payload: Vec<u8> }, Udp { payload: Vec<u8> } }
pub struct Packet { pub frame: usize, pub src: SocketAddr, pub dst: SocketAddr, pub transport: Transport }
pub fn read_capture(path: &Path) -> anyhow::Result<Vec<Frame>>;              // pcap and pcapng; link types Ethernet(1), Null(0), Loop(108), LinuxSll(113), LinuxSll2(276), Raw IP(101/228/229)
pub fn parse_packets(frames: &[Frame]) -> Vec<Packet>;
pub struct Stream { pub data: Vec<u8>, pub segments: Vec<(usize /*offset*/, usize /*frame*/)> }  impl Stream { pub fn frame_at(&self, offset: usize) -> usize; }
pub struct Flow { pub client: SocketAddr, pub server: SocketAddr, pub c2s: Stream, pub s2c: Stream, pub first_frame: usize }
pub fn tcp_flows(packets: &[Packet], server_ports: &Ports) -> Vec<Flow>;  // reassembly by seq, SYN or lowest-seq start, duplicates dropped; the side connecting to a server port is the client
pub fn udp_datagrams(packets: &[Packet], port: u16) -> Vec<(usize, SocketAddr, SocketAddr, Vec<u8>)>;
// connection.rs
pub struct Ports { pub dcc: u16, pub server: u16, pub tracker: u16, pub gconsole: u16 }  // defaults 10600/10700/10800/10900
pub enum Dir { ClientToServer, ServerToClient }
pub enum Record { Hello { dir, frame, hello: Hello }, Proof { dir, frame, proof: Proof, auth_ok: Option<bool>, challenge_ok: Option<bool> },
                  Message { dir, frame, header: Header, payload: Vec<u8>, message: Result<Message, DecodeError> },
                  DataHeader { dir, frame, header: Header }, Data { dir, frame, len: u64 },
                  Stopped { dir, frame, reason: String } }
pub struct KeySource { pub label: String, pub key: AccountKey }   // "--password #1", "empty password", "105 Server Info of 127.0.0.1:58652"
pub struct Connection { pub flow_index: usize, pub kind: ConnectionKind, pub client: SocketAddr, pub server: SocketAddr, pub key: Option<KeySource>, pub records: Vec<Record>, pub harvested_keys: Vec<KeySource> }
pub struct Options { pub ports: Ports, pub passwords: Vec<Vec<u8>>, pub table: Option<ChallengeTable>, pub audit: bool }
pub fn dissect_flow(flow: &Flow, opts: &Options, extra_keys: &[KeySource]) -> Connection;
pub fn dissect_capture(path: &Path, opts: &Options) -> anyhow::Result<Capture>;  // Capture { connections: Vec<Connection>, registrations: Vec<RegistrationRecord> }; processes flows in first-frame order so 105 keys harvested from a parent feed later attach logins
// audit.rs
pub enum Audit { Exact, Mismatch { offset: usize, expected: Vec<u8>, actual: Vec<u8> }, Undecoded(DecodeError), Unknown }
pub fn audit(message: &Result<Message, DecodeError>, payload: &[u8]) -> Audit;
```
Key choice per connection: try `extra_keys` (harvested 105 session keys), then each `--password`, then the empty password; the first whose `proof_auth(key, server_nonce)` equals the client proof's `auth` wins. On a tracker connection the client sends the guest key. Challenge checks run only when `opts.table` is `Some`. Session decode: read 16-byte header block, `open_header(…, cap by connection kind)`, then `padded_len` bytes, `open_payload`; kinds 414 and 417 are data headers: emit `DataHeader`, then skip `ceil(len/16)*16` bytes emitting `Data`. A decrypt failure or short stream ends with `Stopped`.

CLI (`main.rs`):
```
openkdx-dissect [-p PASSWORD]... [--table PATH] [--no-table] [--audit] [--hex] [--export DIR] [--server-port N] [--tracker-port N] [--dcc-port N] [--gconsole-port N] CAPTURE
```
Default table: `openkdx_table::from_env(Story::Kdx)` then `Cache::new(Cache::default_dir()).get(Story::Kdx)`, else none (prints a one-line note). Output: one block per connection, one line per record: `[frame] C->S txid 6f2c62e0 100 Set Identity len 558  nick "labuser" …` using `text::summary(&Message)`; `--hex` adds a hexdump; `--audit` appends `audit: exact` / `audit: MISMATCH at 0x…` / `audit: undecoded: …`. Exit code 2 if any `Stopped`, undecoded, or mismatch with `--audit`.

- [ ] **Step 1: Failing test** `tests/captures.rs`:
```rust
fn opts() -> Options { /* password "openkdx-lab", table from env/cache if present, audit true */ }
#[test] fn first_connect_decodes_completely() {
    let cap = dissect_capture(Path::new("../../captures/first-connect.pcapng"), &opts()).unwrap();
    assert_eq!(cap.connections.len(), 1);
    let c = &cap.connections[0];
    assert_eq!(c.key.as_ref().unwrap().label, "--password #1");
    assert!(c.records.iter().all(|r| !matches!(r, Record::Stopped{..})));
    // both proofs auth_ok == Some(true); challenge_ok == Some(true) when a table is present
    // kinds in order: [100, 101, 105, …] — fill in from the first run, cross-checked against sequences.md#login
}
#[test] fn guest_login_uses_the_guest_key() { /* key label "empty password" */ }
#[test] fn tracker_basic_registration_is_byte_exact() { /* registrations.len()==1, frame 1, len 84, decode→encode == datagram; name "New Server", port 10700, users 0 or 1 as captured */ }
#[test] fn every_capture_has_no_stopped_connection() { /* loop over the seven files; assert no Stopped and every TCP flow became a Connection */ }
```
- [ ] **Step 2: Run** `cargo test -p openkdx-dissect` — compile failure.
- [ ] **Step 3: Implement** the library and CLI. Loopback captures from the lab netns are Ethernet-framed; handle the other link types listed so macOS and `tcpdump -i any` captures also work.
- [ ] **Step 4: Run** the tests — PASS; then run `cargo run -p openkdx-dissect -- -p openkdx-lab --audit captures/first-connect.pcapng` and read the output: all payloads are `Unknown` for now (audit `unknown`), which is expected until Task 9.
- [ ] **Step 5: Commit** `feat(dissect): pcap reading, TCP reassembly, handshake and session decoding`.

---

### Task 9: Messages 100–118 (session) — the login sequence byte-exact

**Files:** Create `src/msg/session.rs`; Modify `src/msg/mod.rs` (`Message` arms), `crates/openkdx-dissect/tests/captures.rs`, `src/text.rs` (summaries).

**Interfaces (Produces):** one struct per message in `messages/session.md`, named by the spec's struct caption (`SetIdentity`, `UserList`, `UserChanged`, `UserJoined`, `UserDeparted`, `ServerInfo`, `ServerIcon`, `ServerGreeting`, `Ping`, `Pong`, `SetMiniUserIcon`, `MiniUserIconChanged`, `RequestGreeting` (112), `GetExtraServerInfo`, `ExtraServerInfo`, `GetMyAddress`, `YourAddress`, `Skpk`, `LicenseReport` (118)), fields and types per the spec tables, each `impl Payload`. `ServerInfo.session_key: [u8; 32]` is what Task 8's harvesting reads (`Connection::harvested_keys` gets `KeySource{label: "105 Server Info of <client addr>", key}`), and `ServerInfo.user_id` pairs with attach logins `\0` + BE u32.

- [ ] **Step 1: Failing tests**: in `tests/captures.rs` extend `first_connect_decodes_completely` and the guest test: every `Record::Message` has `message.is_ok()` and `audit == Audit::Exact`; the first 100 decodes to `SetIdentity{ user: UserRecord{ nick: b"labuser", client_port: 10600, background_color: ff cc 00, … } }`; the 105 has `name == b"New Server"`, `version == 0x0654`. Unit tests in `session.rs`: synthetic round trip for every struct (`fn round_trip<T: Payload + PartialEq>(t: T)` helper in `msg/mod.rs` tests → move to `src/msg/test_util.rs` under `#[cfg(test)]`), and `set_identity_decodes_without_padding_and_user_list_with_it` (565 vs 568 bytes behaviour: `padding` empty vs 3 bytes).
- [ ] **Step 2: Run** — fail.
- [ ] **Step 3: Implement** the structs. Where the spec gives example bytes "as seen", use them as additional vectors.
- [ ] **Step 4: Run** `cargo test --workspace` — PASS. Run the CLI with `--audit` on first-connect and guest-login: every line `audit: exact`.
- [ ] **Step 5: Commit** `feat(proto): session messages 100–118; login captures byte-exact`.

---

### Task 10: Messages 200–223 (chat) and 300–301 (private messages)

**Files:** Create `src/msg/chat.rs`; Modify `mod.rs`, dissect `text.rs`, `tests/captures.rs`.

**Interfaces:** structs per `chat.md` and `private-messages.md` (`CreatePrivateChat`, …, `ChatWindowInfo` with its out-of-order string lengths, `SendChatText`, `ReceiveChatText`, `SetChatTopic`, `ChatTopicChanged`, `ChatList`, `ChatInfo`, `SetChat`, `ChatDccAddresses`, `SendPrivateMessage`, `IncomingPrivateMessage`, …), reusing `ChatRecord`/`UserRecord`.

- [ ] **Step 1: Failing tests**: `chat_basic_is_byte_exact` and `pm_basic_is_byte_exact` in `tests/captures.rs` (no Stopped, all Exact; kinds pinned in order after the first run; `ReceiveChatText` for `hello from the lab` and the `/me waves` action; the topic `lab topic`; `IncomingPrivateMessage` text `hi from admin` and `hello back`). Synthetic round trips for every struct, including `ChatWindowInfo` with all strings non-empty.
- [ ] **Steps 2–4**: fail, implement, pass.
- [ ] **Step 5: Commit** `feat(proto): chat and private message payloads; chat and pm captures byte-exact`.

---

### Task 11: Messages 600–614 (news)

**Files:** Create `src/msg/news.rs`; Modify as before.

- [ ] **Step 1: Failing tests**: `news_basic_is_byte_exact` (kinds pinned; `PostNewsMessage{ subject: b"Lab post", body: b"Posted from the lab scenario." }`; the 610 for the posted message round-trips with its `Time1900`), synthetic round trips for all 15 structs.
- [ ] **Steps 2–4**: fail, implement, pass.
- [ ] **Step 5: Commit** `feat(proto): news payloads; news capture byte-exact`.

---

### Task 12: Messages 400–453 (files, transfers, volumes, search)

**Files:** Create `src/msg/files.rs`; Modify as before; dissect `connection.rs` (414/417 data handling already generic; summaries show fork sizes).

**Interfaces:** structs per `files.md`, including `FileList` entries (`FileEntry` with its uninitialised padding kept), `DownloadFile`, `DownloadFileReply`, `DownloadFileData` (the data header payload struct: forks and sizes), `UploadFile*`, `VolumeInfo`, `FileSearch*`, `GenerateCatalog*`.

- [ ] **Step 1: Failing tests**: `files_basic_is_byte_exact`: two connections; the second's key label starts with `105 Server Info of`; the first has 400/401 twice with `FileList` naming `Admin Dropbox`, `tiny.txt` (size 3), `Uploads`; the second has 412, 413, a 414 `DataHeader` with `len` covering 3 bytes of data, and `Record::Data{len: 3}`. Review Focus 4: `file_name_with_invalid_utf8_round_trips` (entry name `Erik\xd5s`; `text::quoted` renders `Erik\xd5s`). Synthetic round trips for all structs.
- [ ] **Steps 2–4**: fail, implement, pass. If the 401 entry layout disagrees with the spec, trust the bytes and add a row to the Deltas table in `docs/protocol/README.md`.
- [ ] **Step 5: Commit** `feat(proto): file, transfer, volume, and search payloads; files capture byte-exact`.

---

### Task 13: Messages 700–721 (tracker over TCP)

**Files:** Create `src/msg/tracker.rs`.

- [ ] **Step 1: Failing tests**: `tracker_basic_is_byte_exact`: connection kind Tracker, key `empty password`, kinds `[700, 701, 702, 703]`, `TrackerGroupList` with 8 groups `Business, Chat, Education, Games, General, Macintosh, Trackers, Windows`, `GetTrackerServerList{ group: b"General" }`, `TrackerServerList` with one server `New Server` at `127.0.0.1:10700`. Synthetic round trips incl. 711/713/721 settings and statistics.
- [ ] **Steps 2–4**: fail, implement, pass.
- [ ] **Step 5: Commit** `feat(proto): tracker payloads; tracker capture byte-exact`.

---

### Task 14: Messages 502–551 (accounts) and privileges

**Files:** Create `src/msg/accounts.rs` (+ `Privileges` bitflags-by-hand: `pub struct Privileges(pub [u8; N])` with named `const` bit accessors per `privileges.md`; no `bitflags` crate).

- [ ] **Step 1: Failing tests**: synthetic round trips for all structs; `privilege_bits_match_spec` for at least five named bits taken from `privileges.md`; any example bytes in `accounts.md` as vectors. No capture covers accounts yet (`captures/README.md` says so); say that in the test file header.
- [ ] **Steps 2–4**; **Step 5: Commit** `feat(proto): account, class, and privilege payloads`.

---

### Task 15: Messages 140–143 (monitoring) and 150–191 (administration, server settings, history)

**Files:** Create `src/msg/admin.rs`.

- [ ] **Step 1: Failing tests**: synthetic round trips for every struct, including `ServerSettingsInfo`/`SetServerSettings` (the largest struct; every field from the spec table, allow/deny list entries with expiries) and the history record kinds 184–191 as an enum `HistoryRecord` inside 181/183.
- [ ] **Steps 2–4**; **Step 5: Commit** `feat(proto): monitoring and administration payloads`.

---

### Task 16: Messages 800–804, 900–905, 1000–1003, 1100–1107

**Files:** Create `src/msg/misc.rs` (processes, view display, voice, folder access).

- [ ] **Step 1: Failing tests**: synthetic round trips for every struct; 903 `ViewDisplayUpdate` keeps the pixel data as bytes (format unknown per the design spec §5.1); 1003 `VoiceData` keeps bytes.
- [ ] **Steps 2–4**; **Step 5: Commit** `feat(proto): process, view display, voice, and folder access payloads`.

---

### Task 17: GConsole — `%HGC` handshake and messages 4000–4201

**Files:** Create `src/msg/gconsole.rs`; Modify `src/handshake.rs` (nothing new on the wire: `SubProtocol::Hgc`, `Role::GConsole` exist since Task 4) and `src/challenge.rs` docs: the client answers an `%HGC` peer's challenge from the **GConsole** story but picked its own challenge from the KDX story's length (`handshake.md#challenge-tables`).

- [ ] **Step 1: Failing tests**: synthetic round trips for 4000, 4001 (draw records as an enum per `gconsole.md`), 4200, 4201; `hgc_hello_round_trip` (sub_protocol Hgc, sub_version 0x40, role GConsole); `challenge_table_switch_rule`: a helper `pub fn table_for_peer(peer: &Hello, kdx: &ChallengeTable, hgc: Option<&ChallengeTable>) -> Result<&ChallengeTable, HandshakeError>` returns `hgc` for an `%HGC` 0x40 hello and an error if `hgc` is `None`.
- [ ] **Steps 2–4**; **Step 5: Commit** `feat(proto): GConsole handshake rule and messages 4000–4201`.

---

### Task 18: `openkdx-codec` — `HandshakeCodec` and `SessionCodec`

**Files:** Create `crates/openkdx-codec/src/{handshake.rs,session.rs}`; rewrite `lib.rs`; `Cargo.toml` deps: `openkdx-proto`, `tokio-util`, `bytes`, `thiserror`; dev-deps: `tokio` (rt, macros, io-util), `futures-util` (sink, stream), `openkdx-dissect` (to read captured streams). Test: `tests/session.rs`, `tests/handshake.rs`.

**Interfaces (Produces):**
```rust
pub enum HandshakeFrame { Hello(Hello), Proof(Proof) }
pub struct HandshakeCodec { expect: Expect /* Hello | Proof | Done */, pub hello_key: u32 /* XOR key used when encoding our hello; random by default via a caller-supplied value */ }
impl Decoder for HandshakeCodec { type Item = HandshakeFrame; type Error = CodecError; }    // Hello::decode gives Incomplete{needed} → wait; after Hello, expects exactly 44 bytes
impl Encoder<HandshakeFrame> for HandshakeCodec;
pub enum Event { Message { header: Header, payload: Bytes }, DataStart(Header), Data(Bytes), DataEnd }
pub enum Item { Message { header: Header, payload: Bytes }, DataHeader(Header), Data(Bytes) }   // Data chunks: any length; the codec buffers to 16-byte blocks and pads the final block (tracked from DataHeader.len) with padding_word(txid)
pub struct SessionCodec { cipher: SessionCipher, max_len: u64, state: State /* Header | Payload{header} | Data{header, remaining} */, data_kinds: [u32; 2] /* 414, 417 */ }
impl SessionCodec { pub fn new(key: &AccountKey, max_len: u64) -> Self; pub fn into_parts(self) -> (SessionCipher, u64); }
impl Decoder for SessionCodec { type Item = Event; type Error = CodecError; }
impl Encoder<Item> for SessionCodec;
#[derive(Debug, thiserror::Error)] pub enum CodecError { Session(#[from] SessionError), Handshake(#[from] HandshakeError), Io(#[from] std::io::Error), DataOverrun{ sent: u64, declared: u64 }, Protocol(&'static str) }
```

- [ ] **Step 1: Failing tests**: `session_codec_decodes_first_connect_streams` (load both streams of flow 0 with `openkdx_dissect::capture::{read_capture, parse_packets, tcp_flows}`, skip the handshake bytes using `Hello::decode`'s consumed length + 44, feed the rest through `FramedRead` in 1-byte, 7-byte, and whole-buffer chunkings; the event sequence equals Task 8's `Record::Message` list), `session_codec_encodes_back_byte_exact` (encode each decoded message with `Item::Message` and compare with the original ciphertext bytes — works because padding is deterministic from txid), `transfer_data_mode_round_trip` (files-basic transfer connection: `DataStart`, `Data` totalling 3 bytes, `DataEnd`; and encoding `Item::DataHeader` + `Item::Data(b"abc")` yields the captured blocks), `rejects_len_over_cap_before_buffering` (Review Focus 2: a sealed header with len 0x20001 at cap 0x20000 → `CodecError::Session(LengthTooLong)` immediately with only 16 bytes fed), `handshake_codec_waits_for_incomplete_hello_then_proof`.
- [ ] **Step 2: Run** — compile failure.
- [ ] **Step 3: Implement.** The decoder must never `reserve` more than `padded_len` of a header already validated against `max_len`.
- [ ] **Step 4: Run** `cargo test -p openkdx-codec` — PASS.
- [ ] **Step 5: Commit** `feat(codec): tokio-util codecs for the handshake and the session, with transfer data mode`.

---

### Task 19: Dissector finish — strict audit over all seven captures, `--export`, robustness, lab CI

**Files:** Modify `crates/openkdx-dissect/src/{main.rs,connection.rs,audit.rs}`, `tests/captures.rs`, `README.md`; `.github/workflows/ci.yml` (lab job); `captures/README.md`, `captures/*.md`; `docs/protocol/README.md`.

- [ ] **Step 1: Failing tests**:
  - `all_seven_captures_are_byte_exact`: for each file, every connection has a key, no `Stopped`, every `Record::Message` is `Ok` and `Audit::Exact`, every registration re-encodes exactly. (Most pass already; this is the phase gate.)
  - `every_payload_prefix_decodes_without_panic` (Review Focus 1): for every captured `(kind, payload)`, for every `n in 0..payload.len()`, `Message::decode(kind, &payload[..n])` returns `Err` or `Ok` — the test passes if it finishes (a panic fails it).
  - `export_writes_both_streams` (tempdir; files `<client port>.c2s.bin` and `.s2c.bin` with the reassembled plaintext-free raw TCP bytes).
  - `inventory_matches_capture_notes`: each `captures/<name>.md` gets a `## Messages` section listing `frame  dir  kind name` lines; the test regenerates that text from the capture with `text::inventory(&Capture)` and asserts equality, so the notes can never drift from the bytes.
- [ ] **Step 2: Run** — fail (missing sections/flags).
- [ ] **Step 3: Implement**: `--export`, `text::inventory`, and a `openkdx-dissect --inventory` flag printing exactly that section; append the section to the seven `.md` files with it. Update `captures/README.md` ("Decoding" paragraph: the tool exists; how to run with the password and table). In `docs/protocol/README.md`, add to "Confirmed so far": the session key in 105 drives attach logins `[O files-basic <frames>]`, the UDP registration layout and checksum `[O tracker-basic 1]`, the header scramble and padding `[O first-connect <frames>]`; add any Deltas found in Tasks 9–13. CI: in the `lab` job, after the smoke test, run `cargo test -p openkdx-dissect -p openkdx-proto -p openkdx-codec` with `OPENKDX_KDX_TABLE="$(base64 -w0 ~/.cache/openkdx/table/kdx.bin)"` — verify the real cache path on the lab with `cargo run -p openkdx-table -- path` and use what it prints — so the challenge checks run somewhere in CI.
- [ ] **Step 4: Run** `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` and `python3 ci/check-blocklist.py` and `python3 ci/check-deps.py` — all pass. Run `openkdx-dissect --audit` by hand on each capture and read every line once.
- [ ] **Step 5: Commit** `feat(dissect): audit, export, inventory; captures documented; lab CI runs the protocol tests`.

---

### Task 20: Phase close-out

- [ ] **Step 1:** `docs/superpowers/specs/2026-11-08-openkdx-design.md`: under §8 add "**Phase 1 complete:** <date>, branch `phase1/protocol-core` (PR #2)…" in the style of the Phase 0 line. Crate READMEs for proto, codec, dissect describe what exists (short).
- [ ] **Step 2:** Mark the PR ready (`gh pr ready`), confirm all CI jobs (check, rust ×3, msrv, lab) are green with `gh pr checks --watch`.
- [ ] **Step 3: Commit** `docs: record Phase 1 completion`. Hand over for the whole-branch review (superpowers:requesting-code-review), then merge per superpowers:finishing-a-development-branch.

---

## Self-review notes

- **Spec coverage:** §5.1 handshake/crypto (T4, T5), challenge-table-at-runtime rule (T4 test skip rule, Global Constraints), ports (T8 `Ports`), every message type (T7, T9–T17), UDP (T6), GConsole handshake (T17), §7.1 crate responsibilities (file structure), §7.5 "every claim tagged with evidence" (T19 docs step), §8 item 1 "decode every capture byte-exact; re-encode byte-exact" (T19 gate), deferred minors (T1). Not in scope and deliberately absent: voice codec, icon bitmap format, view-display pixel format (kept as bytes), 0.9x handshake (not needed for 1.x compatibility).
- **Type consistency:** `AccountKey`, `ChallengeTable`, `Hello`, `Proof`, `Header`, `SessionCipher`, `Payload`, `Message`, `Record`, `Audit`, `Options` are named identically across tasks; `Role`/`SubProtocol` from T4 are what T8 and T17 use.
- **Proportion:** interface blocks give signatures and test names; struct field lists defer to the spec tables by page, which the implementer must read anyway.
