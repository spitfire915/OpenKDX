# `Catalog.dat`

**Status:** read/write. Byte-exact over three lab files; our encoder rebuilds
the server's file from its entries, index included.

The file catalog that 452 File Search reads, written next to the server by
450 Generate Catalog (the client's "Generate File Catalog"). Clear text,
magic `%FlK`. Evidence `[O lab catalog 50–52]`: `fixtures/legacy/
lab-2026-11-13/server/catalog/`, generated on a fresh server, after one
upload, and after a second upload and a new folder.

## Layout

| Off | Size | Field |
|---|---|---|
| 0x00 | 4 | `%FlK` |
| 0x04 | 2 | version, 1 |
| 0x06 | 2 | 1 |
| 0x08 | 4 | entry count |
| 0x0c | 4 | offset of the index |
| 0x10 | 8 | generation time, `Time1900` |
| 0x18 | 24 | zero |
| 0x30 | … | entries |
| index | 4 × count | one search mask per entry |
| | 4 × count | the entries' offsets |

### An entry

Every folder and file under `Bases/`, depth first, children sorted by name;
entry 0 is `Bases` itself.

| Off | Size | Field |
|---|---|---|
| 0x00 | 8 | size in bytes; 0 for a folder |
| 0x08 | 8 | created, `Time1900` |
| 0x10 | 8 | modified |
| 0x18 | 4 | 1 file, 2 folder |
| 0x1c | 4 | folders: number of items below at any depth; files: 0 `[O 51, 52]` |
| 0x20 | 4 | index of the parent entry (0 for `Bases`) |
| 0x24 | 4 | 0 |
| 0x28 | 1 | `0x03` folder, `0x10` file |
| 0x29 | 1 | 0 |
| 0x2a | 1 | name length |
| 0x2b | 1 | 0 |
| 0x2c | … | name, then zero `pad4` |

### The search mask

One `u32` per entry, in entry order: bit *n* is set when the name holds the
*n*-th letter of the alphabet, case-insensitively (`Bases` → bits 0, 1, 4,
18 = `0x00040013`); a space sets bit 27, `.` bit 28, `[` and `]` bit 31
`[O 50–52]`, every fixture name). Digits and other punctuation have not
been seen; `name_mask` gives them bit 31 `[I]`. Search presumably tests the
query's mask against each entry's before comparing names.

`openkdx_legacy::catalog::{Catalog, CatalogEntry, name_mask, decode_catalog,
encode_catalog}`, `LegacyDir::{load_catalog, save_catalog}` (no `.orig`: the
server regenerates it on request).
