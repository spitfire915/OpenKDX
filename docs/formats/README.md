# Haxial on-disk formats

The files the original KDX programs keep on disk. OpenKDX reads and writes
them so that an existing server directory keeps working, in both directions:
`openkdxd` serves it, and `KDXServer.lexe` can be pointed back at it later.

Nothing here is documented by Haxial. Each format is worked out from the
programs' behaviour, from diffs of files written before and after a single
change, and from static analysis. The authoring tools (Appearance Editor
1.240, KDXSoundListEdit 1.000; see `lab/MANIFEST.txt`) produce controlled
samples of the client-side formats.

| File | Side | Holds | Status | Page |
|---|---|---|---|---|
| `Accounts.dat` | server | accounts, classes, folder access, 32-byte derived keys | verified | `accounts-dat.md` |
| `KDXServer.stg` | server | settings: name, description, address, ports, greeting, server icon, speed limit, allow/deny, trackers, history | read/write | `server-stg.md` |
| `KDXTrackerServ.stg` | tracker | tracker settings | read/write | `tracker-stg.md` |
| `News.dat` | server | newsgroups and messages (created empty at first run) | read/write | `news-dat.md` |
| `News Archives/` | server | archived news, text | read/write | `news-archives.md` |
| `History/` | server | tab-separated history logs | read/write | `history.md` |
| `Downloads/` (client) | client | downloaded files, optionally auto-sorted | from docs | — |
| `Catalog.dat` | server | file catalog for Search | read/write | `catalog.md` |
| `Bases/` conventions | server | `[UL]`, `[DB]`, `[<item>]` suffixes; drop-box owner | read | `bases.md` |
| `KDX.stg` | client | identity, icon, address book, settings, window positions, registration | verified (identity, address book) | `client-stg.md` |
| `Appearances/*.hap` | client | appearance (theme) files | unknown — see below | `hap.md` |
| `Sound Lists/*` | client | KDXSoundListEdit output | unknown — see below | `soundlist.md` |

Status moves through `unknown` → `partial` → `read` → `read/write` → `verified`
(round-trips the user's real files byte for byte and the original program
accepts the result).

`inventory.md` lists the real files available for this work.

## `.hap` and sound lists: why they are still unknown

Appearance Editor 1.240 and KDXSoundListEdit 1.000 run under Wine in the
lab, but their Haxial-drawn Save and Open dialogs list no volumes there (the
same dialog in Client 1.600 does not either, `lab/scenarios/files-basic.sh`),
so a new document cannot be saved and no controlled sample can be made from
the editors' defaults. The way out is a file to start from: a `.hap` from
Haxial's appearance pack (`lab/MANIFEST.txt`, manual download, lab-only)
opened by command-line argument, changed one field at a time and saved in
place with ⌃S, which needs no dialog. Sound lists the same way from a
pack with sound lists. Until then both formats wait for Phase 6, which is
their first consumer.
