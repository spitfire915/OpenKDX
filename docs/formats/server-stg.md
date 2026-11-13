# `KDXServer.stg`

**Status:** verified. Byte-exact over 18 lab files; Server 1.620 starts on
our rewrite and shows the name we set (`lab/legacy.sh`).

The server's settings. The file is the same `%KDXSSTG` blob the wire carries
in 171 Server Settings Info and 172 Set Server Settings (reference
`messages/administration.md`), so `openkdx_proto::msg::admin::ServerSettings`
is the one model for both; `openkdx_legacy::server_stg` adds the file checks
and the live-store methods. The server rewrites the whole file on every 172
it accepts `[O lab legacy-samples]`.

Evidence tags as in `docs/protocol/README.md`. `[O lab legacy-samples NN]`
is a diff between consecutive snapshots of `fixtures/legacy/lab-2026-11-13/
server/stg/`, each after one change in the Windows client's Server Settings
window (`lab/scenarios/legacy-samples.sh` lists the steps).

## Layout

| Off | Size | Field | Notes |
|---|---|---|---|
| 0x00 | 8 | magic | `%KDXSSTG` |
| 0x08 | 2 | version | 1 |
| 0x0a | 2 | ignored | 0 |
| 0x0c | 88 | table | 11 × (`u32` offset from the file start, `u32` length) |
| 0x64 | 11 | value_64 | `01 01 01 01 00 01 00 02 00 00 00` in every file |
| 0x6f | 1 | tracker count | the number of entries in section 1 `[O 12, 15]` |
| 0x70 | … | sections | in index order, each at a 4-aligned offset, zero padded |

Table entries for an empty section: sections 1, 2, 3, 5 and 7 carry the
running offset with length 0; 4, 6, 8, 9 and 10 carry `(0, 0)` and have never
been seen non-empty `[O 00–16]`. (A client's 172 differs: 7 is `(0, 0)` and 5
is `(offset, 0)` when the icon is left out — reference.) The encoder keeps a
`placeholders` flag per section so both forms re-encode exactly.

| Section | Holds | Page in the client |
|---|---|---|
| 0 | main (below) | General Settings, Port Numbers, Server History, flags |
| 1 | trackers | Trackers |
| 2 | allow/deny list | Allow/Deny Addresses |
| 3 | greeting: 4 bytes then the text (107 layout) | Greeting |
| 5 | the server icon, as 106 carries it | General Settings |
| 7 | chat rooms (`Public Chat`, `Voice Chat`, `Admin Lounge` on a fresh server; 0x104 bytes) | — (kept as bytes until Phase 4) |

## Section 0: main

0x5c bytes of header, then the strings in order, then zero padding to 4.

| Off | Size | Field | Evidence |
|---|---|---|---|
| 0x00 | 18 | ignored | zero |
| 0x12 | 2 | file transfer port, 0 = none | `[O 14]` |
| 0x14 | 2 | ignored | zero |
| 0x16 | 2 | alternate port, 0 = none | `[O 05]` |
| 0x18 | 4 | history kinds, a bitmask in the window's order: bit 0 Login, 1 Disconnected, then Download File, Upload File, Delete File, Get File List, Miscellaneous File, Modify Account, Miscellaneous Account, Server Settings, Miscellaneous | `[O 07, 08]`, bits 2–10 `[I]` |
| 0x1c | 4 | history: start a new file when size exceeds, bytes (1 MiB default) | `[O 09]` |
| 0x20 | 3 | ignored | zero |
| 0x23 | 1 | flags: bit 0 register with trackers, bit 1 greeting on | `[O 13]`, `[O 06]` |
| 0x24 | 2 | ignored | zero |
| 0x26 | 2 | port | 10700 |
| 0x28 | 4 | ignored | zero |
| 0x2c | 12 | twelve bytes that differ per server directory; an identifier? | `[I]` |
| 0x38 | 4 | total outgoing speed limit, bytes/s; 0 = off | `[O 04]` |
| 0x3c | 22 | ignored | zero |
| 0x52 | 1 | name length | `[O 01]` |
| 0x53 | 1 | description length | `[O 02]` |
| 0x54 | 1 | unpinned | 0 in every file; sits among the length bytes, so the decoder refuses a non-zero value rather than misread the strings |
| 0x55 | 1 | address length | `[O 03]` |
| 0x56 | 6 | ignored | zero |
| 0x5c | … | name, description, address | then pad to 4 |

The reference calls the name NUL-terminated; the zero after it is the
section's `pad4`, which is absent when the strings end on a multiple of 4
(`03-address`: 0x5c + 34 = 0x7e, two zero bytes; a 63-byte name alone gives
one) `[O 02, 03]`.

## Section 1: a tracker entry

0x20-byte header, strings, pad to 4 `[O 12, 15]`.

| Off | Size | Field |
|---|---|---|
| 0x00 | 20 | ignored (zero) |
| 0x14 | 1 | name length |
| 0x15 | 1 | comments length |
| 0x16 | 1 | address length |
| 0x17 | 1 | login length |
| 0x18 | 2 | unpinned (zero; refused when set) |
| 0x1a | 1 | groups length |
| 0x1b | 1 | unpinned (zero; refused when set) |
| 0x1c | 4 | ignored (zero) |
| 0x20 | … | name, comments, address, login, groups |

The password typed into Edit Tracker (five characters, visible as dots in
`captures/legacy-samples-tracker-dialog.png` on the lab) does not appear in
the file written afterwards, and no length byte changes: the server does
not keep it in `KDXServer.stg` `[O 15]`.

## Section 2: an allow/deny entry

0x1c-byte header, strings, pad to 4 `[O 10, 11, 16]`.

| Off | Size | Field |
|---|---|---|
| 0x00 | 8 | expiry, `Time1900` (ms since 1900); 0 = never |
| 0x08 | 8 | ignored (zero) |
| 0x10 | 2 | 1 allow, 0 deny |
| 0x12 | 6 | ignored (zero) |
| 0x18 | 1 | address length |
| 0x19 | 1 | name length |
| 0x1a | 1 | comments length |
| 0x1b | 1 | unpinned (zero; refused when set) |
| 0x1c | … | address, name, comments |

An entry whose expiry has passed is dropped the next time the server writes
the file: the allow entry of `11-allow-expiry` (expiry = the minute it was
made) is gone from `12-tracker` `[O 11, 12]`.

Strings are capped at 255 bytes by their length byte; `ServerSettings::
validate` rejects longer ones before a save or a 172, and the encoder never
writes more bytes than the length byte says. Bytes after the last section
are kept in `trailing` and written back.

## Not yet seen

Sections 4, 6, 8–10; the icon in a file (only on the wire so far, reference
`captures/linux-server-settings.pcapng`); a description over 255 bytes.
