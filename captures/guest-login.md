# Capture: guest-login

Recorded 2026-11-10 in the lab (`lab/scenarios/guest-login.sh`).
Programs: KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3).
Frames: 14.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'guestuser'.
1. Start capturing loopback.
1. Connect → address '127.0.0.1', login '<empty>', password <empty>; press Connect.
1. Wait 5 s connected (idle), then stop capturing.

## Server log

```
26/11/10 12:04:00 AM [] KDX Server started o
26/11/10 12:04:20 AM [1 unnamed] Incoming connection from 127.0.0.1:50750
26/11/10 12:04:22 AM [1 unnamed] Set Name: <guestuser> (guest)
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/guest-login.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:50750 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:50750 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:50750 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:50750 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:50750 -> 127.0.0.1:10700  100 Set Identity
12  S->C  127.0.0.1:50750 -> 127.0.0.1:10700  101 User List
13  S->C  127.0.0.1:50750 -> 127.0.0.1:10700  105 Server Info
```
