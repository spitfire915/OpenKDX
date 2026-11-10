# Captures

Packet captures of the original Haxial programs talking to each other in the
lab, with the exact steps that produced each one. These are the fixtures for
the protocol work: every claim in `docs/protocol/` cites a capture and frame,
or a binary and address.

All captures are loopback traffic inside the lab's private network namespace.
Programs: KDX Server 1.620 (Linux), KDX Tracker 1.110 (Linux), KDX Client 1.600
(Windows, under Wine 6.0.3), KDX Client 1.620 beta 5 (Linux). Each scenario is
reproducible with `lab/scenarios/<name>.sh`; the `.md` beside each capture
lists the steps and the server log. Screenshots stay on the lab machine: they show Haxial's artwork, which this repository does not carry.

| Scenario | Shows | Frames |
|---|---|---|
| `first-connect` | Handshake and login as the administrator; idle session | 14 |
| `guest-login` | Handshake and login with empty login and password (guest) | 14 |
| `chat-basic` | Chat list, join Public Chat, a line, a `/me` action, set topic | 29 |
| `pm-basic` | Two clients; user list; a private message each way | 38 |
| `news-basic` | List newsgroups, read General News, post a message, re-read | 27 |
| `files-basic` | List the base folder; download a 3-byte file on a second connection; refresh | 38 |
| `tracker-basic` | Server registers with the tracker over UDP on startup; client browses groups and servers over TCP | 20 |

Not yet captured: an upload (the Windows client's own file chooser shows no
volumes under Wine; the Rust client will capture uploads in Phase 2), DCC and
voice between clients, view display, accounts and administration, and
GConsole.

Every capture starts before the client connects, so each holds the full
handshake; a transfer connection (`files-basic`) is keyed by the session key
its parent received in 105 Server Info.

Decoding: `cargo run -p openkdx-dissect -- -p openkdx-lab --audit captures/<name>.pcapng`
prints every hello, proof, and message, and `--audit` confirms each payload
re-encodes byte for byte. With the KDX challenge table available (the
`openkdx-table` cache or `OPENKDX_KDX_TABLE`), the proofs' challenge bytes are
checked too. Each `.md` ends with a `## Messages` inventory that
`crates/openkdx-dissect/tests/captures.rs` checks against the bytes; the
scenario framework regenerates it, or run `openkdx-dissect --inventory`.

## Rules

- A capture is committed together with its `.md` and the scenario script that
  produced it.
- The lab administrator account is `admin` with password **`openkdx-lab`**, fixed and published so that anyone can decrypt the committed sessions (the account key is derived from it as the reference spec's `account_key`). It protects nothing: the lab server only ever listens on a sandbox loopback.
- Captures contain only lab traffic: synthetic names and loopback addresses.
