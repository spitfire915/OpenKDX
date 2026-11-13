# Capture: server-history

Recorded 2026-11-13 in the lab (`lab/scenarios/server-history.sh`).
Programs: KDX Server 1.620 (Linux), OpenKDX client (openkdx CLI, 05965e2).
Frames: 75.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start capturing loopback.
1. Admin: server history --follow 8 (180 → 181 lists the Server Started and Login records; 183 follows for each new record).
1. A guest logs in and out: the historian prints 183 Login and 183 Disconnected records.
1. A guest posts a news message: another 183.
1. The historian leaves (182).

## Server log

```
26/11/13 02:55:21 PM [] KDX Server started o
26/11/13 02:55:22 PM [1 unnamed] Incoming connection from 127.0.0.1:37426
26/11/13 02:55:22 PM [1 unnamed] Set Name: <historian> (admin)
26/11/13 02:55:24 PM [2 unnamed] Incoming connection from 127.0.0.1:37436
26/11/13 02:55:24 PM [2 unnamed] Set Name: <passer-by> (guest)
26/11/13 02:55:24 PM [2 passer-by] Disconnected
26/11/13 02:55:24 PM [3 unnamed] Incoming connection from 127.0.0.1:37440
26/11/13 02:55:24 PM [3 unnamed] Set Name: <poster> (guest)
26/11/13 02:55:30 PM [1 historian] Disconnected
26/11/13 02:55:54 PM [3 poster] Disconnected
```

## Client transcript

```
$ openkdx -l '' --nick passer-by login
name: New Server
description: 
version: 1.620
address: 
user_id: 2
users: 2
transfer_port: 0
can_modify_settings: 0
$ openkdx -l '' --nick poster news post 1 --subject hello --body from the lab
$ openkdx -l 'admin' --nick historian server history --follow 8
time                 event           user_id  nick       login  address          detail  path
2026-11-13 14:55:21  Server Started  0
2026-11-13 14:55:22  Login           1        historian  admin  127.0.0.1:37426
2026-11-13 14:55:22  ?               1        historian  admin  127.0.0.1:37426
user_id: 2
nick: passer-by
time: 4003570524536
event: Login
nick: passer-by
login: guest
detail: 
time: 4003570524537
event: Disconnected
nick: passer-by
login: guest
detail: 127.0.0.1:37436
user_id: 2
nick: #2
user_id: 3
nick: poster
time: 4003570524647
event: Login
nick: poster
login: guest
detail: 
time: 4003570524688
event: ?
nick: poster
login: guest
detail: 
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/server-history.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:37426 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:37426 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:37426 -> 127.0.0.1:10700  100 Set Identity
11  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  101 User List
12  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  105 Server Info
13  C->S  127.0.0.1:37426 -> 127.0.0.1:10700  118 (unnamed; license report)
15  C->S  127.0.0.1:37426 -> 127.0.0.1:10700  180 Server History Join
17  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  181 Server History Join (R) (reply)
22  C->S  127.0.0.1:37436 -> 127.0.0.1:10700  hello
24  S->C  127.0.0.1:37436 -> 127.0.0.1:10700  hello
26  C->S  127.0.0.1:37436 -> 127.0.0.1:10700  proof
27  S->C  127.0.0.1:37436 -> 127.0.0.1:10700  proof
28  C->S  127.0.0.1:37436 -> 127.0.0.1:10700  100 Set Identity
29  S->C  127.0.0.1:37436 -> 127.0.0.1:10700  101 User List
30  S->C  127.0.0.1:37436 -> 127.0.0.1:10700  105 Server Info
31  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  103 User Joined
33  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  183 Server History Added
35  C->S  127.0.0.1:37436 -> 127.0.0.1:10700  118 (unnamed; license report)
37  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  183 Server History Added
41  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  104 User Departed
46  C->S  127.0.0.1:37440 -> 127.0.0.1:10700  hello
48  S->C  127.0.0.1:37440 -> 127.0.0.1:10700  hello
50  C->S  127.0.0.1:37440 -> 127.0.0.1:10700  proof
51  S->C  127.0.0.1:37440 -> 127.0.0.1:10700  proof
52  C->S  127.0.0.1:37440 -> 127.0.0.1:10700  100 Set Identity
53  S->C  127.0.0.1:37440 -> 127.0.0.1:10700  101 User List
54  S->C  127.0.0.1:37440 -> 127.0.0.1:10700  105 Server Info
55  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  103 User Joined
57  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  183 Server History Added
59  C->S  127.0.0.1:37440 -> 127.0.0.1:10700  118 (unnamed; license report)
61  C->S  127.0.0.1:37440 -> 127.0.0.1:10700  611 Post News Message
63  S->C  127.0.0.1:37440 -> 127.0.0.1:10700  1 Status (reply)
64  S->C  127.0.0.1:37426 -> 127.0.0.1:10700  183 Server History Added
67  C->S  127.0.0.1:37426 -> 127.0.0.1:10700  182 Server History Leave
71  S->C  127.0.0.1:37440 -> 127.0.0.1:10700  104 User Departed
```
