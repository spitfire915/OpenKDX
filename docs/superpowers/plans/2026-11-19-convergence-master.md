# Phases 3B–3F: Convergence — master plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. The convergence is five phases, **3B–3F**, each one fresh-context session of 1–4 tasks with its own short plan file (`2026-11-19-phase3c-…` etc.) that points back here for the Global Constraints, the type mapping and the ledger. Every session starts by reading this file, its phase file and `## Ledger`, and ends by appending a ledger entry (see "How a session runs").

**Goal:** OpenKDX stops carrying its own copies of the protocol, codec, client, dissector and tracker: the `kdx-*` crates of a fork `spitfire915/kdx` (integration branch, upstreamed by PR to `scottanderson/kdx`) hold one implementation, with our 13 protocol deltas, 15 captures, the client features of `openkdx-client`, and `openkdx-legacy` as a new `kdx-legacy`; OpenKDX depends on them by pinned git revision, keeps `openkdx-table`, `openkdx-cli`, `openkdx-ui`, `openkdx-app`, `openkdxd`, `lab/`, `re/`, `ci/`, and the lab's differential suite gates both repositories.

**Architecture:** Two repositories, one lab. The fork's `main` is an *integration branch*: upstream `main` merged in (merge commits, never rebased, so pinned SHAs stay reachable) plus our merged `feat/*` PRs; each merged change worth sharing is cherry-picked onto `upstream/main` as `up/<topic>` and opened as a PR against `scottanderson/kdx`. OpenKDX pins `rev = <fork main SHA>` in `Cargo.toml`; local work uses an untracked `.cargo/config.toml` `[patch]` to the sibling checkout `../kdx`. The lab machine runs two GitHub Actions runners (one per repository, both labelled `lab`, both serialised by `~/lab/run/lab.lock`); `lab/differential.sh` and `lab/legacy.sh` live in OpenKDX and take the kdx checkout to test as an argument, so the fork's `lab.yml` checks out OpenKDX and calls them.

**Tech Stack:** Rust; fork crates on upstream's terms (edition 2021, `rust-version = "1.96"`, no `Payload` trait, inherent `decode`/`encode`, `MessageType(pub u32)` consts, `kdx-test-tables` for tests, `ci/check-docs.py`, codespell); OpenKDX crates edition 2024, MSRV raised to 1.96. GitHub REST API with `git credential fill` (no `gh`). Lab: `ssh openkdx-lab`, `~/openkdx-src` (`git pull` first), `~/.cargo/bin`, admin `admin` / `openkdx-lab`.

**Spec:** `docs/superpowers/specs/2026-11-08-openkdx-design.md` — §6.1 amendment of 2026-11-19, §8 item 3b, §9 (amended first bullet), §5.3 (applies to both repositories), §7.5. Upstream rules: `AGENTS.md` and `CONTRIBUTING.md` of scottanderson/kdx (reference clone: `/private/var/folders/wk/hs0yq57n5q9g5s8kb5ybhh400000gn/T/opencode/kdx-ref`, HEAD `7f82081`; `git clone https://github.com/scottanderson/kdx` there if gone).

## Global Constraints

- **Two conventions, by repository.** In OpenKDX: `type(scope): summary`, trailer `Co-Authored-By: Claude <noreply@anthropic.com>` on every commit (CONTRIBUTING). In the fork: upstream `AGENTS.md` — subject an imperative sentence with no area prefix, reasons in the body, author/committer `spitfire915`, the doc update in the same commit as the code, prose wrapped at 80 columns, evidence as `<!-- [O file frame] -->` / `<!-- [S FUN_…] -->` comments, `python3 -I ci/check-docs.py` and `codespell` clean, `cargo clippy --workspace --all-targets` with no warnings, `cargo fmt`. The trailer is the user's mandated `Co-Authored-By: Claude <noreply@anthropic.com>` (upstream's example names the model; see Open points).
- **Fork branches.** `main` = integration (merge `upstream/main` in with `git merge --no-ff upstream/main`; never rebase or force-push). `feat/<topic>` → PR into fork `main` (CI: hosted `check`, `docs`, `lab`). After merge: `git checkout -b up/<topic> upstream/main && git cherry-pick <commits>` → PR into `scottanderson/kdx` with a prose body and no `Co-Authored-By` line in the description. Record every PR number in the ledger.
- **Nothing of Haxial's in either repository** (§5.3): no tables, no `.hap`, no artwork; the OpenKDX blocklist rule applies to what we push to the fork too (`python3 ci/check-blocklist.py` over the fork's tree before each fork push: `python3 ci/check-blocklist.py --root ../kdx`, added in Task 1). Captures hold only lab traffic; the published lab password is `openkdx-lab`.
- **Byte-exactness is the acceptance test everywhere.** Every one of our 15 captures dissects under `kdx-dissect --audit` with no `mismatch`; every legacy fixture satisfies `encode(decode(f)) == f` under `kdx-legacy`; every differential test of `crates/openkdx-client/tests/lab.rs` has a passing counterpart in `crates/kdx-client/tests/lab.rs`.
- **Pin discipline.** OpenKDX's `Cargo.lock` names the fork's `main` SHA for `kdx-*`; the only commits that change those entries are `build: pin kdx-* at <sha>` commits. CI runs `cargo metadata --locked` first; a lockfile written under the local `[patch]` fails it.
- **MSRV** of OpenKDX becomes 1.96 (`rust-version`, the `msrv` CI job, CONTRIBUTING, README) in Task 2, because the fork's crates require it.
- **Lab budget.** Each differential run opens at most 25 connections per original-server start (ban after 30 in 300 s). The lock: `exec 9>"$RUN/lab.lock"; flock -n 9` as `lab/differential.sh` does; a CI run holds it up to ~25 min after each push to either repository.
- **Out of scope:** `kdx-server`, `openkdxd` and the retargeting of the Phase 4 plan (Phase 4, its first session); the tracker program (Phase 5, on `kdx-tracker`); GUI; voice/DCC/view display/GConsole beyond what the captures already pin; crates.io publication (upstream publishes before our release, §6.1).
- **Deletions happen last** (Task 14) and only after Tasks 4–13 are green on both CIs; until then OpenKDX builds both `openkdx-*` and `kdx-*`.

## Review Focus

1. **A `Cargo.lock` committed with the `[patch]` active** would make every hosted job build from `../kdx` paths that do not exist there. Pinned by Task 2's `cargo metadata --locked` step in `ci.yml` and the test `check_deps_rejects_patched_lock` in `ci/check-deps_test.py`.
2. **Our captures hitting upstream decoders that are shaped differently** (143 with `transfer_id != 0` and a path; 183 added records; 171 with eleven sections and a 172 that sends `(0, 0)` for section 7; 151 with `flags = 2`): `--audit` must show `Same`, not `Mismatch`, and not a silent `rest` swallowing. Pinned by Task 4's `EXPECTED`/`NO_CODEC` assertions being unchanged except for the documented additions, and by the per-capture fixtures `openkdx-connection-monitor-<port>.s2c.bin` etc. matching.
3. **`send_checked`'s barrier (115 Get My Address)** must not mistake a *success* Status for 402/507/511/1100/150/172 (our lab shows the server answers those with `Status::Success { value: request type }`) for an error, nor swallow a Status aimed at a different txid. Pinned by Task 9's mock test `success_status_reply_is_not_an_error_and_is_correlated_by_txid`.
4. **Resume arithmetic** on upload: 416 offers an offset that is a multiple of 0x1000 (0x20000 after a 150000-byte cut) and the `.prt` partial is listed by 401. Pinned by `lab_files_list_get_put_resume` in Task 11 against Server 1.620, and the mock test `upload_resumes_from_the_offered_offset` in Task 10.
5. **Two runners, one lock.** The fork's lab job and OpenKDX's lab job may start within seconds of each other; the loser must *wait*, not fail. Pinned by Task 11: `lab/differential.sh` and `lab/legacy.sh` use `flock -w 2400 9` (wait up to 40 min) when `LAB_WAIT=1`, which both workflows set; the test is a dry run of both scripts' lock acquisition on the lab machine (`lab/lock-test.sh`).
6. **Legacy byte-exactness after the type swap**: `Account.privilege_mask`, `AccountClass` info-vs-list layouts, `FolderAccess.class_rules: [u32; 32]` vs our `[FolderAccessRule; 32]`, and the typed `ServerSettings` must still write the 50 fixtures byte for byte. Pinned by Task 12's `fixtures_round_trip` over every file under `crates/kdx-legacy/tests/fixtures/`.

---

## Type mapping: `openkdx-*` → `kdx-*`

The executor of every porting task uses this table; where the right column says **add**, the task that adds it is named. Upstream has no `Payload` trait and no `Message` enum: a request is `client.request(MessageType::X, &payload.encode()?)` and a reply is `Reply::decode(&bytes)?`.

