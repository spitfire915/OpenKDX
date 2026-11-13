# `KDX.stg`

**Status:** verified (lab) for the identity and the address book (byte-exact over
six lab files; Client 1.600 starts on a file we wrote and logs in with the
name we set — `lab/scenarios/client-samples.sh`). Window positions,
settings and registration are kept as bytes until Phase 7.

The client's settings, written on a clean exit (palette → Exit; a killed
client loses changes). Scrambled with the client's stream
(`accounts-dat.md`, "Scrambling": `0x41a609fb`, `0x2319`, `0x35662cb6`).

Evidence `[O lab client 60–64]`: `fixtures/legacy/lab-2026-11-13/client/`,
one change per exit (listed in that directory's README).

## Plain layout

| Off | Size | Field |
|---|---|---|
| 0x00 | 8 | `%KDXSTNG` |
| 0x08 | 2 | version, 2 |
| 0x0a | 2 | 0 |
| 0x0c | 4 | 0 |
| 0x10 | 96 | 12 × (`u32` offset, `u32` length) |
| 0x70 | 16 | `00 00 00 00 03 01 00 00 00 00 00 02 01 01 01 01` |
| 0x80 | … | the sections, contiguous, in the order 0, 1, 7, 8, 10, 9, 11 (not index order; the model keeps `order`) |

| Section | Length (fresh) | Holds |
|---|---|---|
| 0 | 0x214 | window positions and sizes, opaque (Phase 7) |
| 1 | 0x234 + name + description | identity, below |
| 7 | 0xe8 + entries | address book, below |
| 8 | 0x74 | opaque (starts `b7 1a 00 02`) |
| 9 | 0x38 | zero |
| 10 | 0x24 | 36 bytes that look random; registration or a machine id `[I]` |
| 11 | 0x18 | six 4-byte values `FD\0d`, `FD\0f`, …; sound or volume settings `[I]` |

### Section 1: identity

| Off | Size | Field |
|---|---|---|
| 0x00 | 12 | zero |
| 0x0c | 4 | `00 ff cc 00` (the background colour again?) |
| 0x10 | 12 | `00 00 02 0b` then zero |
| 0x1c | 1 | name length `[O 61]` |
| 0x1d | 1 | description length `[O 62]` |
| 0x1e | 2 | zero |
| 0x20 | 2 | `01 02` |
| 0x22 | 3 | `ff ff ff` |
| 0x25 | 3 | background colour (`ff cc 00`, the client's default) `[I]` |
| 0x28 | 3 | text colour (`00 00 00`) `[I]` |
| 0x2b | 512 | the 32×32 icon, 4 bits per pixel, 16 bytes per row |
| 0x22b | … | name, description, zero pad to 4 |

The two colours are inferred from the client's defaults (text black on
`#ffcc00`, as it sends in 100 Set Identity); a colour-change sample is still
to be made.

### Section 7: address book

A list: `38 fd db 01`, `u32` count, `u32` (equal to the count in every
file; kept as read), `u32` 0, `u32[count]` entry ids (1, 2, 3 …), `u32` 0,
`u32[count]` cumulative end offsets, then the records `[O 63, 64]`.

A record, 0x84-byte header then strings, padded to 4:

| Off | Size | Field |
|---|---|---|
| 0x00 | 16 | zero |
| 0x10 | 4 | flags: `01 00 00 00` on the stock entry, `00 00 00 80` on one made in New Address Book Item with its default check boxes; not pinned |
| 0x14 | 4 | `0x16` on the stock entry, 0 otherwise |
| 0x18 | 4 | 0 |
| 0x1c | 32 | the password as `AccountKey::from_password`; the guest key when empty `[O 63]` |
| 0x3c | 64 | zero |
| 0x7c | 5 | lengths: name, description, comments, address, login |
| 0x81 | 3 | zero |
| 0x84 | … | name, description, comments, address, login |

The stock entry (`Official Haxial Server`, `kdxserver.haxial.com`) keeps
its one-line text in *comments* (the Description column is empty in the
window); an entry from the window puts the Description field at 0x7d
`[O 60, 63]`.

`openkdx_legacy::client_stg::{ClientSettings, ClientIdentity,
AddressBookEntry, decode_client_stg, encode_client_stg}`.
