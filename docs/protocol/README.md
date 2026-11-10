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
