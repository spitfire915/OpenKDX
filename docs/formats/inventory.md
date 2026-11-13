# Legacy data inventory

Real files written by the original programs, the evidence behind
`openkdx-legacy`. Three sources:

1. **Lab-generated** (`fixtures/legacy/lab-2026-11-09/`, committed): produced
   in the lab on 2026-11-09 by KDX Server 1.620 (Linux) and KDX Client 1.600
   (Windows under Wine), with known inputs. Small, reproducible, and the only
   set whose contents we fully know.
2. **Lab-generated, Phase 3** (`fixtures/legacy/lab-2026-11-13/`, committed):
   56 files, one controlled change each; see below.
3. **Haxial's appearance pack** (`~openkdx/haxial/legacy/appearances/` on the
   lab, never committed): 111 `.hap` files, the only real appearances.

Still wanted: **the maintainer's early-2000s server and client directories**
(to go under `~openkdx/haxial/legacy/`, never committed). Run
`lab/inventory.sh` and append its table here when they arrive; they move the
formats from "verified (lab)" to "verified": many accounts, custom classes,
folder access items, news archives, history logs, a server icon, sound lists.

## Lab-generated fixtures, Phase 0 (2026-11-09)

| File | Size | SHA-256 (first 16) | First 8 bytes | Notes |
|---|---:|---|---|---|
| `server/Accounts.dat` | 2312 | `20db32cb2b460baa` | `bf996c398e85a1d2` | Scrambled (an LCG XOR stream, `accounts-dat.md`); admin, guest and two added accounts. Made with an earlier lab admin password, so the known-plaintext tests use the 2026-11-13 set. |
| `server/KDXServer.stg` | 476 | `7dc20c8423d8a73b` | `254b445853535447` | Magic `%KDXSSTG`, then `00 01` (version?), then what looks like a table of 32-bit big-endian offsets/lengths (`0x70`, `0x68`, `0xd8`…). Mostly zero at defaults. Grew to 536 bytes when a tracker entry was added. |
| `server/News.dat` | — | — | `254b44584e55555a` | **Not committed**: it carries Haxial's stock welcome post verbatim (about 2 KB of their prose). Observed in the lab: magic `%KDXNUUZ`, `00 02 00 01`, then plaintext newsgroup name `General News` and the posts. Created as a 0-byte file at first run; written on first post; 2424 bytes with the stock post and two lab posts. Phase 3 regenerates it in the lab after deleting the stock post. |
| `client/KDX.stg` | 1688 | `29322c4c84543306` | `102d68ee759011cc` | No plaintext: scrambled like `Accounts.dat`. Written only when the client exits cleanly (palette → Exit); a SIGINT loses it. Holds identity (`labuser`), the address book (one stock entry, "Official Haxial Server"), settings, and licence acceptance. |

What the lab server directory looks like after first run and some use:

```
KDXServer.lexe  KDXUpgradeHelper.lexe
Accounts.dat    KDXServer.stg    News.dat
Bases/Default/{Admin Dropbox [DB], Uploads [UL]}
```

`History/` and `News Archives/` are not created until their features are
turned on (Server Settings → Server History; newsgroup archive size).

What the lab client directory looks like after a clean exit:

```
KDXClient.exe  KDX.stg  Appearances/  Sound Lists/  Downloads/
```

`Appearances/` and `Sound Lists/` are created empty at first run.

## Lab-generated fixtures, Phase 3 (`fixtures/legacy/lab-2026-11-13/`)

Written by `lab/scenarios/legacy-samples.sh`,
`client-samples.sh` and three one-off probes, one controlled change per
file; the README in that directory lists every step. 17 `KDXServer.stg`,
10 `Accounts.dat`, 7 `News.dat`, one `News Archives/` file, one `History/`
file, 3 `Catalog.dat`, one `KDXTrackerServ.stg`, 5 `KDX.stg`. Every one
round-trips byte for byte (`crates/openkdx-legacy/tests/`).

## Observations for Phase 3 (written before the work; kept for the record)

- Two file families: **clear with a magic** (`%KDXSSTG`, `%KDXNUUZ`), and
  **scrambled** (`Accounts.dat`, client `KDX.stg`). The scrambled ones are the
  ones that hold secrets (derived keys, address-book passwords), so the
  obfuscation is probably a fixed-key cipher or the handshake's XOR stream
  with a stored key; the first 4 bytes (`bf996c39`, `102d68ee`) may be that
  key, as in the handshake. *(Outcome: a fixed-constant LCG XOR stream per
  program, no key in the file — `accounts-dat.md`.)*
- The reference spec says `Accounts.dat` stores the 32-byte derived key per
  account, not the password. The lab's admin password is known, so the
  derived key can be computed (`account_key` in the spec) and searched for
  once the scrambling is undone: a built-in known-plaintext check.
- `KDXServer.stg` and `News.dat` share the `%KDX` + 4-letter tag convention
  and big-endian 32-bit fields, like the wire protocol.
