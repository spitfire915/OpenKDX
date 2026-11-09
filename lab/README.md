# Lab

The lab runs the original Haxial KDX programs in sandboxes on a Linux machine
and records their traffic. It is the evidence behind every protocol claim and
the other half of every differential test.

## Machine

Ubuntu 22.04, reachable only from the maintainer's workstation. One-time root
setup is `lab/setup.sh`; everything after that runs as the unprivileged user
`openkdx`, who is also the self-hosted GitHub Actions runner labelled `lab`.

Layout under `/home/openkdx`:

| Path | Holds | In git? |
|---|---|---|
| `openkdx-src/` | a clone of this repository | — |
| `haxial/bin/` | the Haxial archives from `lab/MANIFEST.txt`, each unpacked beside it | **never** |
| `haxial/tables/` | `kdx.bin`, `hgc.bin` (challenge tables) | **never** |
| `haxial/legacy/` | the maintainer's old server and client directories, `.hap` files, sound lists | **never** |
| `haxial/lab.env` | the lab admin login and password | **never** |
| `lab/run/<component>/` | working directories and logs of running sandboxes | no |
| `captures/` | `*.pcapng` and `*.md` scenario notes; copied into `captures/` in the repo by hand | yes, after review |
| `actions-runner/` | the GitHub Actions runner service | — |

## Scripts

| Script | Runs as | Does |
|---|---|---|
| `lab/setup.sh` | root, once | installs i386 libs, Wine, bubblewrap, dumpcap, tools; groups; directories; rustup for the lab user |
| `lab/fetch-binaries.sh` | openkdx | downloads and verifies every archive in `MANIFEST.txt`, unpacks it, extracts the challenge tables |
| `lab/kdx-lab` | openkdx | starts, stops, and captures the original programs in a private network namespace |
| `lab/ci-smoke.sh` | the runner | the `lab` CI job: proves the sandbox and the original server work |
| `lab/scenarios/*.sh` | openkdx | the scripted capture scenarios |

## Admin login

The lab server's administrator account is `admin`. Its password is in
`~openkdx/haxial/lab.env` (`KDX_ADMIN_PASSWORD=…`), created by `kdx-lab` on
the server's first run. It protects nothing real — the server only ever
listens on the sandbox's loopback — but it stays out of the repository all
the same.

## Rules

- Nothing from `haxial/` is ever copied into the repository. CI's blocklist
  (`ci/check-blocklist.py`) catches the known hashes; the rule covers the rest.
- Ghidra projects and decompiled output stay on the lab machine. Only our own
  notes and address landmarks are committed.
- A capture is committed together with its `.md` describing the exact steps
  that produced it, so it can be reproduced.
