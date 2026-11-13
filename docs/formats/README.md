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
| `Accounts.dat` | server | accounts, classes, folder access, 32-byte derived keys | verified (lab) | `accounts-dat.md` |
| `KDXServer.stg` | server | settings: name, description, address, ports, greeting, server icon, speed limit, allow/deny, trackers, history | verified (lab) | `server-stg.md` |
| `KDXTrackerServ.stg` | tracker | tracker settings | read/write | `tracker-stg.md` |
| `News.dat` | server | newsgroups and messages (created empty at first run) | read/write | `news-dat.md` |
| `News Archives/` | server | archived news, text | read/write | `news-archives.md` |
| `History/` | server | tab-separated history logs | read/write | `history.md` |
| `Downloads/` (client) | client | downloaded files, optionally auto-sorted | from docs | — |
| `Catalog.dat` | server | file catalog for Search | read/write | `catalog.md` |
| `Bases/` conventions | server | `[UL]`, `[DB]`, `[<item>]` suffixes; drop-box owner | read | `bases.md` |
| `KDX.stg` | client | identity, icon, address book, settings, window positions, registration | verified (lab; identity, address book) | `client-stg.md` |
| `Appearances/*.hap` | client | appearance (theme) files | read/write (pack, lab-only) | `hap.md` |
| `Sound Lists/*` | client | KDXSoundListEdit output | unknown — see below | `soundlist.md` |

Status moves through `unknown` → `partial` → `read` → `read/write` →
`verified (lab)` (byte-exact over the lab's files and the original program
accepts what we write) → `verified` (the same over the maintainer's real
early-2000s directories, `inventory.md`, still to be added).

`inventory.md` lists the real files available for this work.

## Sound lists: why they are still unknown

KDXSoundListEdit 1.000 runs under Wine in the lab, but its Haxial-drawn Save
dialog lists no volumes there (the same dialog in Client 1.600 and Appearance
Editor does not either), so no sample can be made from it, and no sound-list
pack has turned up. `.hap` was pinned from Haxial's appearance pack instead
(`hap.md`); sound lists wait for a source file or for Phase 6.
