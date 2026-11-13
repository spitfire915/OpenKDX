# Capture: folder-access

Recorded 2026-11-13 in the lab (`lab/scenarios/folder-access.sh`).
Programs: KDX Server 1.620 (Linux), OpenKDX client (openkdx CLI, 05965e2).
Frames: 34.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start capturing loopback.
1. One admin session runs: folder-access list (1106/1107), show UL (1103/1104), create Pictures (1100 → Status 0x44c), show Pictures, files mkdir '' Pictures --access Pictures (402 → Status 0x192), files ls (400/401 shows 'Pictures [Pictures]'), folder-access delete Pictures (1101 → 1102), list.

## Server log

```
26/11/13 02:54:02 PM [] KDX Server started o
26/11/13 02:54:03 PM [1 unnamed] Incoming connection from 127.0.0.1:52352
26/11/13 02:54:03 PM [1 unnamed] Set Name: <openkdx> (admin)
26/11/13 02:54:03 PM [1 openkdx] Disconnected
```

## Client transcript

```
$ openkdx -l 'admin' --nick openkdx run --keep-going
> folder-access list
name       icon     flags  description
[Default]  0x0      0x1    If a folder and its parents do not specify an access item, this item is used by default.
DB         dropbox  0x2    Dropbox.
UL         uploads  0x0    Uploads folder.
> folder-access show "UL"
name: UL
icon: uploads
flags: 0x0
owner: 
description: Uploads folder.
rule: 0x0
class_rules: ["0:0xfffe","1:0xfde0","2:0xfde0","3:0xfde0","4:0xfde0","5:0xfde0","6:0xfde0","7:0xfde0","8:0xfde0","9:0xfde0","10:0xfde0","11:0xfde0","12:0xfde0","13:0xfde0","14:0xfde0","15:0xfde0","16:0xfde0","17:0xfde0","18:0xfde0","19:0xfde0","20:0xfde0","21:0xfde0","22:0xfde0","23:0xfde0","24:0xfde0","25:0xfde0","26:0xfde0","27:0xfde0","28:0xfde0","29:0xfde0","30:0xfde0","31:0xfde0"]
> folder-access create Pictures --icon pictures --description pics
> folder-access show Pictures
name: Pictures
icon: pictures
flags: 0x0
owner: 
description: pics
rule: 0x0
class_rules: []
> files mkdir "" Pictures --access Pictures
created Pictures
> files ls -L
name                  size  modified             type  access
Uploads [UL]/               2026-11-13 14:53:54        [UL]
Admin Dropbox [DB]/         2026-11-13 14:53:54        [DB]
Pictures [Pictures]/        2026-11-13 14:54:03        [FA]
> folder-access delete Pictures
> folder-access list
name       icon     flags  description
[Default]  0x0      0x1    If a folder and its parents do not specify an access item, this item is used by default.
DB         dropbox  0x2    Dropbox.
UL         uploads  0x0    Uploads folder.
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/folder-access.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  100 Set Identity
11  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  101 User List
12  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  105 Server Info
13  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  118 (unnamed; license report)
15  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  1106 Get Folder Access List
17  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  1107 Folder Access List (reply)
18  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  1103 Get Folder Access
19  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  1104 Folder Access Info (reply)
20  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  1100 Create Folder Access
21  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  1 Status (reply)
22  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  1103 Get Folder Access
23  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  1104 Folder Access Info (reply)
24  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  402 Create Folder
25  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  1 Status (reply)
26  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  400 Get File List
27  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  401 File List (reply)
28  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  1101 Delete Folder Access
29  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  1102 Folder Access Deleted
30  C->S  127.0.0.1:52352 -> 127.0.0.1:10700  1106 Get Folder Access List
31  S->C  127.0.0.1:52352 -> 127.0.0.1:10700  1107 Folder Access List (reply)
```
