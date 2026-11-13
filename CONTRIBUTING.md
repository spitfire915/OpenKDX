# Contributing

## Checks

Before pushing:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 ci/check-deps.py
python3 ci/check-blocklist.py
for t in ci/*_test.py; do python3 -m unittest "$t"; done
```

CI runs the same on Linux, macOS, and Windows, checks the MSRV (Rust 1.88),
and on the self-hosted lab runner runs the smoke test, the protocol tests
with the real KDX challenge table, and `lab/differential.sh`: the client
library against KDX Server 1.620 and Tracker 1.110
(`crates/openkdx-client/tests/lab.rs`). Locally the table tests skip unless
the table is in the `openkdx-table` cache or `OPENKDX_KDX_TABLE`, and the
differential tests skip unless `OPENKDX_LAB_SERVER` is set.

## Protocol code

- `openkdx-proto` does no I/O and must not depend on an async runtime, a
  network library, or a UI crate; `ci/check-deps.py` enforces it.
- Every payload struct is byte-exact: `ignored_*`, `extra`, and `padding`
  bytes are fields and are written back; bytes a struct does not account for
  fail decoding. `crates/openkdx-proto/tests/round_trip.rs` checks that
  whatever decodes re-encodes to the same bytes, and
  `crates/openkdx-dissect/tests/captures.rs` checks every capture.
- A new or changed capture is committed with its scenario script and notes;
  the notes end with the `## Messages` inventory that `openkdx-dissect
  --inventory` prints (the scenario framework appends it).

## Client code

- `openkdx-client` talks to servers only through `Connection` (requests
  correlated by txid and reply flag, everything else on the event stream) and
  `Transfer` (one download or upload per connection). New server features are
  `impl Session` blocks in a file per area, each with a mock-server test in
  `crates/openkdx-client/tests/` (`tests/common/mock.rs` runs the real
  handshake and framing, needs no table, and records script violations; end
  each test with `server.check()`).
- Records with a trailing `padding` field (accounts, classes, folder access)
  go out padded to a multiple of 4; the server rejects them otherwise.
- Anything the original server does that the reference spec does not say, or
  says differently, goes in `docs/protocol/README.md` with an evidence tag,
  and where it can, a capture.
- The differential tests run serially and open at most two connections each:
  the server bans an address after 30 connections in 300 s. Keep a run under
  25 connections, or restart the server.
- `openkdx-cli` stays thin: argument parsing in `src/args.rs`, output in
  `src/out.rs` (text and `--json`), one `src/cmd/*.rs` per area, and nothing
  the library could do instead. Its binary tests drive the built `openkdx`
  against the mock. New subcommands are reachable from `openkdx run`, which
  the lab scenarios use.

## Commits

`type(scope): summary` — types `feat`, `fix`, `docs`, `test`, `build`, `ci`,
`lab`, `captures`, `refactor`. One logical change per commit.

## Evidence

Every protocol or format claim in `docs/` carries an evidence tag (see
`docs/protocol/README.md`). A capture is committed with the scenario script
and notes that produced it.

## Legal rules

- **Never commit Haxial material**: binaries, archives, the challenge-table
  stories (not even a 20-byte excerpt — compare digests instead), `.hap`
  files, icons, or artwork. `ci/check-blocklist.py` catches the known hashes;
  the rule covers the rest.
- Ghidra projects and decompiled output stay local. Our own notes, scripts,
  and address landmarks are fine to commit.
- The challenge tables are fetched or imported at runtime
  (`docs/setup/challenge-tables.md`). Tests that need them skip without them.
- Contributions are licensed MIT OR Apache-2.0, like the project.
