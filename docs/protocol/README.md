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
| _(none yet)_ | | | | |

## Confirmed so far

| Claim | Evidence |
|---|---|
| Client HELLO: `%TXP%KDX`, version 1, sub-version 0x10, login name in clear after the XOR stream | `[O first-connect 4]`, decoded in `captures/first-connect.md` |
| Server HELLO: sub-version 0x20, empty name, nonce shares bytes 2–9 with the client's | `[O first-connect 6]` |
| A 64-byte junk hello is rejected with `The network protocol is unknown. (ID 4/113 @ TRN941)`; an 8-byte one only disconnects | `lab/ci-smoke.sh`, server log |
| Server registers with the tracker by one 84-byte UDP datagram to 10800 on startup | `[O tracker-basic 1]` |
| File transfers open a second TCP connection to 10700 | `[O files-basic]`, server log `[2 labuser] Download "tiny.txt"` |
