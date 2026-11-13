# Phase 2: Client Library + CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `openkdx-client`, an async library that logs in to a KDX 1.x server and does chat, PM, news, files (list, get, put, resume), accounts/classes/folder access, administration, and tracker browse; a thin `openkdx` CLI over it; differential tests and eight new captures against KDX Server 1.620 in the lab.

**Architecture:** One `Connection` actor per TCP connection (tokio task owning `Framed<TcpStream, SessionCodec>`): requests are correlated by txid + reply flag, everything else is an `Event` on a broadcast channel. `Session` = logged-in connection plus the login state (105 Server Info, user list). `Transfer` is a second connection keyed by the 105 session key that drives one download or upload at a time directly on its `Framed`, using the codec's data mode. The CLI is a clap binary with one subcommand per function plus `run` (many commands on one session, read from stdin), which the lab scenarios drive.

**Tech Stack:** Rust 2024, MSRV 1.88; tokio, tokio-util codecs (Phase 1), `rand` 0.9 (nonce, hello key, txid seed), clap 4, serde/serde_json for `--json`; lab: bash scenarios, `openkdx-dissect`.

**Spec:** `docs/superpowers/specs/2026-11-08-openkdx-design.md` §8 item 2, §5.1, §7.1, §7.5, §9. Protocol reference: scottanderson/kdx `docs/protocol` (pinned `db2c1481`; local clone `/private/var/folders/wk/hs0yq57n5q9g5s8kb5ybhh400000gn/T/opencode/kdx-ref`, re-clone with `git clone https://github.com/scottanderson/kdx` if gone) plus our deltas in `docs/protocol/README.md`.

## Global Constraints

- Rust stable, edition 2024, `rust-version = "1.88"`, one workspace; every crate `#![forbid(unsafe_code)]`, MIT OR Apache-2.0 (`ci/check-deps.py` checks edition + licence; `openkdx-proto` must stay free of tokio/network deps).
- Crypto only `twofish`, `sha1` from RustCrypto, via `openkdx-proto`; no new crypto deps.
- Challenge tables are never committed; tests read them from `OPENKDX_KDX_TABLE` (base64) or the `openkdx-table` cache and **skip** (print `skipped: …`) when absent. The lab job exports `OPENKDX_KDX_TABLE` (`.github/workflows/ci.yml` lines 78–83).
- Wire identity (spec §9): the client is a registered Client 1.600: hello `version 1, sub_version 0x10`; 100 Set Identity with `client_port 10600`; then `118 LicenseReport { value: 1 }`. Nothing in the handshake depends on Haxial's registration data.
- Strings on the wire are `Vec<u8>`; the CLI prints them lossily (`String::from_utf8_lossy`) and never panics on non-UTF-8.
- Lab: `ssh openkdx-lab`; repo at `~/openkdx-src` (`git pull` first); cargo in `~/.cargo/bin`; admin `admin` / `openkdx-lab`; everything that starts the server goes through `lab/kdx-lab` and takes `$RUN/lab.lock` with `flock -n` as `lab/ci-smoke.sh` does. **Flood ban:** Server 1.620 bans an address after 30 accepted connections in 300 s (reference `sequences.md` "Flood ban"), so each lab test run must stay under 25 connections per server start, or restart the server.
- Branch `phase2/client-cli`, PR #3 (open via the GitHub API with `git credential fill`; no `gh`). Commit after every task.
- The CLI binary is named `openkdx` (spec §8). Spec §5.3 reserves `openkdx` for the GUI client later; when Phase 7 ships `openkdx-app` as `openkdx`, this binary becomes `openkdx-cli`. Record that in the crate README.
- Deferred Phase 1 minors: fold in the `Hello` byte-exactness fix (Task 15) because the client encodes hellos; `Draw::encode` clamping stays deferred (GConsole, Phase 10) and is listed in `docs/protocol/README.md`'s known-inexact note.

## Review Focus

1. Wrong password or unknown login: the server answers with a failure Proof (status ≠ Ok) or closes; the client must return `ClientError::LoginFailed` within the timeout, not hang. Test: Task 2 `handshake_wrong_password_is_login_failed` (mock) and Task 13 `lab_login_wrong_password`.
2. The server closes the connection or sends 151 Disconnect Message while requests are pending: every pending request fails with `ClientError::Disconnected`, and the CLI prints the 151 message. Test: Task 3 `pending_requests_fail_when_peer_closes`; Task 13 `lab_disconnect_from_victim_side`.
3. A resumed download where the local file already holds the whole fork (server sends a 414 of length 0): the local file is left unchanged, not truncated. Test: Task 9 `download_resume_at_end_leaves_file_intact` (mock).
4. Error replies: the server answers a request with 1 Status (reply flag, `code` ≥ 0x10000) instead of the expected type; `request_as` must map this to `ClientError::Server { code, name }`, not a decode error. Test: Task 3 `status_reply_becomes_server_error`.
5. File paths and names with non-UTF-8 bytes or a `/` in CLI arguments: `parse_path("a/b c/d.txt")` gives three names; printing a listing with byte `0xA5` in a name does not panic. Test: Task 8 `parse_path_splits_on_slash`, `format_listing_is_lossy`.

---

## File structure

```
crates/openkdx-client/
  Cargo.toml                 + rand, tokio-util, futures-util, bytes, thiserror
  src/lib.rs                 pub use of everything below
  src/error.rs               ClientError
  src/table.rs               load_kdx_table()
  src/identity.rs            Identity → UserRecord
  src/handshake.rs           client_handshake(): Hello/Proof over any AsyncRead+AsyncWrite
  src/txid.rs                TxidSource
  src/connection.rs          Connection actor, Event, request/reply
  src/session.rs             Session::connect, login state, users, ping; re-exports area impls
  src/chat.rs  pm.rs  news.rs  files.rs  accounts.rs  admin.rs   impl Session blocks
  src/path.rs                parse_path / FilePath helpers
  src/transfer.rs            Transfer: download/upload with resume
  src/tracker.rs             Tracker::connect, groups, servers
  tests/common/mock.rs       MockServer (in-process KDX server on a TcpListener)
  tests/handshake.rs connection.rs session.rs transfer.rs    mock-based
  tests/lab.rs               differential tests; skip unless OPENKDX_LAB_SERVER is set
crates/openkdx-cli/
  Cargo.toml                 [[bin]] name = "openkdx"
  src/main.rs                clap tree, exit codes
  src/args.rs                Cli / Command enums (one enum reused by `run`)
  src/out.rs                 text and --json printers
  src/cmd/{session,chat,pm,news,files,accounts,admin,tracker}.rs
  README.md
lab/differential.sh          lock, start server+tracker, run tests/lab.rs inside the namespace
lab/lib/scenario.sh          + cli()/cli_bg() helpers, PROGRAMS for the Rust client
lab/scenarios/{upload-resume,accounts-classes,folder-access,server-settings,broadcast,disconnect-ban,server-history,connection-monitor}.sh
captures/<same eight>.pcapng + .md; captures/README.md table
crates/openkdx-dissect/tests/captures.rs   eight new entries
docs/protocol/README.md      new deltas and confirmations
```

