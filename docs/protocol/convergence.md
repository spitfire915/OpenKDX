# Convergence worksheet: our deltas against upstream

Working document for Phases 3C–3E (master plan Task 3). Rows move into the
fork's `docs/protocol/` and `kdx-proto` and are then struck here; the file is
deleted in Phase 3E (Task 14). Upstream state read at `scottanderson/kdx`
`5b176ca`/`7f82081` (2026-11-20; the fork's `main` equals upstream `main` plus
`lab.yml`). Captures are cited by their fork names (`openkdx-<scenario>`).

## Deltas

| # | Our claim (`docs/protocol/README.md`) | Upstream now says | Action | Task |
|---|---|---|---|---|
| 1 | 207/208 bytes 0x08–0x0b: `u16` at 0x08 is 1 for a `/me` action; the client strips `/me ` | `messages/chat.md:214` `ChatText.ignored: u8[4]`; no action flag anywhere | **code + doc PR**: `ChatText { flags: u16, ignored_0a: [u8;2] }`, `FLAG_ACTION = 1`; tag `[O openkdx-chat-basic 24, 25]` | 5 |
| 2 | Payload padding word laid down little-endian, phase-aligned to the payload offset (`byte i = word.to_le_bytes()[i % 4]`) | `framing.md:22-24` "repeated 32-bit memory words", no byte order or phase; `session.rs::padding_fill` returns LE bytes — whether `seal` phases them from the payload start is checked by the audit | **verify in Task 4**; doc clarification PR naming the byte order and phase, `[O openkdx-first-connect 10, 13; openkdx-files-basic 16, 28]` | 4, 5 |
| 3 | 414 data padding is `0xfedc7e9f` LE for txid `28814ca0`, not the txid LCG | `framing.md:26-31` "a different repeated 32-bit value" | **doc PR** adding the observed value as a data point (formula still open) | 5 |
| 4 | 401 `kind` is empty for `tiny.txt`; 413 gives `text/plain` → server has a MIME table | `files.md:82-86` "Empty for folders. `exttype/bin` for a `.bin` file" | **doc PR**: "also empty for a `.txt` file whose 413 says `text/plain`" `[O openkdx-files-basic 16, 28]` | 5 |
| 5 | Server 1.620 answers a wrong password with proof status `1`, auth word `nonce[4..8] ^ 0x201f7790`, rest zero, then closes; log `Login failed` | `handshake.md:376` documents `1 failed` for the **0.9x** one-byte reply only; `handshake::Status` has `Other(u8)` | **code + doc PR**: `Status::Failed` (= 1) in the 1.x proof; `[O openkdx-disconnect-ban 9]`; fact test `wrong_password_gets_proof_status_1_and_auth_word` | 5 |
| 6 | 703 for an empty group is a 0-byte payload | `messages/tracker.md:97-98` "The payload is empty for an empty group" | **already there** | — |
| 7 | 415/417 resume: partial kept as `<name>.prt` (401 lists it); written in large chunks (150000 cut → 131072 on disk, 416 offers 0x20000; 20000 cut → 0-byte `.prt`, offset 0) | `files.md:383, 409`: resumes a partial upload, offset rounded down to 0x1000; nothing on `.prt` or the chunking | **doc PR**: the `.prt` name and the two observations `[O openkdx-upload-resume 61, 88]`; fact tests in `openkdx_upload_resume.rs` | 5 |
| 8 | 171 main byte 0x52 is the name length; a 172 that renames without updating it leaves the old length | `administration.md:227` `ignored_52` "Probably the name's length `[I]`" | **code + doc PR** as part of the typed `ServerSettings` (`name_len` derived on encode); tag `[O openkdx-server-settings 10, 29]` + lab round trip | 6 |
| 9 | 511 `editor` holds the current login; 510 returns the *last editor*, so echoing a 510 as 511 asks for a rename (`0x1000d`) | `accounts.md:331-333` renames on a different login and fails `0x1000d` if it exists; does not say 510's `editor` is the last editor | **doc PR**: one sentence on 510's `editor` and the echo trap `[O openkdx-accounts-classes]` | 5 |
| 10 | 600 Create / 604 Set Newsgroup are answered by a success Status, not 603 | `news.md:49-52, 71-79` Status 0x258 / 0x259 | **already there** | — |
| 11 | 1100 record must be padded to 4 or `0x1000a`; same for 507/511/504 | `folder-access.md:33`, `accounts.md:32, 119, 366` | **already there** (confirmation; add the `0x1000a` consequence if absent: `grep 0x1000a` found none) | 5 |
| 12 | 155 reaches every session incl. the sender; 151 before the close on a ban carries flag 2; 150 answered by a success Status | `administration.md:61` flags 2 banned; `:132, 172` sender receives the broadcast | **already there** for 155 and 151 flag; verify "150 → success Status" and add if absent | 5 |
| 13 | 143 lists a transfer with `transfer_id != 0`, `bytes_total` and the path beside the sessions | `monitoring.md:69, 74` `[S]` only | **doc PR** adding the `[O openkdx-connection-monitor 70]` observation; fact test `transfer_row_has_id_total_and_path` | 5 |

Also from our "Confirmed so far" table, not stated upstream: the server's own
ban list does not apply to 127.0.0.1 — `sequences.md:67` has it `[S]`; add
`[O openkdx-disconnect-ban 78–87]` (Task 5). 118 `{ value: 1 }` suppresses the
licence line — `session.md` 118 says a reminder follows an unregistered
client; add the registered observation from `lab/differential.sh` (Task 5).

Counts: already there 3 (6, 10, 11), doc PR 6 (2, 3, 4, 7, 9, 13), code + doc
PR 3 (1, 5, 8), verify first 1 (2, counted under doc).

## Captures

| Fork name | Evidence for | Upstream fact test (Task 5) / fixture |
|---|---|---|
| openkdx-first-connect | handshake, 100/101/105, padding phase (#2) | fixture `openkdx-first-connect.{c2s,s2c}.bin` |
| openkdx-guest-login | guest key, `session_key` layout | — |
| openkdx-chat-basic | #1 `/me` flag, 209/210 topic | `openkdx_chat_basic.rs` |
| openkdx-pm-basic | 300/301 both ways, 101 with two users | — |
| openkdx-news-basic | 605–611 | — |
| openkdx-files-basic | #2, #4, transfer connection keyed by 105 session key, two 414 headers | fixture (both connections) |
| openkdx-tracker-basic | UDP registration (84 bytes), 700–703 | registration fixture check as `linux-tracker-reg-*` |
| openkdx-upload-resume | #7 | `openkdx_upload_resume.rs` |
| openkdx-accounts-classes | #9, 502–515 | — |
| openkdx-folder-access | #11, 1100–1107, 402 with `access` | — |
| openkdx-server-settings | #8, 170–173, fresh 105 | `openkdx_server_settings.rs` (Task 6) |
| openkdx-broadcast | #12 (155 at sender) | — |
| openkdx-disconnect-ban | #5, #12 (151 flag 2), loopback not banned | `openkdx_disconnect_ban.rs` |
| openkdx-server-history | 180–183, guest post refused | — |
| openkdx-connection-monitor | #13 | `openkdx_connection_monitor.rs` |

`kdx-dissect` passwords to add: `openkdx-lab`, `wrong-password` (the failed
login in `openkdx-disconnect-ban` is expected to report `no candidate key`).