| Ours (`openkdx_proto`) | Upstream (`kdx_proto`) | Note |
|---|---|---|
| `wire::{Reader, Writer, pad4_len}`, `DecodeError::{Truncated, Invalid, Trailing}` | — (crate-private helpers `need`, `u32_at`, `strings`, `pad4`, `padding`, `decode_counted`); `msg::{DecodeError::{TooShort, Invalid}, EncodeError::TooLong}` | `kdx-legacy` gets its own `wire.rs` (Task 12). Trailing bytes are not an error upstream: `Decoded<T>` keeps them as `rest`. |
| `Payload::{KIND, decode, encode(&mut Writer), to_bytes}` | inherent `decode(&[u8]) -> Result<Self, DecodeError>` / `encode(&self) -> Vec<u8> \| Result<Vec<u8>, EncodeError> \| [u8; LEN]`; lists `decode_list`/`encode_list` | |
| `MessageType::<NAME>`, `.name()`, `ALL`, `is_known()` | `MessageType::<NAME>` (same names; ours `LICENSE_REPORT` = his `UNNAMED_118`), `.title()` | |
| `AccountKey::GUEST`, `.as_bytes()` | `AccountKey::EMPTY_PASSWORD`, `.0` | |
| `ChallengeTable::{new, default_table, len, max_offset, verify}` | `ChallengeTable::{from_bytes(Story, &[u8]), extract, default(), offsets(), response()}`; `Story::{Kdx, Hgc}` | `from_bytes` checks SHA-256; `ChallengeTable::default()` is the 31-byte `DEFAULT` line — the mock uses it (Task 8). |
| `openkdx_table::Story::{Kdx, Gconsole}` | `kdx_proto::Story::{Kdx, Hgc}` | `openkdx-table` returns `kdx_proto::ChallengeTable` from Task 2. |
| `handshake::{Hello{sub_protocol: SubProtocol, …, pad, skipped, padding}, Role, SubProtocol, ProofStatus::{Ok, Failed, …}, Proof::{compute, failure}, proof_auth, table_for_peer}` | `handshake::{Hello{sub_protocol: [u8;4], …, name}, Status::{Ok, UnknownProtocol, TooOld, TooNew, Other(u8)}, Proof::{new_ok, new_error, verify}, compute_auth}`; consts `CLIENT_SUB_VERSION 0x10`, `SERVER_SUB_VERSION 0x20`, `TRACKER_SUB_VERSION 0x30` | **add** `Status::Failed` (= 1) in Task 5 (delta "Proof status 1"). |
| `session::{Header{kind: u32, reply: bool}, padding_bytes, *_RECV_CAP}` | `session::{Header{kind: MessageType, flags: u8, FLAG_REPLY}, padding_fill, SERVER_MAX_PAYLOAD_LEN, CLIENT_MAX_PAYLOAD_LEN}` | Phase alignment of the fill is verified by Task 4 (delta "payload padding"). |
| `registration::{Registration{…, string11, string12, ignored_1c, ignored_26}, udp_xor, udp_crc, RegistrationError}` | `registration::{Registration{…, groups: [Vec<u8>;6], padding}, crypt, checksum}`, errors in `DecodeError` | Task 4 checks `tracker-basic` frame 1 re-encodes; if not, the missing fields go upstream. |
| `msg::Status{code, request_type_or_value, detail}`, `error_name` | `msg::Status::{Success{value}, Error{code, request, module, line, extra, value_14}, TrackerError{code, request}}`, `.code()` | **add** `pub fn error_name(code: u32) -> Option<&'static str>` to `kdx_proto::msg::status` (Task 9; the CLI prints it). |
| `common::Color` | `msg::Color` | |
| `common::Time1900` | — | moves to `kdx_legacy::time::Time1900` (Task 12); wire fields stay `u64` ms upstream. |
| `common::{FilePath{names: Vec<PathName>}, PathName}` | `msg::FilePath(pub Vec<Vec<u8>>)`, `decode_delimited`, `encode_delimited`, `new` | `PathName.ignored` is dropped upstream (always 0 in every capture; Task 4 confirms via audit). |
| `common::UserRecord`, `ChatRecord` | `msg::UserRecord` (identical fields), `msg::chat::ChatRecord` (identical) | |
| `session::{SetIdentity, UserList, UserChanged, UserJoined}` | `UserRecord::encode` (100/102/103), `UserRecord::decode_list` (101) | |
| `session::{UserDeparted, ServerInfo, ServerIcon{icon}, ServerGreeting, SetMiniUserIcon, MiniUserIconChanged, ExtraServerInfo, YourAddress{address}, Skpk{key:[u8;32]}, LicenseReport}` | `user::{UserDeparted, MiniUserIcon, MiniUserIconChanged, RegistrationNotice}`, `server_info::{ServerInfo, ServerIcon(Vec<u8>), ServerGreeting}`, `admin::{ExtraServerInfo, YourAddress(Vec<u8>)}`, `dcc::Skpk{key: AccountKey}` | |
| `chat::{SendChatText, ReceiveChatText}{flags: u16, ignored_0a, FLAG_ACTION}` | `chat::ChatText{ignored: [u8;4]}` | **add** `flags: u16, ignored_0a: [u8;2]`, `const FLAG_ACTION: u16 = 1`, `is_action()` (Task 5; delta "/me"). |
| `chat::{JoinChat, ChatWindowInfo, CreatePrivateChatReply{info}, ChatUserJoined/Departed, SetChatTopic, ChatTopicChanged, ChatInvite, ChatInvitation, ChatList{chats}, ChatDeleted, ChatDccAddresses{addresses}, CreateChat/ChatInfo/SetChat{chat}}` | `chat::{JoinChat, ChatWindowInfo, ChatUserChange, SetChatTopic, ChatTopicChanged, ChatInvite, ChatInvitation, ChatListEntry::decode_list, ChatDeleted, DccAddress::decode_list, ChatRecord, ChatId}` | |
| `chat::{SendPrivateMessage, IncomingPrivateMessage}` | `msg::PrivateMessage` | |
| `news::*` | `news::{NewsgroupEntry, NewsgroupInfo, NewsMessage, NewsMessages, PostNewsMessage, NewsMessageChange, NewsgroupId, NewsMessageId}`, consts `NEWS_POST/DELETE/READ` | same fields. |
| `files::*` | `file::{FileListEntry, FileInfo, CreateFolder, MoveItems, MoveToTrash, SetFileInfo, VolumeInfo, SetVolumeInfo, LaunchFile, GenerateCatalog{flags: u32}, CatalogStatus, FileSearch, FileSearchEntry, FileData}`; `transfer::{DownloadFile, DownloadInfo, FileStrings, UploadFile, UploadReply, NotifySpeedLimit, OPTION_PLAIN}` | ours `GenerateCatalog.flags: Option<u32>` → his `u32` with empty payload ⇔ 0. |
| `accounts::{Privilege, Privileges([u8;16])}` | `privileges::{Privilege, Privileges(u128)}` with `of`, `contains`, `insert`, `iter`, `decode([u8;16])`, `encode()` | ours has `Unnamed85`-style bits; his `unnamed() -> u128`. |
| `accounts::{Account, AccountClassInfo, AccountClassListEntry, AccountListEntry, AccountList, AccountClassList, GetAccountClass, SetAccountClass, CreateAccount/AccountInfo/SetAccount{account}, DeleteAccount/GetAccount/AccountDeleted{login}, ChangeOwnPassword{account_key}}` | `admin::{Account, AccountClass (decode_info/decode_set/decode_list), AccountListEntry::decode_list, AccountClassIndex(u8), AccountLogin(Vec<u8>), ChangeOwnPassword(AccountKey)}` | `AccountClassInfo{second_string, editor_login}` → `AccountClass{second, editor}`. |
| `admin::{ConnectionMonitorInfo{records}, DisconnectUser, DisconnectMessage, GetUserInfo, UserInfo, SendBroadcast, RcvBroadcast, ShutdownServer, HistoryList, HistoryAdded{record}, HistoryRecord{kind: HistoryKind}}` | `admin::{MonitoredConnection::decode_list, DisconnectUser, DisconnectMessage, HistoryRecord::{decode_added}, HistoryList, SendBroadcast, RcvBroadcast, ShutdownServer}`, `user_info::{GetUserInfo, UserInfo}`; `HistoryRecord.kind: MessageType` | |
| `admin::ServerSettings{main: ServerSettingsMain, trackers, allow_deny, greeting, icon, chats, other, trailing, placeholders}`, `Greeting`, `TrackerEntry`, `AddressRule` | `server_info::ServerSettings{sections, value_64: [u8;12], body: Vec<u8>}` + accessors | **replace** upstream's `body` with our typed sections, keeping his accessors (`section(i)`, `name()`, `speed_limit()`, `greeting()`, `icon()`, `flags()`) over the typed fields (Task 6). |
| `misc::{ProcessList, ExitTerminateProcess, ViewDisplay*, StartVoiceReply, VoiceFormat, VoiceData}` | `admin::{ProcessList, Process, ExitTerminateProcess}`; 902–905, 1001, 1002 have **no codec** upstream | ours stay out: Phase 10 brings them with evidence; `NO_CODEC` keeps listing them. |
| `misc::{FolderAccess{class_rules: [FolderAccessRule;32], rule}, FolderAccessItem, FolderAccessList, GetFolderAccessList, CreateFolderAccess/FolderAccessInfo/SetFolderAccess{item}, Delete/Get/FolderAccessDeleted{name}}` | `admin::{FolderAccess{class_rules: [u32;32], rule: u32}, FolderAccessItem::decode_list, GetFolderAccessList{flags: Option<u32>, rest}, FolderAccessName(Vec<u8>)}`; `FolderAccess::{BUILT_IN, OWNER_IN_NAME}` | **add** icon consts `ICON_NORMAL 0x28, ICON_UPLOADS 0x2b, ICON_DROPBOX 0x2c, ICON_PICTURES 0x34, ICON_AUDIO 0x35` (Task 9). |
| `tracker::{TrackerGroupList{groups}, GetServerList, TrackerServerList{servers}, TrackerSettings{sections, empty_offsets}, TrackerStats}` | `tracker::{GroupListEntry::decode_list, GetServerList, ServerListEntry::decode_list, TrackerSettings{present, sections}, TrackerStats, GeneralSettings, SettingsGroup, AdminSettings}` | `kdx-legacy`'s `tracker_stg` uses his `TrackerSettings` (Task 12). |
| `gconsole::*` | `gconsole::*` (his `DrawRecord` is an enum; ours is `{kind, body, padding}`) | ours drops with `openkdx-proto`; nothing in scope uses it. |
| `openkdx_codec::{SessionCodec, Event, Item, HandshakeCodec, HandshakeFrame, CodecError}` | `kdx_codec::{SessionCodec::{client, server, new}, Frame::{Message, DataHeader, Data}, HandshakeCodec, Handshake::{Hello{hello, key}, Proof}, Error}` | |
| `openkdx_client::{Session, Credentials, ConnectOptions, Identity, Connection (actor), Event, Transfer, Tracker, ClientError}` | `kdx_client::{Client, Login, Connection (&mut self), Event = (Header, Vec<u8>), Tracker, ServerError}` | Port target: **his model** (`&mut self`, request/reply by txid, `next_event()`); Task 8 adds `Notice` (typed events) and Tasks 9–10 add the missing `impl Client` methods. Phase 7 decides whether the GUI needs an actor over it. |

---

## File structure

