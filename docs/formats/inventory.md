# Legacy data inventory

Real files written by the original programs, available for the format work in
Phase 3. Two sources:

1. **Lab-generated** (`fixtures/legacy/lab-2026-11-09/`, committed): produced
   in the lab on 2026-11-09 by KDX Server 1.620 (Linux) and KDX Client 1.600
   (Windows under Wine), with known inputs. Small, reproducible, and the only
   set whose contents we fully know.
2. **The maintainer's early-2000s server and client directories** (on the lab
   machine under `~openkdx/haxial/legacy/`, never committed): to be added;
   run `lab/inventory.sh` and append its table here when they arrive. These
   give real-world variety: many accounts, custom classes, folder access
   items, news archives, history logs, a server icon, `.hap` files.

## Lab-generated fixtures

| File | Size | SHA-256 (first 16) | First 8 bytes | Notes |
|---|---:|---|---|---|
| `server/Accounts.dat` | 2312 | `20db32cb2b460baa` | `bf996c398e85a1d2` | No plaintext anywhere: the whole file is encrypted or scrambled. Grew from 2008 bytes (admin + guest after first run) to 2312 after two accounts were added from the client, so one account record is about 152 bytes on disk. |
| `server/KDXServer.stg` | 476 | `7dc20c8423d8a73b` | `254b445853535447` | Magic `%KDXSSTG`, then `00 01` (version?), then what looks like a table of 32-bit big-endian offsets/lengths (`0x70`, `0x68`, `0xd8`…). Mostly zero at defaults. Grew to 536 bytes when a tracker entry was added. |
| `server/News.dat` | 2424 | `a4429688fa939fce` | `254b44584e55555a` | Magic `%KDXNUUZ`, `00 02 00 01`. Plaintext: newsgroup name `General News`, the stock welcome post (author `KDX`), and the lab's posts. Created as a 0-byte file at first run; written on first post. |
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

## Observations for Phase 3

- Two file families: **clear with a magic** (`%KDXSSTG`, `%KDXNUUZ`), and
  **scrambled** (`Accounts.dat`, client `KDX.stg`). The scrambled ones are the
  ones that hold secrets (derived keys, address-book passwords), so the
  obfuscation is probably a fixed-key cipher or the handshake's XOR stream
  with a stored key; the first 4 bytes (`bf996c39`, `102d68ee`) may be that
  key, as in the handshake.
- The reference spec says `Accounts.dat` stores the 32-byte derived key per
  account, not the password. The lab's admin password is known, so the
  derived key can be computed (`account_key` in the spec) and searched for
  once the scrambling is undone: a built-in known-plaintext check.
- `KDXServer.stg` and `News.dat` share the `%KDX` + 4-letter tag convention
  and big-endian 32-bit fields, like the wire protocol.
