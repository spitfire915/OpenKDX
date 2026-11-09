# Captures

Packet captures of the original Haxial programs talking to each other in the
lab, with the exact steps that produced each one. These are the fixtures for
the protocol work: every claim in `docs/protocol/` cites a capture and frame,
or a binary and address.

All captures are loopback traffic inside the lab's private network namespace.
Programs: KDX Server 1.620 (Linux), KDX Tracker 1.110 (Linux), KDX Client 1.600
(Windows, under Wine 6.0.3), KDX Client 1.620 beta 5 (Linux). Each scenario is
reproducible with `lab/scenarios/<name>.sh`; the `.md` beside each capture
lists the steps and the server log, and the `.png` is the display at the end.

| Scenario | Shows | Frames |
|---|---|---|
| `first-connect` | Handshake and login as the administrator; idle session | 14 |
| `guest-login` | Handshake and login with empty login and password (guest) | 14 |
| `chat-basic` | Chat list, join Public Chat, a line, a `/me` action, set topic | 15 |
| `pm-basic` | Two clients; user list; a private message each way | 8 |
| `news-basic` | List newsgroups, read General News, post a message, re-read | 12 |
| `files-basic` | List the base folder; download a 3-byte file on a second connection; refresh | 24 |
| `tracker-basic` | Server registers with the tracker over UDP on startup; client browses groups and servers over TCP | 20 |

Not yet captured: an upload (the Windows client's own file chooser shows no
volumes under Wine; the Rust client will capture uploads in Phase 2), DCC and
voice between clients, view display, accounts and administration, and
GConsole.

Decoding: `openkdx-dissect` (Phase 1) reads these files. Until then,
`first-connect.md` shows the two hellos decoded by hand.

## Rules

- A capture is committed together with its `.md` and the scenario script that
  produced it.
- Lab passwords never appear in the notes (`(set)` stands in).
- Captures contain only lab traffic: synthetic names, loopback addresses, and
  the lab admin's derived key material, which protects nothing real.
