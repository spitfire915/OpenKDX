# Capture: pm-basic

Recorded 2026-11-10 in the lab (`lab/scenarios/pm-basic.sh`).
Programs: KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3), KDX Client 1.620 beta 5 (Linux).
Frames: 38.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start KDX Client 1.620 beta 5 (Linux); accept the licence; set user name 'lnxuser'.
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'labuser'.
1. Start capturing loopback.
1. Connect → address '127.0.0.1', login '<empty>', password <empty>; press Connect.
1. Connect → address '127.0.0.1', login 'admin', password (set); press Connect.
1. Admin (Windows client) opens User List and expands the server: lnxuser (guest, black bar) and labuser (admin, red bar).
1. Right-click lnxuser → Send Message…; type 'hi from admin'; Send. The guest's Messages WonderLight lights up.
1. Guest (Linux client) opens Messages, selects labuser, types 'hello back' and presses Return.

## Server log

```
26/11/10 02:05:19 PM [] KDX Server started o
26/11/10 02:05:47 PM [1 unnamed] Incoming connection from 127.0.0.1:58420
26/11/10 02:05:47 PM [1 unnamed] Set Name: <lnxuser> (guest)
26/11/10 02:05:58 PM [2 unnamed] Incoming connection from 127.0.0.1:41754
26/11/10 02:06:00 PM [2 unnamed] Set Name: <labuser> (admin)
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/pm-basic.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:58420 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:58420 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:58420 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:58420 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:58420 -> 127.0.0.1:10700  100 Set Identity
12  S->C  127.0.0.1:58420 -> 127.0.0.1:10700  101 User List
13  S->C  127.0.0.1:58420 -> 127.0.0.1:10700  105 Server Info
18  C->S  127.0.0.1:41754 -> 127.0.0.1:10700  hello
20  S->C  127.0.0.1:41754 -> 127.0.0.1:10700  hello
22  C->S  127.0.0.1:41754 -> 127.0.0.1:10700  proof
23  S->C  127.0.0.1:41754 -> 127.0.0.1:10700  proof
24  C->S  127.0.0.1:41754 -> 127.0.0.1:10700  100 Set Identity
26  S->C  127.0.0.1:41754 -> 127.0.0.1:10700  101 User List
27  S->C  127.0.0.1:41754 -> 127.0.0.1:10700  105 Server Info
29  S->C  127.0.0.1:58420 -> 127.0.0.1:10700  103 User Joined
31  C->S  127.0.0.1:41754 -> 127.0.0.1:10700  300 Send Private Message
32  S->C  127.0.0.1:58420 -> 127.0.0.1:10700  301 Incoming Private Message
35  C->S  127.0.0.1:58420 -> 127.0.0.1:10700  300 Send Private Message
36  S->C  127.0.0.1:41754 -> 127.0.0.1:10700  301 Incoming Private Message
```
