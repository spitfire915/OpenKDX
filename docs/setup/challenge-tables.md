# Challenge tables

The KDX handshake proves that each side knows a *challenge table*: a blob of
text built into every Haxial program. There are two:

| Table | Size | Held by | Cache file |
|---|---:|---|---|
| The KDX story ("Sale of the Cesspool") | 1739 bytes | every client, server, and tracker | `kdx.bin` |
| The GConsole story ("Who Wants To Be A Maggot?") | 3193 bytes | clients only | `hgc.bin` |

Both are Haxial's copyrighted prose, so OpenKDX does not ship them. Each
program obtains them on your machine and keeps them in a per-user cache
(`openkdx-table path` prints where). Every read verifies the SHA-256, so a
corrupt or tampered file is treated as missing.

## Getting the tables

Three ways, in the order the tools try them:

1. **Environment variables.** `OPENKDX_KDX_TABLE` and `OPENKDX_HGC_TABLE`,
   each the table base64-encoded. The self-hosted lab job sets
   `OPENKDX_KDX_TABLE` from its cache (imported once from the server binary
   that lives there) before it runs the protocol tests; hosted runners have
   no table, and those tests skip.
2. **Fetch.** `openkdx-table fetch` downloads the smallest Haxial archive that
   holds each table — Tracker 1.110 for Windows (248 KB) and Client 1.600 for
   Windows (793 KB) — from preterhuman.net, kdx.technowiki.info, or the
   Wayback Machine, checks the archive's SHA-256 against the known value,
   extracts the story, and caches it. A mirror whose bytes do not match is
   skipped; nothing is cached unless the archive hash is right.
3. **Import.** `openkdx-table import FILE` scans a KDX program, a zip of one,
   or a saved table, and caches every story it finds.

```sh
openkdx-table fetch              # both tables
openkdx-table fetch --force hgc  # re-download one
openkdx-table import ~/Downloads/KDXClient1600-Mac.zip
openkdx-table path               # cache dir and what it holds
openkdx-table verify             # exit 0 if both are present and intact
```

Exit codes: `0` ok; `1` network or I/O error; `2` nothing found, or a hash
mismatch.

## For tests

Tests that need a table read it through `openkdx_table::from_env`, then the
default cache, and **skip** (printing `skipped: no … table available`) when
neither has it. Run `cargo run -p openkdx-table -- fetch` once to make them
real. The dissector works the same way: with a table it checks the challenge
bytes of every proof in a capture; without one it checks only the key proofs
and says so.

Crates that must not carry an HTTP client depend on `openkdx-table` with
`default-features = false`, which leaves out `fetch` (the `reqwest`
dependency) and the `openkdx-table` binary; import and the cache still work.
