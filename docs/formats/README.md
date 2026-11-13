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
| `Accounts.dat` | server, tracker | accounts, classes, privilege bitmaps, 32-byte derived keys | unknown | `accounts-dat.md` |
| `KDXServer.stg` | server | settings: name, description, address, ports, greeting, server icon, speed limit, allow/deny, trackers, history | read/write | `server-stg.md` |
| `KDXTrackerServ.stg` | tracker | tracker settings | unknown | `tracker-stg.md` |
| `News.dat` | server | newsgroups and messages (created empty at first run) | unknown | `news-dat.md` |
| `News Archives/` | server | archived news, text | unknown | `news-archives.md` |
| `History/` | server | tab-separated history logs | unknown | `history.md` |
| `Downloads/` (client) | client | downloaded files, optionally auto-sorted | from docs | — |
| catalog | server | file catalog for Search | unknown | `catalog.md` |
| `Bases/` conventions | server | `[UL]`, `[DB]`, `[<item>]` suffixes; drop-box owner | read | `bases.md` |
| `KDX.stg` | client | identity, icon, address book, settings, window positions, registration | unknown | `client-stg.md` |
| `Appearances/*.hap` | client | appearance (theme) files | unknown | `hap.md` |
| `Sound Lists/*` | client | KDXSoundListEdit output | unknown | `soundlist.md` |

Status moves through `unknown` → `partial` → `read` → `read/write` → `verified`
(round-trips the user's real files byte for byte and the original program
accepts the result).

`inventory.md` lists the real files available for this work.