```
../kdx (fork spitfire915/kdx; integration branch main)
  .github/workflows/lab.yml                 self-hosted lab job (skipped unless vars.KDX_LAB_RUNNER == 'true')
  captures/openkdx-<scenario>.{pcapng,md}   our 15 captures, renamed with the prefix (his first-connect exists)
  crates/kdx-proto/src/handshake.rs         Status::Failed
  crates/kdx-proto/src/msg/chat.rs          ChatText::flags, FLAG_ACTION
  crates/kdx-proto/src/msg/server_info.rs   typed ServerSettings (main, trackers, allow_deny, greeting, icon, chats, other, trailing, placeholders)
  crates/kdx-proto/src/msg/status.rs        error_name()
  crates/kdx-proto/src/msg/admin.rs         FolderAccess::ICON_* consts
  crates/kdx-proto/tests/openkdx_*.rs       one per fact our captures pin (chat /me, status 1, settings sections, upload resume, monitor transfers)
  crates/kdx-proto/tests/fixtures/openkdx-*.{c2s,s2c}.bin
  crates/kdx-dissect/src/main.rs            --inventory
  crates/kdx-dissect/tests/captures.rs      PASSWORDS += "openkdx-lab", "wrong-password"; EXPECTED += our disconnect-ban lines
  crates/kdx-client/src/lib.rs              pub mod mock (feature "mock"), pub mod notice, pub use Notice
  crates/kdx-client/src/mock.rs             MockServer, Step, Script, Captured (ported from openkdx-client/tests/common/mock.rs)
  crates/kdx-client/src/notice.rs           enum Notice + Client::next_notice()
  crates/kdx-client/src/{chat,news,files,accounts,folder_access,admin}.rs   impl Client blocks (moved out of client.rs + new)
  crates/kdx-client/tests/{chat,news,files,accounts,folder_access,admin,transfer}.rs   mock tests ported
  crates/kdx-client/tests/lab.rs            the 16 differential tests (KDX_LAB_SERVER, KDX_LAB_TRACKER, KDX_LAB_PASSWORD)
  crates/kdx-legacy/                        openkdx-legacy moved: src/{wire,time,civil,scramble,accounts,bases,catalog,client_stg,hap,history,news,news_archive,server_stg,store,tracker_stg,error,lib}.rs, examples/legacytool.rs, tests/*, tests/fixtures/ (50 files)
  docs/formats/*.md                         our format docs, in upstream's doc style
  docs/protocol/**                          our deltas folded in with evidence tags
  Cargo.toml                                + crates/kdx-legacy member and workspace dep

OpenKDX
  Cargo.toml                                workspace deps kdx-proto/-codec/-client/-legacy by git+rev; rust-version 1.96; members without the deleted crates
  .gitignore                                + /.cargo/config.toml
  .github/workflows/ci.yml                  cargo metadata --locked; msrv 1.96; lab job calls differential.sh/legacy.sh with the pinned checkout
  ci/check-deps.py                          rules renamed (openkdx-table/-cli/-ui/-app/openkdxd); forbids any openkdx-proto/-codec/-client/-dissect/-tracker dependency after Task 14
  ci/check-blocklist.py                     --root <dir> to scan the fork's tree
  lab/differential.sh [KDX_SRC]             runs crates/kdx-client/tests/lab.rs from KDX_SRC + the openkdx CLI lab scenarios; LAB_WAIT
  lab/legacy.sh [KDX_SRC]                   runs crates/kdx-legacy/tests/{lab,hap}.rs from KDX_SRC; LAB_WAIT
  lab/kdx-rev.sh                            prints the pinned kdx rev from Cargo.lock; syncs ~/kdx-src to it
  lab/lock-test.sh                          two concurrent lock takers; the second waits
  lab/lib/scenario.sh                       inventory via kdx-dissect --inventory from KDX_SRC
  crates/openkdx-table/src/lib.rs           load(story) -> kdx_proto::ChallengeTable; OPENKDX_KDX_TABLE=default
  crates/openkdx-cli/**                     on kdx_client::Client; tests on kdx_client::mock
  CONTRIBUTING.md, README.md, lab/README.md, captures/README.md, docs/protocol/README.md   the two-repository workflow
  docs/superpowers/plans/2026-11-13-phase4-server.md (branch phase4/server)   retargeted
```

---

## How a session runs

1. Read this plan, then `## Ledger`. Take the first unchecked task of the session listed there.
2. Sibling checkouts: `~/Documents/OpenKDX` (branch `phase3b/convergence`) and `~/Documents/kdx` (fork; `origin` = `spitfire915/kdx`, `upstream` = `scottanderson/kdx`). If `../kdx` is missing: `git clone git@github.com:spitfire915/kdx ../kdx && git -C ../kdx remote add upstream https://github.com/scottanderson/kdx`.
3. Local `[patch]` in OpenKDX's untracked `.cargo/config.toml` is on while porting; before any OpenKDX commit run `git diff --quiet Cargo.lock || echo "lockfile changed: only a pin commit may do that"`.
4. End of session: run both repositories' checks; push; append a ledger entry: date, tasks done, OpenKDX commits, fork PRs (`feat/*` → number, merged SHA), upstream PRs (`up/*` → number, state), the fork SHA OpenKDX now pins, rulings (anything decided that this plan left open or got wrong), carried items. Commit the ledger as `docs(plan): phase 3b ledger — session N`.

Phases: **3B** Tasks 1–3 (fork and plumbing) · **3C** Tasks 4–7 (upstream the evidence) · **3D** Tasks 8–11 (client port, lab gating) · **3E** Tasks 12–14 (legacy, CLI, deletion) · **3F** Task 15 (close). Review gates (Tasks 14 and any phase-end review) use **Opus 5.5** (`github-copilot/claude-opus-5.5`) in a fresh context.

---

## Phase 3B — fork and plumbing (Tasks 1–3)

### Task 1: Fork `scottanderson/kdx` to `spitfire915/kdx`; secrets, runner, blocklist over the fork

**Files:**
- Create: `../kdx` (clone), `lab/gh-secret.py` (OpenKDX), `.github/workflows/lab.yml` (fork)
- Modify: `ci/check-blocklist.py` (`--root`), `lab/README.md`, `lab/setup.sh` (second runner directory)

**Interfaces:**
- Produces: the fork at `https://github.com/spitfire915/kdx` with `main` = upstream `7f82081…` (or newer), remotes as in "How a session runs"; repository secrets `KDX_CHALLENGE_TABLE`, `HGC_CHALLENGE_TABLE` (base64 of `~/haxial/tables/kdx.bin`, `hgc.bin` on the lab); repository variable `KDX_LAB_RUNNER=true`; a second runner `~/actions-runner-kdx` on the lab machine, user `openkdx`, labels `self-hosted, lab`, registered to `spitfire915/kdx`; fork setting *Fork pull request workflows: require approval for all outside collaborators*.
- `lab/gh-secret.py REPO NAME < value`: reads the repo public key (`GET /repos/{repo}/actions/secrets/public-key`), seals with libsodium (`python3 -I`, `pynacl`; installed on the lab by `lab/setup.sh`), `PUT /repos/{repo}/actions/secrets/{name}`. Token from `git credential fill`. Runs **on the lab machine** so the table never leaves it.
- `python3 ci/check-blocklist.py --root DIR`: same checks over another tree.

