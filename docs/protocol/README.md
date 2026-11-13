# Protocol notes

OpenKDX implements the KDX protocol as documented in the reference spec:

**https://github.com/scottanderson/kdx/tree/main/docs/protocol**
(MIT OR Apache-2.0; pinned at commit `db2c1481648a57e51aa16e353824fcae3a1d402e`, 2026-10-18)

That spec is authoritative unless our own captures or analysis contradict it.
This directory holds only what differs or adds to it.

## Evidence tags

Every claim we add carries its evidence, in the reference spec's style:

- `[O capture frames]` — observed in a capture under `captures/`, e.g. `[O first-connect 4, 6]`.
- `[S binary function]` — read in a binary, e.g. `[S server FUN_08085a04]`. Ghidra projects stay on the lab machine; only addresses and notes are committed.
- `[I]` — inferred.

## Deltas from the reference

| Area | Reference says | We observe | Evidence | Status |
|---|---|---|---|---|
| 207/208 chat text, bytes 0x08–0x0b | `ignored` | A `u16` at 0x08 is 1 for a `/me` action; the client strips the `/me ` prefix and sets it. `openkdx_proto::msg::chat::SendChatText::flags`. | `[O chat-basic 24, 25]` (Client 1.600, Server 1.620) | to report upstream |
| Payload padding (framing.md) | "`txid * 0x41c64e6d + 0x3039` as repeated 32-bit memory words" | The word is laid down little-endian, phase-aligned to the payload offset: byte `i` of the padded buffer is `word.to_le_bytes()[i % 4]`. `openkdx_proto::session::padding_bytes`. | `[O first-connect 10, 13; files-basic 16, 28]` | clarification, to report upstream |
| 414 data padding (framing.md) | "a different repeated 32-bit value" | `0xfedc7e9f` (little-endian) for txid `28814ca0`; not the txid LCG, formula unknown. | `[O files-basic 30]` | open |
| 401 File List `kind` | `exttype/bin` for a `.bin` file | Empty for `tiny.txt`; the matching 413 gives `text/plain`, so the server has a MIME table and 401 does not always carry a type. | `[O files-basic 16, 28]` | observation |
| Proof `status` (handshake.md) | `0x10` OK, `2`, `3`, `4` for the 1.x handshake; `1` failed only in 0.9x | Server 1.620 answers a wrong password with status `1` (auth word `nonce[4..8] ^ 0x201f7790`, the rest zero) and closes; the log says `Login failed`. `openkdx_proto::handshake::ProofStatus::Failed`. | `[O disconnect-ban 9]` | to report upstream |
| 703 Tracker Server List for an empty group (tracker.md) | a u32 count, then the entries | Tracker 1.110 sends a 0-byte payload when the group has no servers. | `[O lab]` `lab_tracker_groups_and_servers` | to report upstream |
| 415/417 upload, partial files (files.md) | "The server resumes an earlier partial upload of the same name." | True, with two details: the partial is kept as `<name>.prt` in the folder (401 lists it), and the server writes it in large chunks: a transfer cut after 150000 bytes left 131072 on disk and 416 offered offset 0x20000; one cut after 20000 bytes left a 0-byte `.prt` and offset 0. | `[O upload-resume 61, 88]` | clarification |
| 171 main section byte 0x52 (administration.md) | "Probably the name's length" | Confirmed: when 172 carries a new name without updating it, the server keeps the old length (`OpenKDX Lab` read back as `OpenKDX La`). `openkdx_client::admin::set_server_name` sets it. | `[O lab]` `lab_server_settings_set_name_round_trip` | confirmation |
| 511 Set Account `editor` (accounts.md) | holds the current login | Confirmed, and 510 returns the *last editor* there, so a client that echoes a 510 record back as 511 asks for a rename (`0x1000d` item already exists). | `[O lab]` `lab_accounts_create_show_delete` | clarification |
| 600 Create Newsgroup, 604 Set Newsgroup (news.md) | Reply is 603 Newsgroup Info | Server 1.620 answers both with a success 1 Status; the new or changed group shows in the next 606 list. `openkdx_client::Session::{create_newsgroup, set_newsgroup}` return `Option<NewsgroupInfo>`. | `[O lab legacy-samples 33, 36]` | delta |
| 1100 Create Folder Access length check (folder-access.md) | "Its total length, padded to a multiple of 4, must equal the payload length." | Confirmed: an unpadded record gets `0x1000a` corrupt data. The same holds for 507/511/504. | `[O lab]` `lab_folder_access_list_create_delete` | confirmation |

