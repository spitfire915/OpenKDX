# `News.dat`

**Status:** read/write. Byte-exact over seven lab files; a post appended by
`openkdx-legacy` to the file before a post produces the file the server
wrote after it.

Newsgroups and their messages. Clear text, `%KDX` + `NUUZ`. A fresh server
creates it empty (0 bytes) and writes it on the first change; an empty file
decodes to an empty store `[O lab news 30, 31]`.

Evidence: `[O lab news NN]` is a diff between consecutive files in
`fixtures/legacy/lab-2026-11-13/server/news/` (`legacy-samples.sh`, one
`openkdx news …` command each, listed in that directory's README).

## Layout

| Off | Size | Field | Notes |
|---|---|---|---|
| 0x00 | 8 | magic | `%KDXNUUZ` |
| 0x08 | 2 | version | 2 |
| 0x0a | 2 | group count | `[O 33]` |
| 0x0c | 4 | ignored | 0 |
| 0x10 | 40 | table | 5 × (`u32` offset, `u32` length); only entry 1 is used: `(0x40, groups section length)` |
| 0x38 | 8 | ignored | `00 00 00 00 01 00 00 00` |
| 0x40 | … | group entries | one per group, each padded to 4 |
| | | message lists | one per group, in group order, each at an 8-aligned offset |
| end | 0–7 | padding | to a multiple of 8; whatever the server's buffer held (`9d 40 00 00`, `00 00 00 02`, zeros) |

The bytes between the groups section and the first list, and between lists,
are the same kind of uninitialised padding (`om C` in one file) and are kept
as read `[O 32, 33]`.

### A group entry

| Off | Size | Field |
|---|---|---|
| 0x00 | 4 | offset of this group's message list |
| 0x04 | 4 | length of that list |
| 0x08 | 4 | last message id handed out: 1 in a new group, so the first post gets 2; a deleted id is not reused `[O 32, 34, 35, 37]` |
| 0x0c | 12 | ignored (zero) |
| 0x18 | … | the wire `NewsgroupInfo` (603): group id, time, read/post class bitmaps, message count, size limit in KiB, name, description |
| | 0–3 | pad to 4 |

`ignored_20` of the `NewsgroupInfo` is `01 00 00 00` for `General News` and
zero for a created group; probably the built-in flag `[I]`.

### A message list

`47 fd db 01`, `u32` count, `u32` 0, `u32[count]` cumulative end offsets,
then the wire `NewsMessage` records (608/610: time, group id, message id,
body length, poster, login, subject, body with CR line ends), each padded
to 4 `[O 32, 34]`.

## Behaviour seen

- A deleted message is removed from the list and the count; the id counter
  keeps going `[O 35]`.
- When a group exceeds its size limit the oldest messages are moved to
  `News Archives/<group>/<group> <yymmdd>.txt` (`news-archives.md`) and
  removed from the file; four 1.5 KB posts into a 4 KiB group left one
  `[O 37]`.
- The stock welcome post of a fresh server is Haxial's text; the fixtures
  start after its deletion.

`openkdx_legacy::news::{NewsStore, NewsGroupRecord, decode_news,
encode_news}` and `LegacyDir::{load_news, save_news}`; `NewsStore::{post,
delete_message, create_group, delete_group}` keep the counters as the
server does.
