# `.hap` appearance files

**Status:** read/write. Byte-exact over all 111 files of Haxial's appearance
pack (`Appearances041205.sit`, lab-only: `~/haxial/legacy/appearances/`);
`openkdx_legacy::hap::{Theme, decode_hap, encode_hap}`. Which colour or
image slot the client draws where is Phase 6's work (the editor's lists give
the names, below).

Evidence `[O pack]`: the 111 files, read with `lab_every_pack_file_round_trips`
(`crates/openkdx-legacy/tests/hap.rs`, `OPENKDX_LAB_HAP_DIR`). No file of
the pack is committed: it is Haxial's and third parties' artwork (§5.3).
The constructed-theme tests cover the writer with pixels of ours. Appearance
Editor 1.240 could not be used for one-field diffs: it ignores a file given
on the command line and its Haxial-drawn Save/Open dialog lists no volumes
under Wine, so nothing can be saved from it.

## Layout

| Off | Size | Field |
|---|---|---|
| 0x00 | 4 | `%HAP` |
| 0x04 | 2 | version, 1 |
| 0x06 | 6 | zero |
| 0x0c | 104 | 13 × (`u32` offset, `u32` length) |
| 0x74 | 20 | `00 ×8, 01 01 01 01, 00 ×8` in every file |
| 0x88 | … | sections, contiguous and in index order; every length is a multiple of 4. The decoder accepts only this shape (and an empty section at offset 0 or the running offset), so a successful decode means the file rewrites byte for byte |

| Section | Holds |
|---|---|
| 4 | information: created, modified (`Time1900`), 18 zero bytes, four length bytes (name, version, creator, description), 14 zero bytes, the strings, pad 4 |
| 5 | images: a 528-slot table, then bitmap records with the 12-byte extra header |
| 6 | 204 colours, `00 RR GG BB` each |
| 7 | icons: a 768-slot table, then bitmap records without the extra header |
| others | never seen non-empty; empty 5 and 7 carry the running offset, the others `(0, 0)` (`Theme::default` marks 4–7 as placeholders; 4 and 6 are never empty) |

A slot table is one `u32` per UI element: 0 for none, else the record's
offset from the section start; several slots may share one record, and every record is used by at least one slot (a table of zeros with no records is rejected). Records follow the table contiguously in offset order.

### A bitmap record

| Off | Size | Field |
|---|---|---|
| 0x00 | 2 | width |
| 0x02 | 2 | height |
| 0x04 | 1 | flag: 1 when byte 7 names a transparent palette index `[I]` |
| 0x05 | 1 | depth in bits per pixel: 1, 2, 4 or 8 |
| 0x06 | 1 | palette entries − 1 |
| 0x07 | 1 | the transparent index when the flag is set; otherwise usually 0, kept as read `[I]`. The flag is 0 or 1; anything else is rejected |
| 0x08 | 12 | images only: a colour (`00 RR GG BB`), four bytes that look like insets (`08 08 07 07`, `0b 0b 0a 0a`), a word (0, `04 00 00 00`, …) `[I]` |
| | 4 × n | the palette, `00 RR GG BB` |
| | stride × height | pixels, most significant bits first, rows padded to 4 bytes |

Every images and icons section of the pack tiles exactly with this record
`[O pack]`.

## The editor's names (for Phase 6)

Appearance Editor 1.240's Colors, Images and Icons pages list the slots by
name in order (screenshots `~/captures/hap-col-*.png`, `hap-images-*.png`,
`hap-icons-*.png` on the lab). Colours, as listed, with the group headers
(each header has its own swatch): Primary Group (Light, Background, Dark,
Frame, Label, Disable Frame, Disable Label); Miscellaneous (Important Label,
Focus Box, Text Box Background/Foreground, Text Hilite Background/Foreground,
Text Insertion Point, List Background, List Label, List Hilite
Background/Foreground, List Sort Column Background, List Separator,
Workspace Background 1–4); Button Group, Button Hilite Group, Button Disable
Group (Light 2, Light 1, face, Dark 1, Dark 2, Frame, Label each); Default
Button Group (Light, face, Dark, Frame); Window Frame (Light 2, Light 1,
Window, Dark 1, Dark 2, Frame, Label); Window Frame Transition 1–18; Window
Frame Focus (as Window Frame); Window Frame Focus Transition 1–18; Menu
Group (Light, Background, Dark, Label, Disable Label); Menu Hilite (Light,
Background, Dark, Label); Progress Bar Group (Transition 1–10, Frame, Label);
Progress Background (Light, Bkgnd, Dark); Scroll Bar Group (Frame, Light,
ScrollBar, Dark, Label), Scroll Bar Hilite (Light, Hilite, Dark, Label),
Scroll Bar Indicator (Light, Indicator, Dark), Scroll Bar Indicator Hilite
(Light, Hilite, Dark), Scroll Bar Background (Light 2, Light 1, Bkgnd, Dark 1,
Dark 2), Scroll Bar Disable (Light, Disable, Dark, Frame, Label); Slider
Group (Bar, Bar Frame, Bar Hilite, Bar Hilite Frame), Slider Indicator
(Light, Indicator, Dark, Frame), Slider Indicator Hilite (same), Slider
Disable (Light, Disable, Dark, Frame); Column Header Group (Frame, Light,
Header, Dark, Label, Hilite Light, Hilite, Hilite Dark, Hilite Label); File
Label Colors 0–15. Counting each header row with a swatch as one row gives more rows (about
215) than the 204 entries in the file, so the row-to-slot mapping is not a
plain 1:1 and is left to Phase 6, which can read it from the client's
rendering.

## Not supported (yet)

Nothing is rendered yet (`openkdx-ui`, Phase 6); `Bitmap::rgba` gives RGBA
with the transparent index at alpha 0 (a 65535×65535 bitmap would need 17 GB
there; the renderer should cap what it draws). Fonts: none seen — sections
0–3 and 8–12 have never been non-empty — though §7.3 expected them.
`Bitmap`'s fields are public; `pixel` and `rgba` assume the invariants
`Bitmap::new` and `decode_hap` establish.
