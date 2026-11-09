# Capture: guest-login

Recorded 2026-11-09 in the lab (`lab/scenarios/guest-login.sh`).
Programs: KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3).
Frames: 14. Screenshot at the end: `guest-login.png`.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'guestuser'.
1. Start capturing loopback.
1. Connect → address '127.0.0.1', login '<empty>', password <empty>; press Connect.
1. Wait 5 s connected (idle), then stop capturing.

## Server log

```
26/11/09 09:45:24 PM [] KDX Server started o
26/11/09 09:45:44 PM [1 unnamed] Incoming connection from 127.0.0.1:37968
26/11/09 09:45:46 PM [1 unnamed] Set Name: <guestuser> (guest)
```