---

### Task 1: Crate scaffolding, errors, table, identity

**Files:**
- Modify: `Cargo.toml` (workspace deps: `rand = "0.9"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`; member `openkdx-cli` is picked up by `crates/*`; add `openkdx-cli` is a bin so no workspace.dependencies entry)
- Modify: `crates/openkdx-client/Cargo.toml`, `src/lib.rs`
- Create: `src/error.rs`, `src/table.rs`, `src/identity.rs`
- Test: `crates/openkdx-client/tests/identity.rs`

**Interfaces:**
- Produces:
  - `pub enum ClientError { Io(std::io::Error), Codec(openkdx_codec::CodecError), Decode(DecodeError), NoTable, LoginFailed(ProofStatus), LoginTooLong, Timeout, Disconnected, Server { code: u32, name: Option<&'static str>, request: u32 }, Unexpected { expected: MessageType, got: MessageType }, Transfer(&'static str) }` with `thiserror`, `From` for the first three. `pub type Result<T> = std::result::Result<T, ClientError>`.
  - `pub fn load_kdx_table() -> Result<ChallengeTable>`: `openkdx_table::from_env(Story::Kdx)` → else `Cache::new(Cache::default_dir()).get(Story::Kdx)` → else `Err(NoTable)`. `NoTable`'s Display: `no KDX challenge table; run \`openkdx-table fetch\` or \`openkdx-table import <KDX binary>\``.
  - `pub struct Identity { pub nick: Vec<u8>, pub info: Vec<u8>, pub text_color: Color, pub background_color: Color, pub icon: Vec<u8>, pub registered: bool }`; `Default` = nick `b"openkdx"`, colours `000000` / `ffcc00` (as Client 1.600 sent in `first-connect` frame 10), empty icon, `registered: true`. `Identity::user_record(&self) -> UserRecord` with `user_id 0`, `status 0`, `client_port 10600`, `account_color 000000`, `flags 0`, everything ignored zero, `padding` empty.

- [ ] **Step 1: Write failing tests** `tests/identity.rs`: `default_identity_matches_client_1600_shape` (user_id 0, client_port 10600, background `ffcc00`, `SetIdentity { user }.to_bytes()` decodes back via `Message::decode(MessageType::SET_IDENTITY, …)`) and `no_table_error_names_the_fix` (set env `OPENKDX_KDX_TABLE` to empty and cache dir to a tempdir via a `load_kdx_table_from(cache: &Cache)` variant; assert `matches!(err, ClientError::NoTable)`). Expose `load_kdx_table_from(cache: &Cache) -> Result<ChallengeTable>` and make `load_kdx_table()` call it with the default dir.
- [ ] **Step 2:** `cargo test -p openkdx-client --test identity` → FAIL (unresolved imports).
- [ ] **Step 3:** Implement the three modules and `lib.rs` re-exports (`pub use error::{ClientError, Result}; pub use identity::Identity; pub use table::{load_kdx_table, load_kdx_table_from};`). Update `crates/openkdx-client/Cargo.toml` deps: `openkdx-proto`, `openkdx-codec`, `openkdx-table` (`default-features = false`), `tokio`, `tokio-util`, `futures-util`, `bytes`, `thiserror`, `rand`.
- [ ] **Step 4:** `cargo test -p openkdx-client --test identity` → PASS; `cargo clippy -p openkdx-client --all-targets -- -D warnings` clean; `python3 ci/check-deps.py` passes.
- [ ] **Step 5:** Commit `feat(client): crate scaffolding, errors, table loading, identity`.

---

### Task 2: Client handshake over an async stream, txids, mock server

**Files:**
- Create: `src/handshake.rs`, `src/txid.rs`, `tests/common/mock.rs`, `tests/handshake.rs`

