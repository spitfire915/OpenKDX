# Capture: pm-basic

Recorded 2026-11-10 in the lab (`lab/scenarios/pm-basic.sh`).
Programs: KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3), KDX Client 1.620 beta 5 (Linux).
Frames: 8.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start KDX Client 1.620 beta 5 (Linux); accept the licence; set user name 'lnxuser'.
1. Connect → address '127.0.0.1', login '<empty>', password <empty>; press Connect.
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'labuser'.
1. Connect → address '127.0.0.1', login 'admin', password (set); press Connect.
1. Start capturing loopback.
1. Admin (Windows client) opens User List and expands the server: lnxuser (guest, black bar) and labuser (admin, red bar).
1. Right-click lnxuser → Send Message…; type 'hi from admin'; Send. The guest's Messages WonderLight lights up.
1. Guest (Linux client) opens Messages, selects labuser, types 'hello back' and presses Return.

## Server log

```
26/11/10 12:05:40 AM [] KDX Server started o
26/11/10 12:05:52 AM [1 unnamed] Incoming connection from 127.0.0.1:37434
26/11/10 12:05:52 AM [1 unnamed] Set Name: <lnxuser> (guest)
26/11/10 12:06:17 AM [2 unnamed] Incoming connection from 127.0.0.1:39146
26/11/10 12:06:19 AM [2 unnamed] Set Name: <labuser> (admin)
```
