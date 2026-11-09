# Capture: first-connect

Recorded 2026-11-09 in the lab (`lab/scenarios/first-connect.sh`).
Programs: KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3).
Frames: 14. Screenshot at the end: `first-connect.png`.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'labuser'.
1. Start capturing loopback.
1. Connect → address '127.0.0.1', login 'admin', password (set); press Connect.
1. Wait 5 s connected (idle), then stop capturing.

## Server log

```
26/11/09 09:44:43 PM [] KDX Server started o
26/11/09 09:45:03 PM [1 unnamed] Incoming connection from 127.0.0.1:48104
26/11/09 09:45:05 PM [1 unnamed] Set Name: <labuser> (admin)
```

## Decoded handshake

 The first two payloads, XOR stream undone per the reference spec (`handshake.md`):

- client → server, 44 bytes: protocol `%TXP` sub `%KDX` version 1 sub_version 0x10 name_len 5 challenge 0x10f nonce `0ba203a41443bf56c327792e` name `admin`
- server → client, 36 bytes: protocol `%TXP` sub `%KDX` version 1 sub_version 0x20 name_len 0 challenge 0x564 nonce `2a3303a41443bf0b970e028b` name ``
