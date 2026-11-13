# `KDXTrackerServ.stg`

**Status:** read/write (byte-exact over the one lab file). Sections are kept
as bytes until Phase 5 models the tracker.

The tracker's settings: the same `%KDXTSTG` blob that 711 Tracker Settings
Info and 712 Set Tracker Settings carry (reference `tracker.md`), scrambled
with the tracker's stream (`docs/formats/accounts-dat.md`, "Scrambling":
multiplier `0x41a609fd`, increment `0x1379`, first state `0xacb8fa74`)
`[O lab tracker fresh]`.

`openkdx_proto::msg::tracker::TrackerSettings` is the one model for the wire
and the file; `openkdx_legacy::tracker_stg` unscrambles, checks the magic and
version (1), and adds `LegacyDir::{load_tracker_settings,
save_tracker_settings}`.

## The file of a fresh Tracker 1.110

660 bytes `[O lab tracker fresh]`:

| Section | Offset / length | Holds |
|---|---|---|
| 0 general | 0x78 / 0x5c | the tracker name `New TrackerServ` (length byte at +0x4e, text at +0x54), a 12-byte identifier at +0x24 that differs between two first runs, more |
| 1 upstream | 0xd4 / 0 | present, empty |
| 2 groups | 0xd4 / 0x124 | nine default groups of 0x20 bytes each: a length byte, three zero bytes, the name (`Business`, `Chat`, `Education`, `Games`, `General`, `Macintosh`, `Trackers`, `Windows`, …) |
| 4 greeting | 0x294 / 0 | present, empty; its table offset is the blob's **end**, not the running offset the wire uses, so the model keeps it (`empty_offsets`) |
| 5 admin | 0x1f8 / 0x9c | `00 00 00 01`, the admin's 32-byte derived key, zeros, a length byte and the login |

Sections 3 and 6–11 are absent (`(0, 0)`, present byte 0). The present
bytes are `01 01 01 00 01 01 00 …`: a present byte is set for every section
in the table, empty or not.

## Not yet seen

A changed tracker setting (the lab has no tracker-admin client yet; the
`openkdx` CLI gains one in Phase 5), the file after a restart with groups
or upstream trackers added.
