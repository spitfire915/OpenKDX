# Contributing

## Checks

Before pushing:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo metadata --locked --format-version 1 > /dev/null
python3 ci/check-deps.py
python3 ci/check-blocklist.py
for t in ci/*_test.py; do python3 -m unittest "$t"; done
```

CI runs the same on Linux, macOS, and Windows, checks the MSRV (Rust 1.96)
and that `Cargo.lock` is current (`cargo metadata --locked`), and on the
self-hosted lab runner runs the smoke test, then `lab/differential.sh` and
`lab/legacy.sh` with the pinned kdx checkout: the fork's client library
against KDX Server 1.620 and Tracker 1.110 (`crates/kdx-client/tests/lab.rs`
there), and the fork's `kdx-legacy` against the files the originals write.
The fork's own lab job checks out this repository and runs the same two
scripts, so a change to either repository is tested against both.

## Working in the kdx fork

The protocol, codec, client, dissector, tracker and legacy-format code is the
`kdx-*` crates of the fork https://github.com/spitfire915/kdx (spec §6.1):
the reference implementation, developed there and upstreamed to
scottanderson/kdx by pull request. OpenKDX keeps `openkdx-table`,
`openkdx-cli`, `openkdx-ui`, `openkdx-app`, `openkdxd`, the lab, the
captures and the static-analysis notes, and depends on `kdx-proto`,
`kdx-codec`, `kdx-client` and `kdx-legacy` by a pinned git revision in
`Cargo.toml` (`lab/kdx-rev.sh` prints it; `sync` checks the lab's copy out
at it).

**Branches in the fork.** `main` is the integration branch: upstream's
`main` merged in with `git merge --no-ff upstream/main` (never rebased or
force-pushed, so pinned revisions stay reachable) plus our merged work.
A change starts on `feat/<topic>`, goes into fork `main` by a pull request
(hosted `check` and `docs` jobs plus the lab job), squash-merged; then
`git checkout -b up/<topic> upstream/main && git cherry-pick <squash commit>`
and a pull request against scottanderson/kdx, with a prose body and no
`Co-Authored-By` line in the description. Upstream pull requests stack when
they depend on each other; nothing here waits on them, because OpenKDX pins
the fork.

**Conventions there are upstream's** (the fork's `AGENTS.md` and
`CONTRIBUTING.md`): an imperative subject with no area prefix, the reasons
in the body, the doc update in the same commit as the code, prose wrapped at
80 columns, evidence as `<!-- [O ...] -->` / `<!-- [S ...] -->` comments,
`python3 -I ci/check-docs.py` and `codespell` clean, `cargo clippy
--workspace --all-targets` with no warnings (use the 1.96 toolchain: `cargo
+1.96 clippy ...`), `cargo fmt`, and the same `Co-Authored-By: Claude <model>
<noreply@anthropic.com>` trailer as here (naming the model that wrote the
commit, e.g. Claude Fable 5.1; upstream lints this with gitlint, UC1). Before every push to the fork run
`python3 ci/check-blocklist.py --root ../kdx`: nothing of Haxial's goes
there either.

**Local work on both.** Clone the fork beside this repository as `../kdx`
and create the untracked `.cargo/config.toml`:

```toml
[patch."https://github.com/spitfire915/kdx"]
kdx-proto = { path = "../kdx/crates/kdx-proto" }
kdx-codec = { path = "../kdx/crates/kdx-codec" }
kdx-client = { path = "../kdx/crates/kdx-client" }
kdx-legacy = { path = "../kdx/crates/kdx-legacy" }
```

Cargo then rewrites `Cargo.lock` to the path sources. **Never commit that
lockfile**: `git diff Cargo.lock` must be empty before any commit except a
pin bump (`build: pin kdx-* at <sha>`), which is made with the patch file
removed. CI refuses a patched lock (`cargo metadata --locked`,
`ci/check-deps.py`). Fork branches follow the fork's `AGENTS.md`: an
imperative subject with no area prefix, reasons in the body, the same
`Co-Authored-By` trailer as here; run `python3 ci/check-blocklist.py --root
../kdx` before pushing there.

**Pin bumps.** After a fork pull request merges, `build: pin kdx-* at <sha>`
updates the four `rev =` values and `Cargo.lock` together, made with the
patch file removed; nothing else changes those lockfile entries. The lab job
syncs `~/kdx-src` to the pin before it runs, so a feature that needs both
repositories lands as: fork PR merged, then the OpenKDX change with its pin
bump.

**What goes where.** A protocol fact, a decoder, a client request, a format
reader: the fork, with the capture or fixture that pins it (our captures are
`captures/openkdx-<name>.pcapng` there, the legacy fixtures
`crates/kdx-legacy/tests/fixtures/`). A scenario, a lab script, a capture's
notes, the CLI, the GUI, the server program: here. A fact our captures add
that upstream has not merged yet is also listed in `docs/protocol/README.md`
with its pull request, so a reader of this repository finds it.

## Client code

- `openkdx-cli` stays thin: argument parsing in `src/args.rs`, output in
  `src/out.rs` (text and `--json`), one `src/cmd/*.rs` per area, and nothing
  `kdx-client` could do instead (a missing request is a fork change). Its
  binary tests (`tests/cli.rs`) drive the built `openkdx` against
  `kdx_client::mock`, which runs the real handshake and framing and needs no
  table; end each test with `server.finish().await`. New subcommands are
  reachable from `openkdx run`, which keeps one `Client` across lines and
  which the lab scenarios use.
- The differential tests run serially and open at most two connections each:
  the server bans an address after 30 connections in 300 s. Keep a run under
  25 connections, or restart the server.
- A new or changed capture is committed with its scenario script and notes;
  the notes end with the `## Messages` inventory that `kdx-dissect
  --inventory` prints from the kdx checkout (the scenario framework appends
  it), and the same bytes go to the fork as `captures/openkdx-<name>.pcapng`
  with a fact test for what they pin.

## Commits

`type(scope): summary` — types `feat`, `fix`, `docs`, `test`, `build`, `ci`,
`lab`, `captures`, `refactor`. One logical change per commit.

Every commit carries the trailer `Co-Authored-By: Claude <model> <noreply@anthropic.com>`,
naming the model that wrote the commit, e.g. Claude Fable 5.1 (upstream lints this
(gitlint, UC1)): the code is written with Claude (Anthropic) as a co-author, and the squash-merge
message of a pull request carries it too. Commits before 2026-11-19 lack the
trailer; `NOTICE.md` states the co-authorship for them, and history is not
rewritten.

## Format code

The originals' files are read and written by the fork's `kdx-legacy`; the
rule there is byte-exact round trips over every committed sample
(`crates/kdx-legacy/tests/fixtures.rs`). New samples come from the lab
scenarios here (`lab/scenarios/legacy-samples.sh`, `client-samples.sh`): one
change per snapshot, listed in the fixture directory's README in the fork,
and never anything of Haxial's (the stock news post, their artwork, their
appearance pack). `lab/legacy.sh` then feeds what the crate writes back to
the original programs. Haxial's appearance pack is lab-only input for the
`.hap` tests (`KDX_LAB_HAP_DIR`); nothing from it, not even one file, is
committed to either repository; every file's hash is in
`ci/haxial-blocklist.txt`.

## Evidence

Every protocol or format claim in `docs/` carries an evidence tag (see
`docs/protocol/README.md`). A capture is committed with the scenario script
and notes that produced it.

## Legal rules

- **Never commit Haxial material**: binaries, archives, the challenge-table
  stories (not even a 20-byte excerpt — compare digests instead), `.hap`
  files, icons, or artwork. `ci/check-blocklist.py` catches the known hashes
  (including every file of the appearance pack); the rule covers the rest.
  Files the originals *wrote from our inputs* in the lab are ours and may be
  committed (the fork's `crates/kdx-legacy/tests/fixtures/`), minus anything
  of theirs inside them, such as the server's stock welcome post.
- Ghidra projects and decompiled output stay local. Our own notes, scripts,
  and address landmarks are fine to commit.
- The challenge tables are fetched or imported at runtime
  (`docs/setup/challenge-tables.md`). Tests that need them skip without them.
- Contributions are licensed MIT OR Apache-2.0, like the project.
