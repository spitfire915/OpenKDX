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

CI runs the same on Linux, macOS, and Windows, plus the lab smoke test on the
self-hosted runner.

## Commits

`type(scope): summary` — types `feat`, `fix`, `docs`, `test`, `build`, `ci`,
`lab`, `captures`, `refactor`. One logical change per commit.

## Evidence

Every protocol or format claim in `docs/` carries an evidence tag (see
`docs/protocol/README.md`). A capture is committed with the scenario script
and notes that produced it.

## Legal rules

- **Never commit Haxial material**: binaries, archives, the challenge-table
  stories, `.hap` files, icons, or artwork. `ci/check-blocklist.py` catches
  the known hashes; the rule covers the rest.
- Ghidra projects and decompiled output stay local. Our own notes, scripts,
  and address landmarks are fine to commit.
- The challenge tables are fetched or imported at runtime
  (`docs/setup/challenge-tables.md`). Tests that need them skip without them.
- Contributions are licensed MIT OR Apache-2.0, like the project.