## Confirmed so far

| Claim | Evidence |
|---|---|
| Client HELLO: `%TXP%KDX`, version 1, sub-version 0x10, login name in clear after the XOR stream | `[O first-connect 4]`, decoded in `captures/first-connect.md` |
| Server HELLO: sub-version 0x20, empty name, nonce shares bytes 2–9 with the client's | `[O first-connect 6]` |
| A 64-byte junk hello is rejected with `The network protocol is unknown. (ID 4/113 @ TRN941)`; an 8-byte one only disconnects | `lab/ci-smoke.sh`, server log |
| Server registers with the tracker by one 84-byte UDP datagram to 10800 on startup | `[O tracker-basic 1]` |
| File transfers open a second TCP connection to 10700 | `[O files-basic]`, server log `[2 labuser] Download "tiny.txt"` |
| The transfer connection logs in as `\0` + u32 user ID and proves the 32-byte `session_key` from the parent's 105 Server Info, not the account key | `[O files-basic 21, 25]`; `openkdx-dissect` keys it that way |
| `session_key` = `account_key[0..16]` ‖ 16 per-session bytes, for admin and guest alike | `[O first-connect 13; guest-login 13]` |
| Proof `auth` for both sides, `account_key`, the guest constant, the XOR stream, the header scramble, Twofish-256 ECB framing: all as the reference says | `crates/openkdx-proto/tests/{handshake,session}.rs`; every capture decodes and re-encodes byte-exact (`crates/openkdx-dissect/tests/captures.rs`) |
| UDP registration layout, LCG cipher, and CRC-32 (poly 0x04c11db7, init 0xdeadbeef, placeholder `ca d3 1e cc`) re-encode byte-exact | `[O tracker-basic 1]` |
| A download sends two 414 data headers: the data fork (reply bit set) and a 0-byte resource fork (reply bit clear, own txid) | `[O files-basic 29, 31]` |
| Client 1.600 sends `rsrc_offset = 0xffffffffffffffff` in 412 | `[O files-basic 27]` |
| The server's 215 Chat List and 401 File List are in creation and `readdir` order; the client sorts | `[O chat-basic 16; files-basic 16]` |
| A client that sends `118 { value: 1 }` after 100 (a registered Client 1.600) logs in normally; Server 1.620 logs no licence line | `lab/differential.sh` greps the log |
| The 417 data-fork header may carry 415's txid with the reply bit, as the server's 414 answers 412; the resource fork's header is a fresh txid of length 0; the server then sends 418 | `[O lab]` every upload in `tests/lab.rs`; capture in `upload-resume` |
| 402, 507, 511, 172, 1100 are answered by a success Status (`code 0`, value = request type) | `[O lab]`; reference status.md |
| 155 Broadcast reaches every session, the sender's included; 151 arrives before the close on a disconnect with a ban and carries flag 2; 150 itself is answered by a success Status | `[O broadcast 50, 52, 53; disconnect-ban 64, 67]` |
| The server's own ban list does not apply to 127.0.0.1 (reference sequences.md): a guest logs in right after a 10 s ban from loopback | `[O disconnect-ban 78–87]` |
| 172 Set Server Settings is answered by 173; a 105 after it carries the new name | `[O server-settings 10, 29]` |
| 1100 → Status 0x44c; a 402 with `access` naming the item makes `Pictures [Pictures]` | `[O folder-access]` |
| 611 from a guest (no Post News) → error Status; the historian still sees the login/logout 183s | `[O server-history 63]` |
| 143 lists a transfer with `transfer_id != 0`, `bytes_total`, and the path, beside the sessions | `[O connection-monitor 70]` |

## Known inexact in our encoders

| Where | What | Status |
|---|---|---|
| `openkdx_proto::msg::gconsole::Draw::encode` | clamps a record body to `MAX_BODY` (0x3fffff) instead of refusing a longer one | deferred to Phase 10 (GConsole); no capture exercises it |
