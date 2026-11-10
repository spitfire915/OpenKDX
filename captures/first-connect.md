# Capture: first-connect

Recorded 2026-11-10 in the lab (`lab/scenarios/first-connect.sh`).
Programs: KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3).
Frames: 14.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'labuser'.
1. Start capturing loopback.
1. Connect → address '127.0.0.1', login 'admin', password (set); press Connect.
1. Wait 5 s connected (idle), then stop capturing.

## Server log

```
26/11/10 12:03:17 AM [] KDX Server started o
26/11/10 12:03:37 AM [1 unnamed] Incoming connection from 127.0.0.1:58652
26/11/10 12:03:39 AM [1 unnamed] Set Name: <labuser> (admin)
```

## Decoded handshake

 The first two payloads, XOR stream undone per the reference spec (`handshake.md`):

- client → server, 44 bytes: protocol `%TXP` sub `%KDX` version 1 sub_version 0x10 name_len 5 challenge 0x40d nonce `49f403a414c2980df0150354` name `admin`
- server → client, 36 bytes: protocol `%TXP` sub `%KDX` version 1 sub_version 0x20 name_len 0 challenge 0x373 nonce `d51503a414c2987f0d832176` name ``

The lab admin password is `openkdx-lab` (see `captures/README.md`), so the session after the handshake can be decrypted with the reference spec's `account_key`.
