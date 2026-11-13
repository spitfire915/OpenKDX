# openkdx-legacy

Readers and writers for the files the original KDX programs keep on disk,
byte for byte, so that an existing data directory can be served by OpenKDX
and handed back to `KDXServer.lexe` at any time (design spec §5.2). Shared
by the server and the client. No network or UI dependency; only
`openkdx-proto` (whose wire records double as the on-disk records) and
`thiserror`.

| Module | File | Status |
|---|---|---|
| `server_stg` | `KDXServer.stg` | verified (lab) — Server 1.620 starts on our rewrite |
| `accounts` | `Accounts.dat` | verified (lab) — an account we wrote logs in; the server lists a post we appended |
| `news` | `News.dat` | read/write, byte-exact, layout matches the server's |
| `history` | `History/*.txt` | read/write (append-only) |
| `news_archive` | `News Archives/<group>/*.txt` | read/write (append-only) |
| `catalog` | `Catalog.dat` | read/write, byte-exact |
| `tracker_stg` | `KDXTrackerServ.stg` | read/write; sections opaque until Phase 5 |
| `client_stg` | `KDX.stg` | verified (lab) for identity and address book — Client 1.600 starts on our file |
| `scramble` | the LCG XOR stream over the three scrambled files | — |
| `store` | `LegacyDir`: `.orig` backups once, atomic saves | — |
| `bases` | `Bases/` folder-name tags | — |

`.hap` appearance files and sound lists are not here yet; `docs/formats/
README.md` says why and how they will be done. Every format has a page in
`docs/formats/` with its evidence.

## The live store

`LegacyDir::open(dir)` copies each known file to `<name>.orig` once (never
again, even if the live file changes) and `save_*` writes `<name>.tmp` and
renames it over the live file with the original's permissions. Readers keep
every byte they do not understand (`ignored_*`, padding, gaps) so that
`encode(decode(file)) == file` for every file in `fixtures/legacy/`, and the
lab checks (`lab/legacy.sh`) feed what we write back to the originals.

## Samples

Everything under `fixtures/legacy/` was written by the original programs in
the lab from inputs of ours; the README there lists the step behind every
file. The scenarios are `lab/scenarios/legacy-samples.sh` (server files) and
`lab/scenarios/client-samples.sh`. Nothing of Haxial's is committed: the
server's stock welcome post is deleted before the first `News.dat` sample,
and the appearance pack, when used, stays on the lab machine.

`examples/legacytool.rs` dumps a file, unscrambles one, adds an account,
renames a server or a client identity, or rewrites a directory.

Part of [OpenKDX](../../README.md). Licensed under MIT OR Apache-2.0.
