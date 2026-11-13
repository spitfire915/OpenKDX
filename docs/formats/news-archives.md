# `News Archives/`

**Status:** read/write (append-only). Byte-exact over the lab file; the
formatter reproduces the server's entry from the message as it was in
`News.dat` before archiving.

When a newsgroup grows past its size limit (`news-dat.md`), the server moves
its oldest messages out of `News.dat` into a text file:

```
News Archives/<group name>/<group name> <yymmdd>.txt
News Archives/Second Group/Second Group 261113.txt
```

Evidence `[O lab news 37]`: `fixtures/legacy/lab-2026-11-13/server/News
Archives/`, after four 1.5 KB posts into a 4 KiB group.

## Layout

UTF-8 BOM, then per archived message, in order:

```
LF
____________________________________________________________   (60 × "_")
LF
<subject>
LF
"<" poster ">" two spaces "Fri, 13 Nov 2026, 06:17:32 PM"
LF
<body, with the CR line ends it had in News.dat>
```

Nothing follows the last body. The date is `Www, d Mmm yyyy, hh:mm:ss AM|PM`
in the server's local time.

`openkdx_legacy::news_archive::{ArchiveEntry, parse_news_archive,
format_archive_entry, format_archive_time, archive_file_name}` and
`LegacyDir::append_news_archive`.
