# Capture: server-settings

Recorded 2026-11-13 in the lab (`lab/scenarios/server-settings.sh`).
Programs: KDX Server 1.620 (Linux), OpenKDX client (openkdx CLI, 05965e2).
Frames: 64.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start capturing loopback.
1. Admin: server settings (170/171), server set-name 'OpenKDX Lab' (170/171 then 172 with section 0's name and length byte changed → Status), server settings again.
1. A second login: 105 Server Info now names the server 'OpenKDX Lab'.
1. Restore the name.

## Server log

```
26/11/13 02:54:16 PM [] KDX Server started o
26/11/13 02:54:17 PM [1 unnamed] Incoming connection from 127.0.0.1:43622
26/11/13 02:54:17 PM [1 unnamed] Set Name: <openkdx> (admin)
26/11/13 02:54:17 PM [1 openkdx] Disconnected
26/11/13 02:54:17 PM [2 unnamed] Incoming connection from 127.0.0.1:43628
26/11/13 02:54:17 PM [2 unnamed] Set Name: <openkdx> (admin)
26/11/13 02:54:17 PM [2 openkdx] Disconnected
26/11/13 02:54:17 PM [3 unnamed] Incoming connection from 127.0.0.1:43636
26/11/13 02:54:17 PM [3 unnamed] Set Name: <openkdx> (admin)
26/11/13 02:54:17 PM [3 openkdx] Disconnected
```

## Client transcript

```
$ openkdx -l 'admin' --nick openkdx run --keep-going
> server settings
version: 1
name: New Server
greeting_enabled: false
total_outgoing_speed_limit: 0
section_lengths: [104,0,0,0,0,0,0,0,0,0,0]
> server set-name "OpenKDX Lab"
> server settings
version: 1
name: OpenKDX Lab
greeting_enabled: false
total_outgoing_speed_limit: 0
section_lengths: [104,0,0,0,0,0,0,0,0,0,0]
$ openkdx -l 'admin' --nick openkdx login
name: OpenKDX Lab
description: 
version: 1.620
address: 
user_id: 2
users: 1
transfer_port: 0
can_modify_settings: 1
$ openkdx -l 'admin' --nick openkdx server set-name New Server
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/server-settings.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:43622 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:43622 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:43622 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:43622 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:43622 -> 127.0.0.1:10700  100 Set Identity
11  S->C  127.0.0.1:43622 -> 127.0.0.1:10700  101 User List
12  S->C  127.0.0.1:43622 -> 127.0.0.1:10700  105 Server Info
13  C->S  127.0.0.1:43622 -> 127.0.0.1:10700  118 (unnamed; license report)
15  C->S  127.0.0.1:43622 -> 127.0.0.1:10700  170 Get Server Settings
17  S->C  127.0.0.1:43622 -> 127.0.0.1:10700  171 Server Settings Info (reply)
18  C->S  127.0.0.1:43622 -> 127.0.0.1:10700  170 Get Server Settings
19  S->C  127.0.0.1:43622 -> 127.0.0.1:10700  171 Server Settings Info (reply)
20  C->S  127.0.0.1:43622 -> 127.0.0.1:10700  172 Set Server Settings
21  S->C  127.0.0.1:43622 -> 127.0.0.1:10700  173 Set Server Settings Done (reply)
22  C->S  127.0.0.1:43622 -> 127.0.0.1:10700  170 Get Server Settings
23  S->C  127.0.0.1:43622 -> 127.0.0.1:10700  171 Server Settings Info (reply)
30  C->S  127.0.0.1:43628 -> 127.0.0.1:10700  hello
32  S->C  127.0.0.1:43628 -> 127.0.0.1:10700  hello
34  C->S  127.0.0.1:43628 -> 127.0.0.1:10700  proof
35  S->C  127.0.0.1:43628 -> 127.0.0.1:10700  proof
36  C->S  127.0.0.1:43628 -> 127.0.0.1:10700  100 Set Identity
37  S->C  127.0.0.1:43628 -> 127.0.0.1:10700  101 User List
38  S->C  127.0.0.1:43628 -> 127.0.0.1:10700  105 Server Info
39  C->S  127.0.0.1:43628 -> 127.0.0.1:10700  118 (unnamed; license report)
46  C->S  127.0.0.1:43636 -> 127.0.0.1:10700  hello
48  S->C  127.0.0.1:43636 -> 127.0.0.1:10700  hello
50  C->S  127.0.0.1:43636 -> 127.0.0.1:10700  proof
51  S->C  127.0.0.1:43636 -> 127.0.0.1:10700  proof
52  C->S  127.0.0.1:43636 -> 127.0.0.1:10700  100 Set Identity
53  S->C  127.0.0.1:43636 -> 127.0.0.1:10700  101 User List
54  S->C  127.0.0.1:43636 -> 127.0.0.1:10700  105 Server Info
55  C->S  127.0.0.1:43636 -> 127.0.0.1:10700  118 (unnamed; license report)
57  C->S  127.0.0.1:43636 -> 127.0.0.1:10700  170 Get Server Settings
59  S->C  127.0.0.1:43636 -> 127.0.0.1:10700  171 Server Settings Info (reply)
60  C->S  127.0.0.1:43636 -> 127.0.0.1:10700  172 Set Server Settings
61  S->C  127.0.0.1:43636 -> 127.0.0.1:10700  173 Set Server Settings Done (reply)
```
