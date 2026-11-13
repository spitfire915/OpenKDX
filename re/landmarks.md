# Landmarks

Function addresses in the Haxial binaries with what they do, as found with
`re/scripts/`. Only addresses and our descriptions here; see `re/README.md`.

| Binary | Function | What | Used in |
|---|---|---|---|
| server (KDXServer.lexe 1.620 Linux) | `FUN_08060268` | compares a settings blob's `%KDX` and `SSTG` words (reference spec) | `docs/formats/server-stg.md` |
| server | string table at `0x080ade3f` | `Accounts.dat`, then `[Default]`, `Administrator`, `User`, `Guest`, `Custom Class 1`… as length-prefixed, NUL-terminated entries | `docs/formats/accounts-dat.md` |
| server | `0x080ae8c1` | `Catalog.dat` in the same table | `docs/formats/catalog.md` |
| server | `0x080ae901`, `0x080aeb1b` | `News.dat`, `History` | — |