- [ ] **Step 1:** `POST /repos/scottanderson/kdx/forks` (`default_branch_only: true`); poll `GET /repos/spitfire915/kdx` until 200. Clone to `../kdx`, add `upstream`, `git fetch upstream`, confirm `main == upstream/main`.
- [ ] **Step 2:** Fork's hosted CI needs the tables: `ssh openkdx-lab 'cd ~/openkdx-src && git pull && base64 -w0 ~/haxial/tables/kdx.bin | python3 -I lab/gh-secret.py spitfire915/kdx KDX_CHALLENGE_TABLE'` and the same for `hgc.bin`/`HGC_CHALLENGE_TABLE`. Write `lab/gh-secret.py` first; unit test `ci/gh_secret_test.py::seal_roundtrip` uses a generated keypair.
- [ ] **Step 3:** Variable: `POST /repos/spitfire915/kdx/actions/variables {name: KDX_LAB_RUNNER, value: "true"}`. Setting: `PATCH /repos/spitfire915/kdx` is not enough for fork-PR approval — do it in the web UI (Settings → Actions → General) and note it in the ledger.
- [ ] **Step 4:** Second runner on the lab: `POST /repos/spitfire915/kdx/actions/runners/registration-token`; on the lab as `openkdx`: `mkdir ~/actions-runner-kdx && cd $_ && tar xzf ../actions-runner/actions-runner-linux-x64-*.tar.gz && ./config.sh --url https://github.com/spitfire915/kdx --token … --labels lab --name openkdx-lab-kdx --unattended && sudo ./svc.sh install openkdx && sudo ./svc.sh start` (the `sudo` steps are the user's; put them in a ```bash block in the reply and wait). `GET /repos/spitfire915/kdx/actions/runners` lists it `online`. Document in `lab/README.md` (Layout table row `actions-runner-kdx/`, section "Two runners, one lock").
- [ ] **Step 5:** `.github/workflows/lab.yml` in the fork (commit on `feat/lab-workflow`): `on: [push: {branches: [main]}, pull_request]`; job `lab` with `if: vars.KDX_LAB_RUNNER == 'true' && (github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository)`, `runs-on: [self-hosted, lab]`; steps: checkout the fork; `actions/checkout@v4` with `repository: spitfire915/OpenKDX`, `ref: main`, `path: openkdx`; `env: LAB_WAIT=1`; `run: openkdx/lab/differential.sh "$GITHUB_WORKSPACE"` and `openkdx/lab/legacy.sh "$GITHUB_WORKSPACE"` (both scripts accept the argument from Task 11/13; until then the job prints "not yet wired" and exits 0 when the script lacks `--help kdx`: implement as `openkdx/lab/differential.sh --supports-kdx-src || exit 0`). Upstream, with the variable unset, the job is skipped and is not in `ci-status.needs`.
- [ ] **Step 6:** `ci/check-blocklist.py --root ../kdx` runs clean; add to the fork's `feat/*` routine in CONTRIBUTING (Task 14 writes the final text; add a one-line note now).
- [ ] **Step 7:** Commit OpenKDX `lab: second runner for the kdx fork; gh-secret.py; blocklist --root`; open fork PR for `lab.yml`, merge after hosted CI (the lab job is skipped until Task 11). Ledger: fork SHA.

---

### Task 2: OpenKDX depends on `kdx-*`; `openkdx-table` serves `kdx_proto::ChallengeTable`; MSRV 1.96; lockfile guard

**Files:**
- Modify: `Cargo.toml`, `rust-toolchain.toml` (unchanged channel; note), `.gitignore`, `.github/workflows/ci.yml`, `ci/check-deps.py`, `ci/check-deps_test.py`, `CONTRIBUTING.md`, `crates/openkdx-table/Cargo.toml`, `crates/openkdx-table/src/{lib.rs, story.rs}`, `crates/openkdx-table/tests/*.rs`

**Interfaces:**
- Produces (`Cargo.toml`): `kdx-proto = { git = "https://github.com/spitfire915/kdx", rev = "<sha>" }`, same for `kdx-codec`, `kdx-client`; `rust-version = "1.96"`.
- Produces (`openkdx-table`): `pub fn load(story: kdx_proto::Story) -> Result<kdx_proto::ChallengeTable, TableError>` — order: env `OPENKDX_KDX_TABLE`/`OPENKDX_HGC_TABLE` (base64; the literal value `default` returns `ChallengeTable::default()`, **test rigs only**, documented), then the cache; `Story` of this crate is replaced by `kdx_proto::Story` (`Kdx`, `Hgc`; rename `Gconsole` → `Hgc`, `file_name()` keeps `hgc.bin`). `TableError::Checksum(kdx_proto::challenge::ChallengeError)` wraps `from_bytes` failures.
- Produces (CI): `check` job step `cargo metadata --locked --format-version 1 > /dev/null` before everything else; `msrv` job toolchain `1.96`, name `MSRV (1.96)`.
- Produces (`.cargo/config.toml`, untracked, documented verbatim in CONTRIBUTING § "Working on kdx-* and OpenKDX together"):
  ```toml
  [patch."https://github.com/spitfire915/kdx"]
  kdx-proto = { path = "../kdx/crates/kdx-proto" }
  kdx-codec = { path = "../kdx/crates/kdx-codec" }
  kdx-client = { path = "../kdx/crates/kdx-client" }
  kdx-legacy = { path = "../kdx/crates/kdx-legacy" }
  ```
- `ci/check-deps.py`: `FORBIDDEN` keys become `openkdx-table` (UI set) and, as a new rule, every workspace member must not depend on a crate named in `RETIRED = {openkdx-proto, openkdx-codec, openkdx-client, openkdx-dissect, openkdx-tracker, openkdx-server}` **once that crate is no longer a member** (checked against `cargo metadata` workspace members, so the rule is inert until Task 14). Edition rule unchanged (2024) for members; git deps are not members.

- [ ] **Step 1: Failing tests.** `crates/openkdx-table/tests/load.rs`: `load_default_sentinel_returns_the_default_table` (`OPENKDX_KDX_TABLE=default` → `table.story() == None && table.offsets() == 12`), `load_rejects_a_table_with_a_wrong_checksum` (1739 random bytes base64 → `TableError::Checksum(ChallengeError::WrongChecksum(Story::Kdx))`), `load_without_env_or_cache_is_no_source` (`TableError::NoSource(Story::Kdx)`). A real table cannot be in a hosted test; the lab job covers `load` from the cache through `differential.sh`. `ci/check-deps_test.py::check_deps_rejects_patched_lock`: a temp lockfile with `source = "../kdx/crates/kdx-proto"`-style path entries for `kdx-proto` → `cargo metadata --locked` surrogate check in `check-deps.py` (`lock_names_fork_rev(lock_text) -> bool`: every `[[package]] name = "kdx-*"` has `source = "git+https://github.com/spitfire915/kdx?rev=…#…"`) returns the offending names.
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement. `cargo update -p kdx-proto` etc. with the patch **off** to write the lock; MSRV bump; CI edits; CONTRIBUTING section (patch file, "never commit a lockfile written under the patch", pin-bump commits).
- [ ] **Step 4:** `cargo test --workspace`, `python3 ci/check-deps.py`, `cargo +1.96 check --workspace --all-targets` (install if needed) → PASS. Push the branch; hosted CI green incl. `--locked`.
- [ ] **Step 5:** Commits: `build: depend on kdx-proto/-codec/-client from the spitfire915/kdx fork at <sha>; MSRV 1.96`, `feat(table): load() returns kdx_proto::ChallengeTable; default sentinel for test rigs`, `ci: --locked guard; check-deps rules for the fork model`.

---

### Task 3: Re-diff the 13 deltas and the capture inventory against upstream HEAD

**Files:**
- Create: `docs/protocol/convergence.md` (OpenKDX; a working document deleted in Task 14 after its rows have moved)

**Interfaces:**
- Produces: a table with one row per delta of `docs/protocol/README.md` ("Deltas" and the "Confirmed so far" rows that upstream does not state) with columns *Our claim · Upstream now says (file:line at `upstream/main` SHA) · Action (`already there` / `doc PR` / `code + doc PR` / `keep local`) · Owning task (4, 5, 6 or 9)*, and a second table listing the 15 captures with the facts each one is the evidence for and which upstream `tests/*.rs` fact file it feeds.

- [ ] **Step 1:** `git -C ../kdx log --oneline db2c148..upstream/main` and read `docs/protocol/{framing,handshake,tracker}.md`, `messages/{chat,files,news,accounts,administration,folder-access,status,monitoring}.md` at HEAD for each delta. Preliminary findings to verify, not to copy: 600/604 → Status is already upstream (`news.md:49-52`); proof status `1 failed` is in `handshake.md:376` as a byte but `Status` has no variant; 703 empty group is at `tracker.md:98`; `.prt` and 0x1000 rounding partly at `files.md:383-409`; the `/me` flag, LE phase-aligned padding, the 414 constant `0xfedc7e9f`, 401 empty `kind`, 171 byte 0x52, 511 `editor` rename quirk, 1100 padding, 155/151 flags and 143 transfer rows appear absent.
- [ ] **Step 2:** Write `docs/protocol/convergence.md`. Every row names the capture and frame (`[O openkdx-chat-basic 24, 25]` — the new names).
- [ ] **Step 3:** Commit `docs(protocol): convergence worksheet — deltas and captures against upstream <sha>`. Ledger: the `Action` counts.

---

## Phase 3C — upstream the evidence (Tasks 4–7)

### Task 4: Our 15 captures dissect byte-exact under `kdx-dissect`

**Files (fork):**
- Create: `captures/openkdx-{first-connect,guest-login,chat-basic,pm-basic,news-basic,files-basic,tracker-basic,upload-resume,accounts-classes,folder-access,server-settings,broadcast,disconnect-ban,server-history,connection-monitor}.{pcapng,md}`; `crates/kdx-proto/tests/fixtures/openkdx-<name>[-<port>].{c2s,s2c}.bin` for the connections a fact test reads (Task 5 chooses; at least `openkdx-chat-basic`, `openkdx-disconnect-ban-<port>` of the failed login, `openkdx-server-settings`, `openkdx-upload-resume-<port>` of the resumed upload, `openkdx-connection-monitor`)
- Modify: `crates/kdx-dissect/tests/captures.rs` (`PASSWORDS` += `"openkdx-lab"`, `"wrong-password"`; `EXPECTED` += the `openkdx-disconnect-ban <port>: stopped: no candidate key matches…` line for the wrong-password connection, if the dissector reports it so — verify), `captures/README.md` (15 rows: File · Scenario · Purpose · State lineage = "OpenKDX lab, fresh server"), `crates/kdx-proto/src/**` where `--audit` reports `mismatch`

**Interfaces:**
- Consumes: Task 3's capture table.
- Produces: `cargo test -p kdx-dissect` green with our captures in; a list in the ledger of every decoder change the audit forced (expected: none for well-modelled types; candidates are `Registration` extra fields on `openkdx-tracker-basic` frame 1, `ServerSettings` on 172 sending `(0,0)` for section 7, `MonitoredConnection` with a transfer row, `HistoryRecord::decode_added`).

- [ ] **Step 1:** Copy the files with the new names. Convert each `.md` to upstream's shape: `# Scenario: openkdx-<name>`, bullets (Date, Server, Client, Server state, Lab password `openkdx-lab`), `## Steps`, `## Observations` (our notes), and keep our `## Messages` inventory as the last section (Task 7 regenerates it with `kdx-dissect --inventory`). Note in each that the OpenKDX scenario script `lab/scenarios/<name>.sh` in spitfire915/OpenKDX produced it.
- [ ] **Step 2:** Run `cargo test -p kdx-dissect -- captures_dissect_cleanly_and_match_fixtures` → expect FAIL listing problems/mismatches; record them.
- [ ] **Step 3:** For each `mismatch`: fix the decoder in `kdx-proto` (fields as our struct names them in the mapping table), with the struct table in `docs/protocol/messages/*.md` updated in the same commit (`check-docs.py` enforces Rust↔table parity). For each `stopped`/`!` line that is a step of the scenario, add it to `EXPECTED` with a comment naming the step.
- [ ] **Step 4:** Export fixtures: `cargo run -p kdx-dissect -- captures/openkdx-<name>.pcapng --password openkdx-lab --export DIR`, copy the streams Task 5 needs; `cargo test -p kdx-dissect -p kdx-proto` → PASS. `python3 -I ci/check-docs.py`, `codespell`, clippy clean.
- [ ] **Step 5:** Commits on `feat/openkdx-captures` (one per decoder fix, one for the captures: "Add the OpenKDX lab captures" with the body explaining the lab, the published password and the prefix). Fork PR; merge. Then `up/openkdx-captures` → upstream PR. Ledger.

---

### Task 5: Protocol facts into `kdx-proto` and `docs/protocol`: `/me`, proof status 1, padding phase, `.prt`, 171 byte 0x52, 511 editor, Status replies

**Files (fork):**
- Modify: `crates/kdx-proto/src/msg/chat.rs` (`ChatText`), `src/handshake.rs` (`Status::Failed`), `src/session.rs` (only if Task 4 showed the fill phase differs), `docs/protocol/messages/chat.md`, `handshake.md`, `framing.md`, `messages/files.md`, `messages/administration.md`, `messages/accounts.md`, `messages/folder-access.md`, `messages/status.md`, `messages/monitoring.md`, `messages/news.md`
- Create: `crates/kdx-proto/tests/openkdx_chat_basic.rs`, `tests/openkdx_disconnect_ban.rs`, `tests/openkdx_upload_resume.rs`, `tests/openkdx_connection_monitor.rs`

**Interfaces:**
- Produces: `ChatText { chat_id: u32, user_id: u32, flags: u16, ignored_0a: [u8; 2], text: Vec<u8> }`, `ChatText::FLAG_ACTION: u16 = 1`, `pub const fn is_action(&self) -> bool`; `handshake::Status::Failed` ↔ byte `1` (before `Other`); doc rows with `<!-- [O openkdx-… N] -->` tags for every `doc PR` row of Task 3.

- [ ] **Step 1: Failing tests** (each reads its fixture through `common::session_with_key` as upstream's tests do): `openkdx_chat_basic.rs::me_action_sets_flag_1_and_strips_the_prefix` (frames 24/25: 207 with `flags == 1`, text without `/me `; 208 echoes `flags == 1`); `openkdx_disconnect_ban.rs::wrong_password_gets_proof_status_1_and_auth_word` (`Proof.status == Status::Failed`, `auth[0..4] == (nonce[4..8] LE ^ 0x201f7790)`, rest zero) and `ban_message_carries_flag_2` (151 `flags == DisconnectMessage::BANNED`); `openkdx_upload_resume.rs::server_offers_0x20000_after_a_150000_byte_cut` (416 `data_offset == 0x20000`) and `prt_partial_is_listed` (401 names `<name>.prt`); `openkdx_connection_monitor.rs::transfer_row_has_id_total_and_path` (143 with `transfer_id != 0`, `bytes_total == 4 MiB`, non-root `path`).
- [ ] **Step 2:** FAIL (field/variant missing; or, for pure-doc facts, the test passes at once — keep it as the evidence anchor).
- [ ] **Step 3:** Implement the code rows; write the doc rows in upstream style (claim, tag; "probably" where `[I]`). Where upstream already states the fact (Task 3 `already there`), add our capture as a second tag only if it adds a platform or a version.
- [ ] **Step 4:** `cargo test -p kdx-proto -p kdx-dissect -p kdx-client` (his `tests.rs` constructs `ChatText` — update), `check-docs.py`, `codespell` → PASS.
- [ ] **Step 5:** One commit per fact ("Name the /me action flag of 207 and 208", …) on `feat/protocol-deltas`; fork PR; merge; `up/protocol-deltas` → upstream PR (one PR; his triage is per topic but these share evidence). Ledger.

---

### Task 6: Typed `ServerSettings` upstream (sections 0–3, 5, 7 as structs)

**Files (fork):**
- Modify: `crates/kdx-proto/src/msg/server_info.rs`, `docs/protocol/messages/administration.md` (struct tables `ServerSettings`, `ServerSettingsMain`, `TrackerEntry`, `AddressRule`, `Greeting`), `crates/kdx-cli/src/bin/kdx.rs` (uses of `.body`/`NAME_AT` if any), `crates/kdx-dissect/src/audit.rs` (unchanged call), `crates/kdx-proto/tests/linux_server_settings.rs` (still passes)
- Create: `crates/kdx-proto/tests/openkdx_server_settings.rs`

**Interfaces:**
- Produces: `pub struct ServerSettings { pub version: u16, pub ignored: [u8; 2], pub value_64: [u8; 11], pub placeholders: [bool; 11], pub main: ServerSettingsMain, pub trackers: Vec<TrackerEntry>, pub allow_deny: Vec<AddressRule>, pub greeting: Option<ServerGreeting>, pub icon: Option<Vec<u8>>, pub chats: Vec<ChatRecord>, pub other: [Vec<u8>; 11], pub trailing: Vec<u8> }` with `decode(&[u8]) -> Result<Self, DecodeError>`, `encode(&self) -> Result<Vec<u8>, EncodeError>`, `pub fn sections(&self) -> [SettingsSection; 11]` (derived), `section(i) -> Vec<u8>` (re-encoded), and his accessors `flags()`, `greeting_enabled()`, `speed_limit()`, `name()`, `greeting()`, `icon()` kept with the same signatures. `ServerSettingsMain`, `TrackerEntry`, `AddressRule` as in `openkdx_proto::msg::admin` (field names unchanged; `Time1900` fields become `u64` ms). `chats: Vec<ChatRecord>`: section 7 is the concatenation of `ChatRecord`s with no count and no per-record padding (Phase 4 plan Task 2 evidence `[O lab legacy-samples 00-fresh]`; verified here on our `openkdx-server-settings` 171 and his `linux-server-settings`).
- Consumes: Task 4's fixtures.

- [ ] **Step 1: Failing tests.** `openkdx_server_settings.rs::settings_171_decode_typed_and_reencode_byte_exact` (name `OpenKDX Lab`, `main.port == 10700`, three stock `ChatRecord`s, re-encode == payload), `set_172_sends_0_0_for_section_7` (`placeholders[7] == false`, re-encode exact), `name_length_byte_0x52_is_the_name_length` (171 `main` byte 0x52 == `name.len()`; delta "171 byte 0x52"). His `linux_server_settings.rs` must stay green.
- [ ] **Step 2:** FAIL. **Step 3:** Port the decoder/encoder from `openkdx-proto/src/msg/admin.rs:456-1100` into upstream idiom (offset helpers; no `Reader`). Update the struct tables.
- [ ] **Step 4:** Workspace tests, `check-docs.py` (Rust parity will list every new pub field — rows for each), clippy → PASS.
- [ ] **Step 5:** Commit "Type the sections of 171 Server Settings Info" on `feat/typed-server-settings`; fork PR; merge; upstream PR. Ledger.

---

### Task 7: `kdx-dissect --inventory`; OpenKDX scenarios use it; `captures/` notes regenerated

**Files:**
- Modify (fork): `crates/kdx-dissect/src/main.rs` (`--inventory`), `docs/dissect/kdx-dissect.md`, the 15 `captures/openkdx-*.md` (`## Messages` regenerated)
- Modify (OpenKDX): `lab/lib/scenario.sh:212-223`, `lab/inventory.sh`, `captures/README.md`, the 15 `captures/*.md` (`## Messages` regenerated so both repositories' notes agree)

**Interfaces:**
- Produces: `kdx-dissect <pcapng> --inventory [--password …]` prints exactly the `## Messages` body that `openkdx-dissect --inventory` printed for the same capture (format of `openkdx_dissect::text::inventory`: one line per connection, then one line per message type with count and direction; copy the format from `crates/openkdx-dissect/src/text.rs` so the existing notes diff clean except for the tool name line).
- Produces (OpenKDX): `lab/lib/scenario.sh` runs `cargo run -q --manifest-path "$KDX_SRC/Cargo.toml" -p kdx-dissect -- "$CAPTURES/$SCENARIO.pcapng" --inventory --password "$KDX_ADMIN_PASSWORD"`, where `KDX_SRC=${OPENKDX_KDX_SRC:-$HOME/kdx-src}` is kept at the pinned rev by `lab/kdx-rev.sh` (Task 11).

- [ ] **Step 1: Failing test** (fork) `crates/kdx-dissect/tests/inventory.rs::inventory_of_openkdx_first_connect_matches_notes`: the `## Messages` section of `captures/openkdx-first-connect.md` equals the command's stdout (port of `openkdx-dissect/tests/captures.rs::inventory_matches_capture_notes`, over all 15 `openkdx-*` notes).
- [ ] **Step 2:** FAIL. **Step 3:** Implement `--inventory` (reuse the dissector's decoded records; no `--audit` output). Regenerate the 15 notes in both repositories.
- [ ] **Step 4:** PASS in the fork; in OpenKDX `crates/openkdx-dissect/tests/captures.rs::inventory_matches_capture_notes` still passes (same text) until Task 14 deletes it.
- [ ] **Step 5:** Fork commit "Add --inventory to kdx-dissect" on `feat/dissect-inventory`; PR; merge; upstream PR. OpenKDX commit `lab: scenario inventories from kdx-dissect`. Ledger.

---

## Phase 3D — client port and lab gating (Tasks 8–11)

### Task 8: `kdx_client::mock` (feature `mock`) and typed `Notice` events

**Files (fork):**
- Create: `crates/kdx-client/src/mock.rs`, `src/notice.rs`, `tests/common/mod.rs` (fixture helpers: `user(id, nick)`, `server_info(user_id)`, `login_steps()`, `client_with(steps) -> (MockServer, Client)`)
- Modify: `crates/kdx-client/Cargo.toml` (`[features] mock = ["tokio/macros", "tokio/rt"]`; the module is `#[cfg(any(test, feature = "mock"))] pub mod mock;` so the crate's own tests use it without the feature), `src/lib.rs`, `src/client.rs` (`login` accepts `ChallengeTable::default()` already; add `pub async fn next_notice(&mut self) -> Result<Notice>`), `README.md` (a "Testing against the mock" section)

**Interfaces:**
- Produces: `kdx_client::mock::{Step, Script, Captured, MockConfig, MockServer}` with the signatures of `openkdx-client/tests/common/mock.rs` translated: `Step::{Expect(MessageType), Reply(MessageType, Vec<u8>), Send(MessageType, Vec<u8>), DataReply { kind, reply, bytes }, ExpectData, Close, Sleep(Duration)}` (payloads as bytes — no `Message` enum upstream); `MockServer::start(accounts: Vec<(Vec<u8>, AccountKey)>, scripts: Vec<Script>) -> MockServer`, `start_as(sub_version, …)`, `start_with(MockConfig { sub_version: u8, reject: Option<handshake::Status> }, …)`, `finish(self)`, `check(&self)` (panics `mock script violated: …`), `payloads(&self, kind) -> Vec<Vec<u8>>`, `pub addr: SocketAddr`, `pub captured: Arc<Mutex<Captured>>`. Table on both sides: `ChallengeTable::default()`.
- Produces: `pub enum Notice { UserJoined(UserRecord), UserChanged(UserRecord), UserDeparted(UserDeparted), ChatText(ChatText), ChatUserJoined(ChatUserChange), ChatUserDeparted(ChatUserChange), ChatTopicChanged(ChatTopicChanged), ChatInvitation(ChatInvitation), ChatDeleted(ChatDeleted), PrivateMessage(PrivateMessage), Broadcast(RcvBroadcast), Disconnect(DisconnectMessage), NewsPosted(NewsMessageChange), NewsDeleted(NewsMessageChange), AccountDeleted(AccountLogin), FolderAccessDeleted(FolderAccessName), HistoryAdded(HistoryRecord), ServerIcon(ServerIcon), ServerGreeting(ServerGreeting), MustJoinChat, SpeedLimit(NotifySpeedLimit), Other(Header, Vec<u8>) }`; `impl Notice { pub fn decode(header: &Header, payload: &[u8]) -> Result<Notice, DecodeError> }`; `Client::next_notice()` = `next_event()` + `Notice::decode` (undecodable → `Other`).

- [ ] **Step 1: Failing tests** `tests/mock.rs::mock_runs_the_handshake_and_records_violations` (a script expecting 214 when the client sends 100 → `check()` panics with the type names), `tests/notice.rs::notices_decode_pushes_and_keep_unknown_as_other` (208, 301, 155, 151 and type 9999 pushed by the mock after login).
- [ ] **Step 2:** FAIL. **Step 3:** Port the mock from `openkdx-client/tests/common/mock.rs` onto `kdx_codec::{HandshakeCodec, SessionCodec, Frame}` (his `tests.rs::accept` is the seed); write `notice.rs`.
- [ ] **Step 4:** `cargo test -p kdx-client --features mock` and without the feature → PASS; `cargo doc` clean (`missing_docs = warn` is a workspace lint).
- [ ] **Step 5:** Commits "Add a scripted mock server behind the mock feature", "Decode server pushes as Notice" on `feat/client-mock-notice`; fork PR; merge; upstream PR. Ledger.

---

### Task 9: `kdx-client` features — files, chat, news, accounts/classes, folder access (mock-tested)

**Files (fork):**
- Create: `crates/kdx-client/src/{chat,news,files,accounts,folder_access}.rs`; `tests/{chat,news,files,accounts,folder_access}.rs`
- Modify: `src/client.rs` (move existing `impl Client` request methods into the new files; keep `login`, `send`, `request`, `send_checked`, `next_event`, `next_notice`, `download`, `upload` here), `src/lib.rs`, `crates/kdx-proto/src/msg/status.rs` (`error_name`), `src/msg/admin.rs` (`FolderAccess::ICON_*`), `docs/protocol/messages/status.md` (names table already there? if so `error_name` cites it), `docs/cli/kdx.md` untouched

**Interfaces:**
- Produces (all `impl Client`, `anyhow::Result`): chat — `say(chat_id, text, action: bool)` (sets `FLAG_ACTION`; `send_chat` becomes a thin wrapper), `set_topic(chat_id, topic)`, `create_chat(chat: &ChatRecord) -> Result<ChatRecord>` (219), `invite(chat_id, user_ids: &[u32], message)`, `delete_chat(chat_id)`; news — `newsgroup(group_id) -> NewsgroupInfo` (602→603), `create_newsgroup(&NewsgroupInfo) -> Result<Option<NewsgroupInfo>>` and `set_newsgroup(…) -> Result<Option<NewsgroupInfo>>` (Server 1.620 answers with a success Status → `None`; a 603 → `Some`), `delete_newsgroup(group_id)`, `post_news` returns `Result<u32>` (the 613's message id when it arrives before the barrier; else 0) — keep his `post_news(...) -> Result<()>` and add `post_news_id`; free fn `to_cr_lines`; files — `file_info(&FilePath) -> FileInfo`, `create_folder(parent, name, access, owner)`, `move_to_trash(folder, names: &[&[u8]])`, `empty_trash()`, `generate_catalog() -> Result<u32>` (451 items), `search(query) -> Vec<FileSearchEntry>`; accounts — `rename_account(current, account)`, `class(index) -> AccountClass` (502→503), `set_class(&AccountClass)` (504), free fns `new_account(login, name, password: Option<&[u8]>, class_index, privileges, privilege_mask) -> Account`, `parse_privileges(&str) -> Result<Privileges>`, `format_privileges(&Privileges) -> String`, `privilege_slug(Privilege) -> String`; folder access — `folder_access_list() -> Vec<FolderAccessItem>`, `folder_access(name) -> FolderAccess`, `create_folder_access(&FolderAccess)`, `set_folder_access(&FolderAccess)`, `delete_folder_access(name)`. Records with `padding` go out padded to 4 (CONTRIBUTING rule; `encode` upstream already does).
- Produces: `kdx_proto::msg::status::error_name(code: u32) -> Option<&'static str>` (the 79-name table from `openkdx_proto::msg::error_name`); `ServerError` gains `pub fn name(&self) -> Option<&'static str>`.
- `send_checked` semantics fixed (Review Focus 3): a Status with the request's txid and `Success{..}` is success; `Error{..}` is `ServerError`; a Status with another txid stays queued.

- [ ] **Step 1: Failing tests** — port `openkdx-client/tests/{chat,news,files,accounts}.rs` onto the mock; names kept; add `accounts.rs::success_status_reply_is_not_an_error_and_is_correlated_by_txid` (mock answers 507 with `Status::Success{value: 507}` after first pushing a `Status::Error` for a *different* txid; `create_account` returns Ok and `next_event()` yields the other Status).
- [ ] **Step 2:** FAIL. **Step 3:** Implement; split `client.rs`. **Step 4:** `cargo test -p kdx-client --features mock`, clippy, `cargo doc` → PASS; `kdx-cli` still builds.
- [ ] **Step 5:** Commits per area on `feat/client-features`; fork PR; merge; upstream PR. Ledger.

---

### Task 10: `kdx-client` administration and transfers to files (mock-tested)

**Files (fork):**
- Create: `crates/kdx-client/src/{admin,transfer}.rs`, `tests/{admin,transfer}.rs`
- Modify: `src/client.rs` (move `download`/`upload` to `transfer.rs`), `src/lib.rs`, `README.md`

**Interfaces:**
- Produces (`impl Client`): `server_settings() -> ServerSettings` (170→171), `set_server_settings(&ServerSettings)` (172→173), free `set_server_name(&mut ServerSettings, name) -> Result<(), EncodeError>` (also sets `main` byte 0x52 = name length), `broadcast(text)` (154; Status), `disconnect_user(user_id, ban_seconds, message, quiet: bool)` (150; Status), `history_join() -> HistoryList` (180→181), `history_leave()` (182), `monitor_open()`/`monitor_close()` (140/141), `monitor_info() -> Vec<MonitoredConnection>` (142→143), `shutdown(mode: u8, message)` (174), `extra_server_info() -> ExtraServerInfo` (113→114), `my_address() -> Vec<u8>` (115→116).
- Produces (`transfer.rs`): `pub struct Progress { pub done: u64, pub total: u64 }`; `Client::download_to(&mut self, path: &FilePath, dest: &Path, resume: bool, on_progress: impl FnMut(Progress)) -> Result<DownloadInfo>` (opens the file, resumes from its length when `resume`, calls his `download`), `Client::download_to_dir(…) -> Result<PathBuf>`, `Client::upload_from(&mut self, local: &Path, folder: &FilePath, name: &[u8], on_progress: impl FnMut(Progress), stop_after: Option<u64>) -> Result<()>` (`stop_after` cuts the stream for the lab's resume scenario); free `kind_for(name: &[u8]) -> Vec<u8>` (our MIME-ish table).
- `Notice::Disconnect` is surfaced by `request()` when the server closes after a 151: `ServerError`-like `anyhow` context "disconnected: <message>" (port of our `DisconnectedBy`).

- [ ] **Step 1: Failing tests** — port `openkdx-client/tests/{admin,transfer}.rs`; add `transfer.rs::upload_resumes_from_the_offered_offset` (mock replies 416 with `data_offset: 0x20000`; captured 417 data == source[0x20000..]).
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS; clippy; `cargo doc`.
- [ ] **Step 5:** Commits on `feat/client-admin-transfers`; fork PR; merge; upstream PR. Ledger.

---

### Task 11: Differential suite in the fork; lab gating for both repositories

**Files:**
- Create (fork): `crates/kdx-client/tests/lab.rs`; `crates/kdx-client/README.md` section "Lab tests"
- Create (OpenKDX): `lab/kdx-rev.sh`, `lab/lock-test.sh`
- Modify (OpenKDX): `lab/differential.sh`, `lab/legacy.sh` (argument + `LAB_WAIT`), `.github/workflows/ci.yml` (lab job: `lab/kdx-rev.sh sync` then `LAB_WAIT=1 lab/differential.sh "$HOME/kdx-src"`), `lab/README.md`
- Modify (fork): `.github/workflows/lab.yml` (drop the "not yet wired" guard)

**Interfaces:**
- Produces (fork `tests/lab.rs`): the 16 tests with their OpenKDX names, env `KDX_LAB_SERVER` (skip when unset), `KDX_LAB_TRACKER`, `KDX_LAB_PASSWORD` (default `openkdx-lab`), `KDX_LAB_TRACKER_HAS_SERVER`; table from `kdx_test_tables::table(Story::Kdx)`; serial (`--test-threads=1` is the runner's job; the README says so); ≤ 25 connections total.
- Produces (OpenKDX): `lab/kdx-rev.sh` → prints the `rev` of `kdx-proto` from `Cargo.lock`; `lab/kdx-rev.sh sync` → `git -C ~/kdx-src fetch origin && git checkout --detach <rev>` (clone from `https://github.com/spitfire915/kdx` if missing). `lab/differential.sh [KDX_SRC]`: `KDX_SRC` defaults to `$HOME/kdx-src`; builds `kdx-client --test lab` there (`cargo test --manifest-path "$KDX_SRC/Cargo.toml" -p kdx-client --test lab --no-run --message-format=json`), exports `KDX_CHALLENGE_TABLE` (base64 of the cached `kdx.bin`, as today's `OPENKDX_KDX_TABLE`), runs it inside the namespace with `KDX_LAB_*`, then the OpenKDX CLI lab scenarios that need a live server (`lab/scenarios/files-basic.sh` with `use_cli`, `accounts-classes.sh`) as a smoke of the application; keeps the licence-line grep. `--supports-kdx-src` exits 0 (Task 1's probe). Lock: `if [[ ${LAB_WAIT:-0} = 1 ]]; then flock -w 2400 9; else flock -n 9; fi`.
- Produces: `lab/lock-test.sh`: starts `differential.sh --lock-only` (holds the lock 5 s and exits) twice concurrently with `LAB_WAIT=1`; both exit 0; total time ≥ 5 s.

- [ ] **Step 1:** Port `lab.rs`; run locally against the lab: `ssh openkdx-lab` … `cd ~/openkdx-src && git pull && lab/differential.sh ~/kdx-src` with `~/kdx-src` on the `feat/lab-suite` branch (`lab/kdx-rev.sh` accepts an explicit ref: `lab/kdx-rev.sh sync feat/lab-suite`). All 16 pass; server log has no licence line.
- [ ] **Step 2:** `lab/lock-test.sh` on the lab → both runs succeed, second waited.
- [ ] **Step 3:** Fork PR `feat/lab-suite` (the lab job now runs: it checks out OpenKDX `main`, which must already contain the new `differential.sh` — so **merge the OpenKDX side first**: push `phase3b/convergence` and temporarily point `lab.yml`'s OpenKDX checkout `ref` at `phase3b/convergence`; Task 14 points it back at `main`). Both lab jobs green.
- [ ] **Step 4:** Upstream PR `up/lab-suite` for `tests/lab.rs` only (skips without `KDX_LAB_SERVER`; `lab.yml` goes too — it is inert without the variable — in the same PR with a body explaining the OpenKDX lab).
- [ ] **Step 5:** Commits: OpenKDX `lab: differential and legacy suites run from the kdx checkout; LAB_WAIT; kdx-rev.sh`, `ci: lab job tests the pinned kdx rev`. Ledger: pin bump `build: pin kdx-* at <sha>`.

---

## Phase 3E — legacy, CLI, deletion (Tasks 12–14)

### Task 12: `kdx-legacy` in the fork, on `kdx-proto` wire structs; fixtures byte-exact

**Files (fork):**
- Create: `crates/kdx-legacy/{Cargo.toml, README.md, src/lib.rs, src/wire.rs, src/time.rs, src/civil.rs, src/scramble.rs, src/error.rs, src/accounts.rs, src/bases.rs, src/catalog.rs, src/client_stg.rs, src/hap.rs, src/history.rs, src/news.rs, src/news_archive.rs, src/server_stg.rs, src/store.rs, src/tracker_stg.rs, examples/legacytool.rs, tests/*.rs, tests/fixtures/** (the 50 files of fixtures/legacy/ with their READMEs)}`, `docs/formats/*.md` (our 12 files)
- Modify: `Cargo.toml` (member + `kdx-legacy.workspace`), `README.md` (crate table), `ci/check-docs.py` only if a protocol-only rule misfires on `docs/formats` (exempt by path for that rule alone, with a comment)

**Interfaces:**
- Produces: `kdx_legacy::wire::{Reader, Writer, pad4_len, DecodeError}` (our `wire.rs`, `DecodeError::{Truncated, Invalid, Trailing}` local to this crate), `kdx_legacy::time::Time1900` (our `common::Time1900`), `Civil`, and every public item of `openkdx-legacy` under the same names; imports become `kdx_proto::msg::admin::{Account, AccountClass, FolderAccess}`, `kdx_proto::msg::news::{NewsMessage, NewsgroupInfo}`, `kdx_proto::msg::server_info::ServerSettings` (typed, Task 6), `kdx_proto::msg::tracker::TrackerSettings`, `kdx_proto::msg::Color`, `kdx_proto::AccountKey::EMPTY_PASSWORD`. Where a `kdx-proto` struct lacks a field ours had (`AccountClassInfo.editor_login` → `AccountClass.editor`; `FolderAccess.class_rules` as `u32`), `kdx-legacy` adapts; where a wire struct must gain a field for the file to round-trip, that is a `kdx-proto` change in the same PR with doc rows.
- Produces: `KDX_LAB_LEGACY_DIR`, `KDX_LAB_HAP_DIR` replace the `OPENKDX_` names in `tests/{lab,hap}.rs`; `lab/legacy.sh [KDX_SRC]` (OpenKDX) runs them from `KDX_SRC` and builds `legacytool` there.
- Produces: `docs/formats/*.md` pass `check-docs.py`: struct tables in the Conventions shape (`Struct \`Name\` (N bytes):<a id="sName"></a>`), evidence as HTML comments, relative links, reachable from the fork's `README.md`; Rust-parity applies only to `crates/kdx-proto/src` so legacy structs need no parity rows.

- [ ] **Step 1: Failing test** `crates/kdx-legacy/tests/fixtures.rs::every_fixture_round_trips_byte_exact` (walks `tests/fixtures/**`, decodes by file name — `Accounts.dat`, `*.stg` by magic, `News.dat`, `Catalog.dat`, `History/*.txt`, `News Archives/**.txt` — re-encodes, compares) plus the ported per-format test files.
- [ ] **Step 2:** FAIL (crate absent). **Step 3:** `git mv`-equivalent port (copy, then sed the imports per the mapping; keep our comments). **Step 4:** `cargo test -p kdx-legacy`, clippy (`missing_docs` warns → document every pub item), `check-docs.py`, `codespell` → PASS. On the lab: `lab/legacy.sh ~/kdx-src` (branch) → the `.hap` pack and the lab snapshots round-trip; Server 1.620 and Client 1.600 restart on files `kdx-legacy` wrote.
- [ ] **Step 5:** Commits "Add kdx-legacy: the original programs' files, read and written byte for byte" (+ docs) on `feat/kdx-legacy`; fork PR (lab job green); merge; upstream PR. OpenKDX: `Cargo.toml` adds `kdx-legacy`; pin bump. Ledger.

---

### Task 13: `openkdx-cli` onto `kdx-client`

**Files (OpenKDX):**
- Modify: `crates/openkdx-cli/Cargo.toml` (deps `kdx-client`, `kdx-proto`, `openkdx-table`; dev `kdx-client = { …, features = ["mock"] }`), `src/{main,args,out}.rs`, `src/cmd/*.rs`, `tests/cli.rs` (`use kdx_client::mock::{MockServer, Step, Script}`; no `#[path]`), `README.md`

**Interfaces:**
- Consumes: Tasks 8–10's `Client`, `Notice`, `mock`; Task 2's `openkdx_table::load(Story::Kdx)`.
- Produces: the same subcommand tree and output (`--json` lines unchanged; `out::event` takes `&Notice`); `Ctx::client() -> Result<Client>` replaces `session()`; `chat watch`/`pm wait` loop on `client.next_notice()` under `tokio::select!` with the timeout; `files get/put` call `download_to`/`download_to_dir`/`upload_from`; the hidden `--insecure-default-table` sets `OPENKDX_KDX_TABLE=default` semantics via `openkdx_table::load`. `openkdx run` keeps one `Client` across lines (lab scenarios depend on it).

- [ ] **Step 1:** The 12 tests in `tests/cli.rs` are the failing tests once the imports change; add `login_without_a_table_explains_how_to_get_one` (exit 2, message names `openkdx-table fetch`).
- [ ] **Step 2:** FAIL. **Step 3:** Port. **Step 4:** `cargo test -p openkdx-cli`, clippy → PASS; on the lab, `lab/scenarios/files-basic.sh` and `accounts-classes.sh` (CLI-driven) reproduce their captures' inventories (diff `## Messages`).
- [ ] **Step 5:** Commit `refactor(cli): openkdx on kdx-client`. Ledger.

---

### Task 14: Delete the duplicates; rewrite the two-repository docs; pin; OpenKDX PR

**Files (OpenKDX):**
- Delete: `crates/openkdx-proto`, `crates/openkdx-codec`, `crates/openkdx-dissect`, `crates/openkdx-client`, `crates/openkdx-tracker`, `crates/openkdx-server` (empty stub; Phase 4 builds `kdx-server` in the fork), `crates/openkdx-legacy`, `fixtures/legacy/`, `docs/formats/` (now in the fork; `docs/formats/README.md` becomes a one-paragraph pointer), `docs/protocol/convergence.md`
- Modify: `Cargo.toml` (members, workspace deps), `crates/openkdx-ui/Cargo.toml` (`kdx-legacy`), `crates/openkdx-app/Cargo.toml` (`kdx-client`), `crates/openkdxd/Cargo.toml` (none yet; `kdx-legacy`), `ci/check-deps.py` (`RETIRED` rule now active), `.github/workflows/ci.yml` (lab job: drop the `openkdx-proto/-codec/-dissect` test step), `CONTRIBUTING.md` (sections "Protocol code", "Client code", "Format code" → one section "Working in the kdx fork" naming the branch model, upstream's conventions, the trailer, the blocklist run over `../kdx`, the pin-bump rule, `lab/kdx-rev.sh`; "Checks" adds `cargo metadata --locked`), `README.md` (crate table: `kdx-proto/-codec/-client/-legacy` "in the fork", status line), `docs/protocol/README.md` (keep the reference link, the evidence-tag section, the "Deltas" table reduced to rows whose upstream PR is **not yet merged**, each with its PR link; "Known inexact" row dropped with `Draw`), `captures/README.md` ("the same bytes are `captures/openkdx-<name>.pcapng` in the fork, where `kdx-dissect` tests them; decode with `cargo run --manifest-path ../kdx/Cargo.toml -p kdx-dissect -- captures/<name>.pcapng --password openkdx-lab --audit`"), `lab/README.md`, `.github/workflows/lab.yml` in the fork (`ref: main`)

- [ ] **Step 1:** Delete; fix the stubs' deps; `cargo build --workspace`, `cargo test --workspace`, `python3 ci/check-deps.py` (the `RETIRED` rule passes), `ci/check-blocklist.py`, `ci/*_test.py` → PASS. `cargo metadata --locked` passes with the patch off.
- [ ] **Step 2:** Docs as listed. **Step 3:** Pin bump to the fork `main` SHA that contains Tasks 4–12. Push; all hosted jobs and the lab job green (~25 min).
- [ ] **Step 4:** Review gate: fresh-context reviewer (Opus 5.5, `github-copilot/claude-opus-5.5`) on `main..phase3b/convergence` and on the fork's `main` since the fork point, with this plan, the spec §6.1/§8/§9 and both CONTRIBUTING files in the prompt. Fix; commit.
- [ ] **Step 5:** Open the OpenKDX PR (`POST /repos/spitfire915/OpenKDX/pulls`, base `main`, head `phase3b/convergence`, title `Phase 3b: convergence on the kdx fork`, body: what moved where, PR numbers upstream, what the lab gates now, deferred items); link it with `session.link`. Squash-merge as #4/#5 did; delete the branch; `git fetch --prune` here and on the lab; lab clone on `main`; fork `lab.yml` `ref: main` PR merged.

---

## Phase 3F — close (Task 15)

### Task 15: Spec note, open upstream PRs, hand-off

**Files:**
- Modify (OpenKDX `main`): `docs/superpowers/specs/2026-11-08-openkdx-design.md` (§8 `**Phase 3b complete:** 2026-11-xx, branch \`phase3b/convergence\` (PR #6). …` after Phase 3; §7.1 workspace block: the `kdx-*` lines marked "in spitfire915/kdx"), `README.md` status line, this plan's `## Ledger` (final entry: table of upstream PRs and their states)

- [ ] **Step 1:** `GET /repos/scottanderson/kdx/pulls?state=all&head=spitfire915:` → list; for each open one, note what review asked for; nothing is blocked on them (OpenKDX pins the fork).
- [ ] **Step 2:** Commit `docs: README and spec reflect Phase 3b` on `main`; push; lab `git pull`.
- [ ] **Step 3:** Ledger final entry. **Phase 3F ends the convergence.** Phase 4 begins with its own session that retargets `docs/superpowers/plans/2026-11-13-phase4-server.md` (branch `phase4/server`) to `kdx-server` in the fork + `openkdxd` here, using the mapping table above; see "Phase 4: what comes next" below.

---

## Open points (decide at execution, record the ruling in the ledger)

1. **Trailer wording in the fork.** Upstream's `AGENTS.md` asks for the model's name (`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`); the user mandated `Co-Authored-By: Claude <noreply@anthropic.com>`. Default: the mandated form in both repositories; if upstream review asks, amend the `up/*` branch only.
2. **Upstream review pace.** `up/*` PRs may sit; nothing here waits on them. If upstream asks for changes, make them on `up/*`, then merge `upstream/main` into fork `main` after his merge (the cherry-picks resolve as empty).
3. **Fork-PR CI upstream fails without secrets** (his `check` job needs the tables): expected; say so in each upstream PR body ("tests pass with the tables; CI on fork PRs cannot see the secrets").
4. **Client model for Phase 7.** The GUI needs sends while receiving; his `&mut self` client needs either `split()` or an actor. Not decided here; `Notice` keeps the typed surface either way.
5. **Edition/MSRV drift**: if upstream raises MSRV again, OpenKDX follows at the next pin bump (`rust-version` and the `msrv` job in the same commit).

## Self-review notes

- **Spec coverage.** §6.1 amendment, sentence by sentence: fork (Task 1); modify the crates there and upstream every change (Tasks 4–12, each ends in an `up/*` PR); depend through Cargo with `--path`/`[patch]` locally and a pinned rev in CI (Task 2); convergence before the server (the Phase 4 retarget is Phase 4's first session, below); upstream 13 deltas (Tasks 3, 5, 6) and 15 captures (Task 4); port client features (Tasks 8–10); `kdx-legacy` (Task 12); delete the duplicates (Task 14); `openkdx-table` stays (Task 2); §5.3 on both repositories (Task 1's blocklist `--root`, Global Constraints); the lab unchanged and gating both (Task 11). §8 item 3b and §9 first bullet: Task 15 records completion.
- **Types across tasks.** `Notice`, `mock::{MockServer, Step}` (Task 8) are what Tasks 9, 10, 13 use; `ChatText::FLAG_ACTION` (Task 5) is what `say` (Task 9) sets; typed `ServerSettings` (Task 6) is what `set_server_name` (Task 10) and `server_stg` (Task 12) use; `openkdx_table::load` (Task 2) is what the CLI (Task 13) and `differential.sh` (Task 11) use; `lab/kdx-rev.sh` (Task 11) is what `scenario.sh` (Task 7) and CI (Task 14) call — Task 7 lands before Task 11, so Task 7 reads `OPENKDX_KDX_SRC` with the `~/kdx-src` default and Task 11 adds the sync.
- **Order risk.** Task 11's fork lab job needs OpenKDX's new scripts on a reachable ref before OpenKDX merges; the plan points `lab.yml` at `phase3b/convergence` until Task 14. Task 12 depends on Task 6 (typed settings); Task 13 on 8–10; Task 14 on everything green.
- **Proportion.** The plan names files, signatures, env variables, branch names and the mapping; bodies are ports of code that exists in `openkdx-*` and are not reproduced.

## Phase 4: what comes next (not part of this plan)

Phase 4 is the existing plan on branch `phase4/server` (`docs/superpowers/plans/2026-11-13-phase4-server.md`, 16 tasks), rewritten onto `kdx-server` in the fork + `openkdxd` here. A session breakout sized for fresh context windows is in the hand-off message of 2026-11-19 and is repeated here so it travels with the plan:

| Session | Tasks of the Phase 4 plan | Why this cut |
|---|---|---|
| 4-0 Retarget | rewrite the plan: crate/paths/deps, type names per this plan's mapping table, `kdx_client::{Client, mock}`, `kdx-test-tables`, fork branch model, PR #7, lab scripts from `~/kdx-src` | docs only; needs this plan's mapping table and the fork at the pinned rev in context, nothing else |
| 4-1 Foundation | 1 (errors, config, log lines), 2 (stock data, setup), 3 (state, handshake, sessions, login) | one crate skeleton; the interfaces every later task consumes |
| 4-2 Milestone 1 | 4 (`openkdxd`, original clients log in; review gate 1) | lab-heavy; one task alone because of the lab round trips |
| 4-3 Content | 5 (chat/PM), 6 (news), 7 (accounts/classes/folder access) | handlers over `State`; each is a port of a captured exchange |
| 4-4 Files | 9 (paths/listing/folders), 10 (transfers), 11 (catalog/search) | the file subsystem; 10 is the biggest single task |
| 4-5 Admin and limits | 8 (settings, broadcast, disconnect, monitor, history, processes, shutdown), 12 (flood/bans), 13 (UDP registration) | administration and the accept path |
| 4-6 Acceptance | 14 (differential suite + original clients against `openkdxd`) | lab-heavy; expect fixes across crates |
| 4-7 Ship | 15 (Docker, systemd, launchd, docs), 16 (docs, review gate 2, PRs, merge) | packaging and closing |

## Ledger

_(appended by each session; newest last)_

| Phase | Date | Tasks | OpenKDX commits | Fork PRs (feat → #, merged SHA) | Upstream PRs (up → #, state) | Pinned fork SHA | Rulings / carried |
|---|---|---|---|---|---|---|---|
| 3B | 2026-11-20 | 1–3 | `9fabeb0` lab/gh-secret/blocklist, `b7fd5ad` deps+MSRV, `d11fd76` table::load, `73e97de` ci guard, `9a61b39` worksheet; draft PR #6 | `feat/lab-workflow` → kdx#1, `5b176ca` | none yet (lab.yml goes up with Task 11) | `5b176ca` | Token was OpenKDX-only: fork made in the UI, token extended; fork-PR approval setting requested in the UI, unverified. Runner needed the proxy `.env`. `openkdx-table` keeps its own `Story` (+ `From` into `kdx_proto::Story`). Workspace `openkdx-table` is `default-features = false` (cargo 1.96). Worksheet: 3 already upstream, 6 doc PRs, 3 code+doc PRs. |
| 3C | 2026-11-20 | 4–7 | `6e4aa29` scenario inventories via kdx-dissect; pin bump | kdx#2 → `da478f3` (captures + facts), kdx#3 → `e728c5d` (typed ServerSettings), kdx#4 → `a6b3a41` (--inventory) | branches `up/openkdx-captures`, `up/typed-server-settings`, `up/dissect-inventory` pushed; **the user opens them** (fine-grained PAT cannot) | `a6b3a411d279118c57f7a70e896d6d327101e01f` | No decoder changes were needed for the 15 captures; padding phase already in `seal()`; 171 from Server 1.620 carries no chat rooms (chats typing proven by round trips only); inventories now in kdx-dissect form, OpenKDX notes regenerate in 3E; OpenKDX `captures/*.md` untouched. |
| 3D | 2026-11-20 | 8–11 | `99d26c6` lab scripts take the kdx checkout, LAB_WAIT, kdx-rev.sh, lock-test.sh; `eade54b` ci lab job syncs the pin; `a59b3a3` pin bump; `6e4d033` ci-smoke waits under LAB_WAIT | kdx#5 → `1ce944c` (mock + Notice), kdx#6 → `cdd73d6` (client split + features, error_name, ICON_*), kdx#7 → `8e45933` (admin + transfers), kdx#8 → `68c9876` (lab suite; the fork's lab job now runs it) | `up/client-mock-notice` → scottanderson/kdx#92, `up/client-features` → #93, `up/client-admin-transfers` → #94, `up/lab-suite` → #95 (all open; stacked on #90 typed settings → #89 captures) | `68c9876ec3152fd4d6acd9645c793dc1c4b72df1` | kdx-client takes a dev-dep on itself with `mock` so `cargo test` needs no flag; `MockServer::finish(&self)` checks, `wait(&self)` does not; 154 Send Broadcast has no reply (capture), so `broadcast` uses the 115 barrier — plan said Status; 417 header keeps upstream's form (fresh txid); download discards the resource fork; `upload_from` always resumes; `post_news_id` returns 0 without a 613 (no timeout in his model); `lab/differential.sh` runs the kdx-client suite then the openkdx-client suite on separate server starts (≤25 connections each), skips the kdx suite on a checkout without `tests/lab.rs`; the CLI lab scenarios are not run from it (they take the lock themselves and rewrite `captures/`) — carried to Task 13; `legacy.sh` runs kdx-legacy only once the checkout has it (Task 12); `lab.yml` still points at `phase3b/convergence` (Task 14 points it at `main`). Lab: 16/16 + 16/16, lock-test ok (second waited 124 s). |
| 3E | 2026-11-20 | 12–14 | `0c4ae12` legacy.sh uses kdx-legacy's legacytool; `79f436f` CLI on kdx-client; `bd1927a` legacy.sh holds the lock for its whole run; `1c6726a` jobs take over a leftover namespace; `4ea54ed` pin bump; `8587e6f` delete the openkdx-* crates; `1eca342` two-repository docs; `4ee1c75` watch keeps the error cause; PR #6 ready for review and merged | `feat/kdx-legacy` → kdx#9, `6fb82b4` | `up/kdx-legacy` pushed (stacked on `up/lab-suite`; **the user opens it**); #89–#95 still open | `6fb82b40362dfef537411aabd691e296e4f7e264` | kdx-legacy: `encode_accounts`/`encode_news` return `Result` (kdx-proto's record encoders fail on overlong strings); `validate_settings` replaces `ServerSettings::validate`; `LegacyError::{Proto, Encode}` added; docs/formats evidence as `<!-- [O] -->` with the fixture named in prose; en-US spellings. CLI: typed errors drive the exit codes (HandshakeError/ServerError/NoTable → 2, Disconnected → 3, Elapsed → 4), each request under `--timeout`; `put --resume` accepted and ignored; `--table-cache` hidden option. Lab: the fork's lab job on kdx#9 failed because legacy.sh released the lock before its scenarios (Review Focus 5 gap) — fixed by holding fd 9 for the whole run (`OPENKDX_LAB_LOCK_HELD`), and jobs now stop a leftover namespace instead of refusing. CLI lab check: accounts-classes.sh and upload-resume.sh reproduce the server-side inventories (client differs by the absent 118 and delete_account's 115 barrier). Review (Opus 5.5): with fixes — watch error collapse fixed; CLI smoke re-graded minor (legacy.sh exercises the CLI every run). PR #6 squash-merged as `c5cde4d`; fork `lab.yml` → `ref: main` merged as kdx#10 (lab job green against OpenKDX `main`; workflow only, no pin bump). Carried: deferred minors in the SDD ledger (post_news_id → Option, differential.sh's pre-lab.rs skip branch, .hap skip under CI, trailer case). |
