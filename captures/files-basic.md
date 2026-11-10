# Capture: files-basic

Recorded 2026-11-10 in the lab (`lab/scenarios/files-basic.sh`).
Programs: KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3).
Frames: 24.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Seed the server's base folder with tiny.txt (3 bytes, 'abc').
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'labuser'.
1. Connect → address '127.0.0.1', login 'admin', password (set); press Connect.
1. Start capturing loopback.
1. Open File Browser: the base lists 'Admin Dropbox' [DB], tiny.txt (3), 'Uploads' [UL].
1. Right-click tiny.txt → Download. The client opens a second connection for the transfer.
1. Verified: Downloads/tiny.txt arrived on the client (abc).
1. Click Refresh to list the base again.

## Server log

```
26/11/10 12:07:53 AM [] KDX Server started o
26/11/10 12:08:13 AM [1 unnamed] Incoming connection from 127.0.0.1:38378
26/11/10 12:08:15 AM [1 unnamed] Set Name: <labuser> (admin)
26/11/10 12:08:26 AM [2 unnamed] Incoming connection from 127.0.0.1:46228
26/11/10 12:08:26 AM [2 labuser] Download "tiny.txt".
26/11/10 12:08:26 AM [2 labuser] Transfer Disconnect
```