**Interfaces:**
- Consumes: `openkdx_codec::{HandshakeCodec, HandshakeFrame}`, `openkdx_proto::handshake::{Hello, Proof, ProofStatus, SubProtocol, Role}`, `Proof::compute`, `proof_auth`, `AccountKey`, `ChallengeTable`.
- Produces:
  - `pub struct HandshakeOutcome { pub server_hello: Hello, pub server_proof: Proof, pub read_buf: bytes::BytesMut }`.
  - `pub async fn client_handshake<S: AsyncRead + AsyncWrite + Unpin>(stream: &mut S, login: &[u8], key: &AccountKey, table: &ChallengeTable) -> Result<HandshakeOutcome>`: `login.len() > 31` → `LoginTooLong`; build `Hello { sub_protocol: Kdx, version: 1, sub_version: 0x10, challenge: random in 0..=table.max_offset(), nonce: random 12 bytes, name: login, pad_len 0, skipped_len 0, ignored [0;4], padding empty }`; `Framed::new(stream, HandshakeCodec::new(random u32))`; send Hello; receive server Hello; send `Proof::compute(key, &server_hello.nonce, table, server_hello.challenge)` (None → `ClientError::Transfer("challenge past table end")` is wrong — use `Unexpected`? No: add variant `BadChallenge(u16)`); receive Proof: `status != Ok` → `LoginFailed(status)`; also verify `proof.auth == proof_auth(key, &our_nonce)` else `LoginFailed(ProofStatus::Ok)`… keep simple: mismatch → `ClientError::Transfer("server proof mismatch")` renamed to a new `ProofMismatch` variant. EOF before Proof → `LoginFailed(ProofStatus::Ok)`? No: EOF → `Disconnected`. Return the Framed's leftover read buffer so the session codec starts with it (`Framed::into_parts().read_buf`).
  - `pub struct TxidSource(AtomicU32)`: `new_random()`, `next(&self) -> u32` (wrapping increment, never 0; Client 1.600 starts at a random value and increments by 1, `[O chat-basic]`).
  - Test-only `MockServer` in `tests/common/mock.rs`: `MockServer::start(table: ChallengeTable, accounts: Vec<(Vec<u8>, AccountKey)>) -> (SocketAddr, JoinHandle)` accepting one connection: server handshake (Hello `sub_version 0x20`, Proof via the same functions; unknown login/wrong proof → `Proof::failure(nonce, ProofStatus::Ok)` with `CODE_AUTH_FAILED` semantics – send failure and close), then a scripted session: `Script = Vec<Step>` with `Step::Expect(MessageType)`, `Step::Reply(Message)` (reply flag, request's txid), `Step::Send(Message)` (own txid), `Step::DataReply { header_kind, len, bytes }`, `Step::ExpectData(len)`, `Step::Close`, `Step::Sleep(Duration)`. Uses `SessionCodec::new(&key, SERVER_RECV_CAP)`. Uses `ChallengeTable::default_table()` on both sides so no real table is needed.

- [ ] **Step 1: Write failing tests** `tests/handshake.rs`: `handshake_succeeds_against_mock` (returns server_hello with `role() == Some(Role::Server)`), `handshake_wrong_password_is_login_failed`, `handshake_rejects_long_login` (32-byte login → `LoginTooLong` before any I/O), `txid_source_increments_and_skips_zero`.
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement `handshake.rs`, `txid.rs`, `tests/common/mock.rs`.
- [ ] **Step 4:** Run → PASS. Also run `cargo test -p openkdx-codec` to be sure nothing moved.
- [ ] **Step 5:** Commit `feat(client): client handshake over an async stream; mock server for tests`.

---

### Task 3: Connection actor: requests, replies, events

**Files:**
- Create: `src/connection.rs`, `tests/connection.rs`

**Interfaces:**
- Consumes: `openkdx_codec::{SessionCodec, Event as CodecEvent, Item}`, `Header`, `Message`, `Payload`, `Status`, `error_name`, `TxidSource`.
- Produces:
  - `pub enum Event { Message(Message), Disconnected(Option<DisconnectMessage>) }` (151 is reported both as `Message` and, when the stream then closes, as `Disconnected(Some(..))`).
  - `pub struct Connection { … }` (Clone, cheap): `Connection::spawn(stream: TcpStream, read_buf: BytesMut, key: &AccountKey, max_len: u64) -> Connection` (builds `Framed::from_parts` with `SessionCodec::new(key, max_len)`; spawns the reader task; a writer `mpsc` of `Item`).
  - `pub async fn request<T: Payload>(&self, req: &T) -> Result<Message>`: txid from `TxidSource`, `Header::request(txid, T::KIND.0, len)`, waits for the frame with the reply flag and that txid, default timeout 30 s (`with_timeout(Duration)` builder); the raw `Message` (may be `Status`).
  - `pub async fn request_as<T: Payload, R: Payload>(&self, req: &T) -> Result<R>`: `Status` with `is_error()` → `Server { code, name: error_name(code), request: T::KIND.0 }`; other kind → `Unexpected`.
  - `pub async fn send<T: Payload>(&self, msg: &T) -> Result<u32>` (no reply expected; returns txid).
  - `pub fn events(&self) -> broadcast::Receiver<Event>` (capacity 256), `pub async fn close(&self)`, `pub fn is_closed(&self) -> bool`, `pub fn local_addr/peer_addr`.
  - Reader task: `CodecEvent::Message` → decode with `Message::decode`; reply-flagged and txid pending → oneshot; else broadcast `Event::Message`. `DataStart/Data/DataEnd` on a main connection → `ClientError::Transfer("data on main connection")` closes it. EOF/error → all pending oneshots get `Disconnected`, broadcast `Event::Disconnected(last_151)`.

- [ ] **Step 1: Write failing tests** `tests/connection.rs` with the mock: `request_gets_reply_by_txid` (mock: `Expect(GET_CHAT_LIST)`, `Send(some 103 UserJoined)`, `Reply(ChatList{..})` → request returns the ChatList, and `events()` yielded the 103), `status_reply_becomes_server_error` (mock replies `Status { code: 0x40070 … }` → `Server { code: 0x40070, name: Some(..) }`), `pending_requests_fail_when_peer_closes` (mock `Close` after `Expect` → `Disconnected`), `request_times_out` (mock `Sleep(2s)`, timeout 200 ms → `Timeout`).
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS, clippy clean.
- [ ] **Step 5:** Commit `feat(client): connection actor with txid-correlated requests and an event stream`.

---

### Task 4: Session login (100/118 → 101/105), users, greeting

**Files:**
- Create: `src/session.rs`, `tests/session.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct Credentials { pub login: Vec<u8>, pub password: Vec<u8> }` (`Credentials::guest()` = both empty; key = `AccountKey::from_password(&password)`).
  - `pub struct ConnectOptions { pub identity: Identity, pub timeout: Duration (30 s), pub table: Option<ChallengeTable> (None → load_kdx_table()) }` with `Default`.
  - `pub struct Session { conn: Connection, pub server: ServerInfo, pub server_addr: SocketAddr, users: Arc<Mutex<Vec<UserRecord>>>, pub greeting: Arc<Mutex<Option<Vec<u8>>>>, key: AccountKey, pub login: Vec<u8> }`.
  - `pub async fn Session::connect(addr: SocketAddr, creds: &Credentials, opts: ConnectOptions) -> Result<Session>`: TCP connect; `client_handshake`; `Connection::spawn(.., CLIENT_RECV_CAP)`; subscribe to events **before** sending 100; `send(SetIdentity { user: identity.user_record() })`; if `identity.registered` `send(LicenseReport { value: 1 })`; then wait (within `timeout`) for both `UserList` and `ServerInfo` events in any order; 107/106/213 arriving meanwhile are kept (greeting stored; 213 → remember `must_join_chat: bool`). A `Status` event with an error before 105 → `Server` error; `Disconnected` → `LoginFailed(ProofStatus::Ok)`? No — `Disconnected`.
  - Session keeps the user list current from 102/103/104 in a background task: `pub fn users(&self) -> Vec<UserRecord>`, `pub fn user_id(&self) -> u32`, `pub fn session_key(&self) -> AccountKey` (105 bytes 0..32), `pub fn transfer_addr(&self) -> SocketAddr` (105 `file_transfer_port` if non-zero, else `server_addr`), `pub fn events(&self) -> broadcast::Receiver<Event>`, `pub fn connection(&self) -> &Connection`, `pub async fn set_identity(&self, Identity)`, `pub async fn ping(&self) -> Result<()>` (108 → expect 109 reply), `pub async fn close(self)`.

- [ ] **Step 1: Write failing tests** `tests/session.rs`: `login_collects_user_list_and_server_info` (mock sends 101, 105, 107 after `Expect(100)`, `Expect(118)` → `session.user_id() == 1`, `greeting == Some(..)`, `server.name`); `guest_login_sends_no_118_when_unregistered` (identity.registered=false; mock `Expect(100)` then directly 101/105; a stray 118 would hit `Expect` mismatch and the mock replies Close); `user_list_tracks_joins_and_departs` (mock sends 103 then 104 → `users()` grows then shrinks); `must_join_chat_is_recorded` (213 before 105).
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement `session.rs`. **Step 4:** Run → PASS; `cargo test -p openkdx-client` all green; clippy clean.
- [ ] **Step 5:** Commit `feat(client): session login, user list tracking, greeting`.
- [ ] **Step 6: Review checkpoint A (requested by the user).** Dispatch a fresh-context reviewer (superpowers:requesting-code-review) on Tasks 1–4 with the spec §5.1/§9, `docs/protocol/README.md`, and `captures/first-connect.md` in the prompt. Fix findings before Task 5. Commit fixes.

---

### Task 5: `openkdx` CLI skeleton: login, users, `run`, output, exit codes

**Files:**
- Create: `crates/openkdx-cli/Cargo.toml` (`name = "openkdx-cli"`, `[[bin]] name = "openkdx" path = "src/main.rs"`; deps `openkdx-client`, `openkdx-proto`, `clap` (derive, env), `tokio`, `anyhow`, `serde`, `serde_json`, `hex`), `src/main.rs`, `src/args.rs`, `src/out.rs`, `src/cmd/mod.rs`, `src/cmd/session.rs`, `README.md`
- Test: `crates/openkdx-cli/tests/cli.rs` (uses the mock server copied as `tests/common/mock.rs` via `#[path]` include of the client crate's file: `#[path = "../../openkdx-client/tests/common/mock.rs"] mod mock;`)

**Interfaces:**
- Produces:
  - `openkdx [GLOBAL] <COMMAND>`; globals: `-s/--server HOST[:PORT]` (default port 10700; env `OPENKDX_SERVER`), `-l/--login` (default empty = guest), `-p/--password` (env `OPENKDX_PASSWORD`; empty = guest), `--nick` (default `openkdx`), `--json`, `--timeout SECS` (30), `-q/--quiet`.
  - `pub enum Command { Login { stay: Option<u64> }, Users, Run, Chat(ChatCmd), Pm(PmCmd), News(NewsCmd), Files(FilesCmd), Accounts(AccountsCmd), Classes(ClassesCmd), FolderAccess(FaCmd), Server(ServerCmd), Tracker(TrackerCmd) }` — later tasks add the variants; this task has `Login`, `Users`, `Run`.
  - `run`: reads lines from stdin; each line is parsed with `Command::try_parse_from(shlex::split(line))` (`shlex` dep) into the **same** enum and dispatched on the one open `Session`; `#` comments and blank lines skipped; `sleep SECS` and `quit` built in; stops on the first error unless `--keep-going`. Prints `> line` before each command when not `--quiet` (so scenario notes are readable).
  - `out.rs`: `pub struct Out { json: bool }` with `fn table(&self, headers: &[&str], rows: Vec<Vec<String>>)`, `fn kv(&self, pairs: &[(&str, String)])`, `fn line(&self, s: impl Display)`, `fn event(&self, Event)`; `pub fn lossy(&[u8]) -> String`; in `--json` mode each item is one JSON object per line (`serde_json::json!`).
  - Exit codes: 0 ok; 1 usage/local error; 2 login failed / server `Status` error; 3 disconnected by server; 4 timeout. `main` maps `ClientError` variants.
  - `login`: prints server name, description, version (`0x0654` → `1.620`), your user id, greeting, user count; `--stay SECS` keeps the session and prints events (103/104/151/155/301/208) until the time elapses.
  - `users`: table `id nick login? status`.

- [ ] **Step 1: Write failing test** `tests/cli.rs`: `login_prints_server_name` runs the built binary (`env!("CARGO_BIN_EXE_openkdx")`) against the mock with `--password x` and `OPENKDX_KDX_TABLE` unset but a `--table-default` hidden flag… instead: hidden global `--insecure-default-table` that uses `ChallengeTable::default_table()` (test only; documented hidden). Assert stdout contains `New Server` and exit 0; `wrong_password_exits_2`; `run_executes_two_commands` (stdin `users\nusers\n`).
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** PASS; clippy; `cargo run -p openkdx-cli -- --help` shows the tree.
- [ ] **Step 5:** Commit `feat(cli): openkdx binary with login, users, run, json output`.

---

### Task 6: Chat and private messages

**Files:**
- Create: `crates/openkdx-client/src/chat.rs`, `src/pm.rs`; `crates/openkdx-cli/src/cmd/chat.rs`, `src/cmd/pm.rs`
- Test: `crates/openkdx-client/tests/chat.rs`

**Interfaces:**
- Produces (`impl Session`): `chat_list() -> Result<Vec<ChatListEntry>>` (214→215); `join_chat(chat_id, key: u32) -> Result<ChatWindowInfo>` (202→203; key from the list entry); `leave_chat(chat_id)` (204, no reply); `say(chat_id, text: &[u8], action: bool)` (207 with `flags = 1` for action, `user_id 0`; no reply, the echo arrives as 208 event; text > `SendChatText::MAX_TEXT` → error); `set_topic(chat_id, topic)` (209, no reply); `create_chat(ChatRecord) -> Result<ChatInfo>` (217→219); `invite(chat_id, user_ids, message)` (211); `send_pm(user_id, text)` (300, no reply; empty text → error).
- CLI: `chat list`, `chat join ID` (also prints members/topic), `chat say ID TEXT...`, `chat me ID TEXT...`, `chat topic ID TEXT...`, `chat watch ID --for SECS` (join then print 208/210/205/206 events; `/me` lines printed as `* nick text`), `pm USER_ID TEXT...`, `pm wait --for SECS` (prints 301 events). Public Chat is chat 1 (`[O chat-basic 16]`): `ID` accepts a number or a name matched against `chat list`.

- [ ] **Step 1:** Failing tests: `say_sets_action_flag` (mock `Expect(207)`, capture payload: `flags == 1`, text without `/me `), `join_returns_window_info`, `send_pm_rejects_empty`. **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS. **Step 5:** Commit `feat(client,cli): chat and private messages`.

---

### Task 7: News

**Files:** `src/news.rs`, `src/cmd/news.rs`, `tests/news.rs`.

**Interfaces** (`impl Session`): `newsgroups() -> Vec<NewsgroupEntry>` (605→606); `news_messages(group_id) -> NewsMessages` (607→608); `news_message(group_id, message_id) -> NewsMessage` (609→610); `post_news(group_id, subject, body) -> Result<()>` (611; the confirmation is a 613 event with its own txid, `[O news-basic]`: wait up to `timeout` for a 613 with that `group_id`, return its `message_id`); `delete_news(group_id, message_id)` (612; 614 event); `create_newsgroup(NewsgroupInfo)` (600→603), `set_newsgroup`, `delete_newsgroup`.
- CLI: `news groups`, `news list GROUP`, `news read GROUP MSG`, `news post GROUP --subject S --body B|--body-file F`, `news delete GROUP MSG`, `news create-group NAME [--description]`.

- [ ] Steps: failing tests `post_waits_for_613` (mock `Expect(611)`, `Send(NewsMessagePosted{..})` → returns message_id), `news_list_round_trip`; FAIL; implement; PASS; commit `feat(client,cli): news`.

---

### Task 8: Files: paths, listing, info, folders

**Files:** `src/path.rs`, `src/files.rs`, `src/cmd/files.rs`, `tests/files.rs`, `crates/openkdx-cli/tests/out.rs`.

**Interfaces:**
- `pub fn parse_path(s: &str) -> FilePath` (split on `/`, drop empty segments, `PathName::new`, root for `""`/`"/"`); `pub fn path_string(&FilePath) -> String` (the proto `Display`); `pub fn join(parent: &FilePath, name: &[u8]) -> FilePath`.
- `impl Session`: `list_files(path: &FilePath) -> Vec<FileListEntry>` (400→401; empty payload = empty folder); `file_info(path) -> FileInfo` (409→410); `create_folder(parent: &FilePath, name, access: &[u8], owner: &[u8])` (402; verify in lab whether a reply or a Status comes back and record it); `move_to_trash(folder, names)` (405); `empty_trash()` (421); `search(query) -> Vec<FileSearchEntry>` (452→453); `volume_info` left for later phases.
- CLI: `files ls [PATH] [-l]` (name, size, modified, kind, `[DB]`/`[UL]` from `access_item` 0x2c/0x2b, folders marked `/`), `files info PATH`, `files mkdir PARENT NAME [--access ITEM] [--owner LOGIN]`, `files trash PATH...`, `files search TEXT`.

- [ ] Steps: failing tests `parse_path_splits_on_slash`, `format_listing_is_lossy` (row with name `b"caf\xa5"` renders), `list_root_sends_empty_path` (mock captures 400 payload == `FilePath::root().to_bytes()`); FAIL; implement; PASS; commit `feat(client,cli): file listing, info, folders`.

---

### Task 9: Transfers: download and upload with resume

**Files:** `src/transfer.rs`, `src/cmd/files.rs` (get/put), `tests/transfer.rs`.

**Interfaces:**
- `pub struct Transfer { framed: Framed<TcpStream, SessionCodec>, txids: TxidSource, user_id: u32 }`; `pub async fn Transfer::open(session: &Session, table: &ChallengeTable) -> Result<Transfer>`: connect to `session.transfer_addr()`, `client_handshake(login = [0u8] ‖ user_id.to_be_bytes(), key = session.session_key())`, `SessionCodec::new(&session_key, TRANSFER_RECV_CAP)` with the leftover read buffer. (Reference `transfers.md` "Connecting"; `[O files-basic 21, 25]`.)
- `pub struct Progress { pub done: u64, pub total: u64 }`; both methods take `on_progress: impl FnMut(Progress)`.
- `pub async fn download(&mut self, path: &FilePath, dest: &Path, resume: bool, on_progress) -> Result<DownloadInfo>`: `data_offset` = local file length if `resume` and the file exists else 0; `rsrc_offset = u64::MAX`; `options 0`; send 412; expect 413 reply (or Status → `Server`); then `DataStart(h)` with reply bit: the server rounds the offset down to 0x1000, so `start = info.data_size - h.len`; open `dest` for write, `set_len(start)`, seek to `start`, write every `Data` chunk; `DataEnd`; then the resource fork `DataStart` (reply bit clear) — drain its data (write to `<dest>.rsrc` only if `len > 0`), `DataEnd`. A `start > local len` cannot happen (server never rounds up) → `Transfer("server resumed past local data")`. Length 0 data fork → do nothing to the file (Review Focus 3).
- `pub async fn upload(&mut self, local: &Path, folder: &FilePath, name: &[u8], resume: bool, on_progress, stop_after: Option<u64>) -> Result<()>`: `UploadFile { data_size = len, rsrc_size 0, options 0, strings: FileStrings { name, second: kind_for(name), third/fourth empty }, folder, rest empty }` where `kind_for` = `exttype/<lowercase ext>` or empty (`[O files-basic 16]` shows the server itself writes `exttype/bin`; confirm in the lab and adjust); send 415, expect 416 reply (`data_offset` already rounded by the server; Status 0x30073 = name exists → `Server`); send `Item::DataHeader(Header::reply? no: Header::request(txid, 417, len = data_size - data_offset))`, stream the file from `data_offset` in 32 KiB `Item::Data` chunks; if `stop_after` bytes have been sent, return `Err(Transfer("stopped"))` and drop the connection (this is how the scenario produces a partial upload); then `Item::DataHeader(Header::request(new txid, 417, 0))` for the resource fork; wait for the 418 event (own txid, no reply flag) → `Ok`.
  - Open question to settle on the lab (record in `docs/protocol/README.md`): whether the first 417 header carries the 415 txid with the reply bit (mirroring the server's 414) or a fresh txid. Implement as **415's txid, reply bit set** (symmetry with `[O files-basic 29]`), and switch if Server 1.620 answers 0x40074.
- CLI: `files get PATH [--to DIR|FILE] [--resume]` (prints a one-line progress at the end, or `--progress` for a bar); `files put LOCAL [FOLDER] [--name N] [--resume] [--stop-after BYTES]` (hidden `--stop-after`, used by `lab/scenarios/upload-resume.sh`). Both open their own `Transfer` from the command's `Session`.

- [ ] **Step 1: Failing tests** with mock `DataReply`/`ExpectData`: `download_writes_fork_bytes`, `download_resume_from_rounded_offset` (local 5000 bytes, server replies 414 of `size - 4096` → file correct), `download_resume_at_end_leaves_file_intact` (414 len 0; mtime/contents unchanged), `upload_sends_417_for_each_fork_and_waits_418`, `upload_resumes_from_416_offset`, `upload_stop_after_closes_early`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS; clippy. **Step 5:** Commit `feat(client,cli): file download and upload with resume on a transfer connection`.

---

### Task 10: Accounts, classes, folder access

**Files:** `src/accounts.rs`, `src/cmd/accounts.rs`, `tests/accounts.rs`.

**Interfaces** (`impl Session`):
- `accounts() -> Vec<AccountListEntry>` (512→513); `account(login) -> Account` (509→510); `create_account(Account)` (507; reply: expect either an empty reply or Status — lab decides; implement `request` and accept `Status` with `code < 0x10000` as success); `set_account(Account)` (511); `delete_account(login)` (508; 515 event); `change_own_password(new: &[u8])` (550 with `AccountKey::from_password(new)` → 551).
- Helper `pub fn new_account(login, name, password: Option<&[u8]>, class_index: u8, privileges: Privileges, mask: Privileges) -> Account` filling times with now (`Time1900::from_unix_ms`), `key` = account key or zeros to keep.
- `classes() -> Vec<AccountClassListEntry>` (505→506); `class(index) -> AccountClassInfo` (502→503); `set_class(AccountClassInfo)` (504).
- `folder_access_list() -> Vec<FolderAccessItem>` (1106 with `rest` = `flags 0` → 1107); `folder_access(name) -> FolderAccess` (1103→1104); `create_folder_access(FolderAccess)` (1100); `set_folder_access(FolderAccess)` (1105); `delete_folder_access(name)` (1101; 1102 event).
- Privileges parsing for the CLI: `pub fn parse_privileges(spec: &str) -> Result<Privileges>` accepting comma-separated `Privilege` names in kebab-case (`can-login,download,upload`) or `all`/`none`; `format_privileges(&Privileges) -> String`.
- CLI: `accounts list|show LOGIN|create LOGIN --password P --name N --class I [--privileges SPEC]|set LOGIN [--name] [--password] [--class] [--privileges]|delete LOGIN|passwd NEW`; `classes list|show I|set I [--name] [--privileges] [--speed-limit] [--flood relaxed|average|strict]`; `folder-access list|show NAME|create NAME --icon normal|uploads|dropbox [--owner] [--description]|delete NAME`.

- [ ] Steps: failing tests `parse_privileges_round_trips`, `new_account_sets_key_from_password`, `account_list_round_trip`; FAIL; implement; PASS; commit `feat(client,cli): accounts, classes, folder access`.

---

### Task 11: Administration: server settings, broadcast, disconnect/ban, history, connection monitor, shutdown

**Files:** `src/admin.rs`, `src/cmd/admin.rs`, `tests/admin.rs`.

**Interfaces** (`impl Session`):
- `server_settings() -> ServerSettings` (170→171); `set_server_settings(ServerSettings)` (172→173 reply or Status); helper `set_server_name(&mut ServerSettings, name)` via `ServerSettingsMain::decode/to_bytes` on section 0 (sections 1–10 pass through untouched).
- `broadcast(text)` (154; no reply; 155 arrives as event to everyone incl. sender — verify).
- `disconnect_user(user_id, ban_seconds: u32, message: &[u8], quiet: bool)` (150; `options` bit0 = quiet).
- `history_join() -> HistoryList` (180→181 reply), `history_leave()` (182); 183 events while joined.
- `monitor_open() -> ()` (140), `monitor_info() -> ConnectionMonitorInfo` (142→143), `monitor_close()` (141).
- `shutdown(mode: u8, message)` (174), `extra_server_info() -> ExtraServerInfo` (113→114), `my_address() -> YourAddress` (115→116).
- CLI: `server settings` (prints name, greeting flag, speed limit, and the raw section lengths), `server set-name NAME`, `server broadcast TEXT...`, `server disconnect USER_ID [--ban SECS] [--message TEXT] [--quiet]`, `server history [--follow SECS]` (table: time kind user nick login address detail path), `server monitor [--for SECS] [--every SECS]`, `server shutdown [--message] [--mode N]`, `server info` (113/115).

- [ ] Steps: failing tests `set_server_name_rewrites_only_section_0`, `history_join_returns_list_then_events`, `monitor_info_round_trip`; FAIL; implement; PASS; commit `feat(client,cli): server administration`.

---

### Task 12: Tracker browse

**Files:** `src/tracker.rs`, `src/cmd/tracker.rs`, `tests/tracker.rs`.

**Interfaces:**
- `pub struct Tracker { conn: Connection }`; `Tracker::connect(addr: SocketAddr, table: &ChallengeTable, timeout) -> Result<Tracker>`: handshake with empty login and `AccountKey::GUEST` (`[O tracker-basic]`: client hello sub_version 0x10, empty name), check `server_hello.role() == Some(Role::Tracker)` else `Unexpected`; `Connection::spawn(.., TRACKER_RECV_CAP)`; `groups() -> Vec<GroupListEntry>` (700→701); `servers(group: &[u8]) -> Vec<ServerListEntry>` (702→703).
- CLI: `tracker groups`, `tracker servers [GROUP]` (address via `socket_addr()`, users, name, description), with `-t/--tracker HOST[:10800]` on the subcommand (env `OPENKDX_TRACKER`).

- [ ] Steps: failing test `tracker_rejects_server_role` (mock with `sub_version 0x20` → `Unexpected`), `groups_and_servers_round_trip` (mock with `0x30`); FAIL; implement; PASS; commit `feat(client,cli): tracker browse`.

---

### Task 13: Differential tests in the lab

**Files:**
- Create: `crates/openkdx-client/tests/lab.rs`, `lab/differential.sh`
- Modify: `.github/workflows/ci.yml` lab job (after line 83 add `lab/differential.sh`), `lab/README.md`

**Interfaces:**
- `tests/lab.rs`: every test starts with `let Some(server) = lab_server() else { eprintln!("skipped: OPENKDX_LAB_SERVER not set"); return; };` where `lab_server()` reads `OPENKDX_LAB_SERVER` (`127.0.0.1:10700`), `OPENKDX_LAB_TRACKER` (optional), `OPENKDX_LAB_PASSWORD` (default `openkdx-lab`), table via `load_kdx_table()`. Tests run **serially** (`--test-threads=1`) and each opens at most 2 connections; keep the total under 25 (Global Constraints). Tests: `lab_login_admin` (105 version 0x0654, user id ≥ 1, no licence reminder: the server log is checked by the script instead), `lab_login_guest`, `lab_login_wrong_password` (`LoginFailed`), `lab_chat_say_and_echo` (join 1, say, 208 with our text and `/me` flag), `lab_pm_between_two_sessions`, `lab_news_post_and_read`, `lab_files_list_get_put_resume` (seed file by uploading 64 KiB with `stop_after 20000`, re-open, upload with resume → 416 `data_offset == 0x4000`; download; byte-compare; resume download of a truncated local copy), `lab_accounts_create_show_delete` (create `tester` class 1, show has_password, delete → 515 event), `lab_classes_list`, `lab_folder_access_list_create_delete`, `lab_server_settings_set_name_round_trip` (set then restore), `lab_broadcast_received_by_other_session`, `lab_disconnect_from_victim_side` (guest session disconnected by admin with ban 5 s: victim gets `Event::Disconnected(Some(151))` with `FLAG_BANNED`; reconnect within the ban fails or closes; after 6 s succeeds), `lab_history_join_sees_login_record`, `lab_connection_monitor_lists_sessions`, `lab_tracker_groups_and_servers` (needs the tracker + the server registered: `OPENKDX_LAB_TRACKER` set).
- `lab/differential.sh`: same lock/guard preamble as `lab/ci-smoke.sh`; `reset all`; `ns start`; `server start` (wait for "KDX Server started"); `tracker start`; `sleep 3` (UDP registration); export `OPENKDX_KDX_TABLE` as the CI job does; `cargo build -q -p openkdx-client --tests`; run `"$LAB" exec env OPENKDX_LAB_SERVER=127.0.0.1:10700 OPENKDX_LAB_TRACKER=127.0.0.1:10800 OPENKDX_KDX_TABLE=… cargo test -q -p openkdx-client --test lab -- --test-threads=1`; grep the server log for `licen` (case-insensitive) and fail if found (spec §9); print the log tail; `ns stop`.

- [ ] **Step 1:** Write `tests/lab.rs` (they skip locally: `cargo test -p openkdx-client --test lab` prints `skipped`). **Step 2:** Write `lab/differential.sh`; `shellcheck lab/differential.sh`. **Step 3:** Push the branch; on the lab: `ssh openkdx-lab 'cd ~/openkdx-src && git fetch && git checkout phase2/client-cli && git pull && PATH=$HOME/.cargo/bin:$PATH lab/differential.sh'` (wait for the lab CI job to release the lock if it reports busy). Expected last line `differential: ok` and 16 tests passed. **Step 4:** Fix whatever the real server disagrees with (417 txid rule, 402/507 replies, upload `kind` string); record each finding in `docs/protocol/README.md` with `[O <scenario> frame]` once Task 14 has the capture, or `[S lab]` meanwhile. **Step 5:** Add the CI step; commit `test(client): differential tests against KDX Server 1.620 in the lab`.

---

### Task 14: New captures: eight scenarios driven by the CLI

**Files:**
- Modify: `lab/lib/scenario.sh` — add `build_cli()` (`cargo build -q -p openkdx-cli`, once), `cli()` = `"$LAB" exec env OPENKDX_KDX_TABLE=… "$REPO/target/debug/openkdx" -s 127.0.0.1 -l "$KDX_ADMIN_LOGIN" -p "$KDX_ADMIN_PASSWORD" "$@"`, `cli_guest()`, `cli_run()` (feeds a heredoc to `openkdx run`), `cli_bg()` (background + pid), and `PROGRAMS="KDX Server 1.620 (Linux), OpenKDX client (openkdx CLI, this commit)"` when a scenario calls `use_cli`.
- Create: `lab/scenarios/upload-resume.sh`, `accounts-classes.sh`, `folder-access.sh`, `server-settings.sh`, `broadcast.sh`, `disconnect-ban.sh`, `server-history.sh`, `connection-monitor.sh`
- Create (recorded on the lab, copied back): `captures/<name>.pcapng` + `.md` ×8
- Modify: `captures/README.md` (table rows, "Not yet captured" paragraph), `crates/openkdx-dissect/tests/captures.rs` (eight names in the list; expected kind sequences for `upload-resume` main `[100,118,101,105,400,401]`-style and transfer `[415,416,417,417,418]` twice with data headers `[(20000,…),…]`, and the inventory check), `docs/protocol/README.md`.

Scenario contents (each: `scenario_begin`, `start_server`, seed, `capture_begin`, `cli_run` with `step` lines, `capture_end`, `scenario_end`):
1. **upload-resume**: 64 KiB random file; `files put big.bin Uploads --stop-after 20000` (fails by design); `files put big.bin Uploads --resume` (416 offset 0x4000); `files ls Uploads`; `files get Uploads/big.bin`; verify equal.
2. **accounts-classes**: `classes list`, `classes show 1`, `accounts list`, `accounts create tester --password t --name Tester --class 1`, `accounts show tester`, `accounts set tester --name "Tester 2"`, `accounts delete tester`, `accounts list`.
3. **folder-access**: `folder-access list`, `show "Uploads"`, `create Pictures --icon normal --description pics`, `files mkdir / Pictures --access Pictures`, `files ls /`, `delete Pictures`.
4. **server-settings**: `server settings`, `server set-name "OpenKDX Lab"`, `login` again shows the new name in 105, `server settings`, restore.
5. **broadcast**: guest `login --stay 20` in background; admin `server broadcast "hello all"`; both logs show 155.
6. **disconnect-ban**: guest `login --stay 30` in background; admin `users`, `server disconnect <guest id> --ban 10 --message bye`; guest process exits 3 and prints the 151; a guest `login` within 10 s fails; one after succeeds.
7. **server-history**: admin `server history --follow 10` in background; guest logs in and out; the foreground shows 181 then 183 records (184 Login, 185 Disconnected).
8. **connection-monitor**: a second admin session runs `files get` of a 4 MiB seeded file; main session `server monitor --for 8 --every 2` shows the transfer row (`transfer_id != 0`, bytes advancing); `monitor close`.

- [ ] **Step 1:** Extend `scenario.sh`; write the eight scripts; `shellcheck` them. **Step 2:** On the lab, run each (`lab/scenarios/<name>.sh`), then `cargo run -q -p openkdx-dissect -- -p openkdx-lab --audit ~/captures/<name>.pcapng` → every line `audit: exact`, exit 0. For a MISMATCH or `undecoded`, fix the proto struct (these are the first real bytes for 415–418, 5xx, 140–191, 1100–1107) and re-audit. **Step 3:** `scp` the `.pcapng`/`.md` pairs into `captures/`; `cargo test -p openkdx-dissect` with the table → PASS including `inventory_matches_capture_notes` for all fifteen. **Step 4:** Update `captures/README.md`, `docs/protocol/README.md` (new deltas/confirmations with `[O name frame]` tags; close the 417 txid question; note what the server put in 401 `kind` for the upload). **Step 5:** Commit `captures: upload-resume, accounts-classes, folder-access, server-settings, broadcast, disconnect-ban, server-history, connection-monitor (openkdx CLI vs Server 1.620)`.

---

### Task 15: Hello byte-exactness (Phase 1 carry-over), docs, PR

**Files:**
- Modify: `crates/openkdx-proto/src/handshake.rs`, `crates/openkdx-proto/tests/handshake.rs`
- Modify: `README.md`, `crates/openkdx-client/README.md`, `crates/openkdx-cli/README.md`, `docs/superpowers/specs/2026-11-08-openkdx-design.md` (§8 "Phase 2 complete" line), `docs/protocol/README.md` (known-inexact note: `Draw::encode` clamp remains)
- Copy this plan to `docs/superpowers/plans/2026-11-10-phase2-client-cli.md`.

**Interfaces:** `Hello` gains `pub pad: Vec<u8>` and `pub skipped: Vec<u8>` (the bytes previously zeroed), `name` keeps all `name_len` bytes with `pub fn login(&self) -> &[u8]` returning the first 31 (what the server uses); `encode` writes them back; `wire_len()` unchanged. Replace test `hello_skips_pad_and_truncates_name_to_31` with `hello_with_pad_and_long_name_round_trips_byte_exact`.

- [ ] **Step 1:** Write the new proto test; FAIL. **Step 2:** Implement; `cargo test -p openkdx-proto -p openkdx-codec -p openkdx-dissect -p openkdx-client` PASS. **Step 3:** Docs. **Step 4:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `python3 ci/check-deps.py`, `python3 ci/check-blocklist.py`; MSRV `cargo +1.88 check --workspace` if installed. **Step 5:** Commit `docs: Phase 2 complete; proto: Hello re-encodes pad, skipped bytes and long names byte-exact`; push; open PR #3 (`POST /repos/spitfire915/OpenKDX/pulls`, base `main`, head `phase2/client-cli`, title `Phase 2: client library + CLI`) and link it with `session.link`.
- [ ] **Step 6: Review checkpoint B (requested).** Fresh-context whole-branch review (superpowers:requesting-code-review) with the spec, this plan, and `docs/protocol/README.md`; fix findings; wait for the lab CI job (`lab/ci-smoke.sh` + `lab/differential.sh`) to go green before reporting done.

---

## Self-review notes

- Spec coverage: §8 item 2 (login, chat, PM, news, files incl. put/resume, accounts, tracker) → Tasks 4, 6, 7, 8–9, 10, 12; "passes differential tests against the original server" → Task 13; §7.5 captures with notes and inventory → Task 14; §9 client identity → Task 4 + `lab/differential.sh` licence-log check; user's capture list (upload with resume, accounts and classes, folder access, server settings, broadcast, disconnect/ban, server history, connection monitor) → Task 14 items 1–8; Phase 1 deferred minors → Task 15 and Global Constraints.
- Decisions the lab must confirm (each has a fallback written into its task): the 417 txid/reply rule (Task 9), the upload `kind` string (Task 9), whether 402/507/172 answer with a reply or a Status (Tasks 8, 10, 11), whether 155 echoes to the sender (Task 11), and whether a registered client's `118 { value: 1 }` changes server behaviour (Task 13 licence grep; the captured Client 1.600 sent none, `[O first-connect]`).
- Type consistency: `Session::connect(addr, &Credentials, ConnectOptions)`, `Connection::request/request_as/send/events`, `Transfer::open(&Session, &ChallengeTable)`, `Tracker::connect(addr, &ChallengeTable, timeout)`, `parse_path(&str) -> FilePath` are the names every later task uses.
- Not in this phase: DCC, voice, view display, GConsole, IRC, volumes, catalog generation, process monitor (CLI stays "thin"; the structs exist in proto for Phase 9/10).
