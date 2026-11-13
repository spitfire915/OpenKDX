# Capture: disconnect-ban

Recorded 2026-11-13 in the lab (`lab/scenarios/disconnect-ban.sh`).
Programs: KDX Server 1.620 (Linux), OpenKDX client (openkdx CLI, 05965e2).
Frames: 120.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start capturing loopback.
1. A login with a wrong password: the server answers the proof with status 1 and closes (log: 'Login failed').
1. A guest session ('victim') logs in and stays.
1. Admin: users (the victim is listed), then server disconnect victim --ban 10 --message bye (150).
1. The victim prints the 151 Disconnect Message (banned, 'bye') and exits with status 3.
1. After the ban (10 s) a guest login succeeds again.

## Server log

```
26/11/13 02:54:53 PM [] KDX Server started o
26/11/13 02:54:54 PM [1 unnamed] Incoming connection from 127.0.0.1:37838
26/11/13 02:54:54 PM [1 unnamed] Set Name: <victim> (guest)
26/11/13 02:54:55 PM [1 victim] Disconnected
26/11/13 02:54:55 PM [2 unnamed] Incoming connection from 127.0.0.1:37842
26/11/13 02:54:55 PM [2 unnamed] Login failed
26/11/13 02:54:55 PM [2 unnamed] Disconnected
26/11/13 02:54:55 PM [3 unnamed] Incoming connection from 127.0.0.1:37852
26/11/13 02:54:55 PM [3 unnamed] Set Name: <victim> (guest)
26/11/13 02:54:56 PM [4 unnamed] Incoming connection from 127.0.0.1:37866
26/11/13 02:54:56 PM [4 unnamed] Set Name: <openkdx> (admin)
26/11/13 02:54:56 PM [4 openkdx] Disconnected
26/11/13 02:54:56 PM [5 unnamed] Incoming connection from 127.0.0.1:37868
26/11/13 02:54:56 PM [5 unnamed] Set Name: <openkdx> (admin)
26/11/13 02:54:56 PM [3 victim] Disconnected
26/11/13 02:54:56 PM [5 openkdx] Disconnected
26/11/13 02:54:57 PM [6 unnamed] Incoming connection from 127.0.0.1:37878
26/11/13 02:54:57 PM [6 unnamed] Set Name: <openkdx> (guest)
26/11/13 02:54:57 PM [6 openkdx] Disconnected
26/11/13 02:55:08 PM [7 unnamed] Incoming connection from 127.0.0.1:35610
26/11/13 02:55:08 PM [7 unnamed] Set Name: <openkdx> (guest)
26/11/13 02:55:08 PM [7 openkdx] Disconnected
```

## Client transcript

```
$ openkdx -l '' --nick victim login --stay 1
name: New Server
description: 
version: 1.620
address: 
user_id: 1
users: 1
transfer_port: 0
can_modify_settings: 0
$ openkdx -l 'admin' --nick openkdx login
$ openkdx -l 'admin' --nick openkdx users
user_id  nick     status  info
3        victim   0
4        openkdx  0
$ openkdx -l 'admin' --nick openkdx server disconnect victim --ban 10 --message bye
$ openkdx -l '' --nick victim login --stay 20
name: New Server
description: 
version: 1.620
address: 
user_id: 3
users: 1
transfer_port: 0
can_modify_settings: 0
user_id: 4
nick: openkdx
user_id: 4
nick: #4
user_id: 5
nick: openkdx
banned: true
shutdown: false
by: openkdx
message: bye
message: bye
error: 
$ openkdx -l '' --nick openkdx login
name: New Server
description: 
version: 1.620
address: 
user_id: 6
users: 1
transfer_port: 0
can_modify_settings: 0
$ openkdx -l '' --nick openkdx login
name: New Server
description: 
version: 1.620
address: 
user_id: 7
users: 1
transfer_port: 0
can_modify_settings: 0
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/disconnect-ban.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:37838 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:37838 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:37838 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:37838 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:37838 -> 127.0.0.1:10700  100 Set Identity
11  S->C  127.0.0.1:37838 -> 127.0.0.1:10700  101 User List
12  S->C  127.0.0.1:37838 -> 127.0.0.1:10700  105 Server Info
13  C->S  127.0.0.1:37838 -> 127.0.0.1:10700  118 (unnamed; license report)
21  C->S  127.0.0.1:37842 -> 127.0.0.1:10700  hello
23  S->C  127.0.0.1:37842 -> 127.0.0.1:10700  hello
25  C->S  127.0.0.1:37842 -> 127.0.0.1:10700  proof
25  C->S  127.0.0.1:37842 -> 127.0.0.1:10700  stopped: no candidate key matches the client's proof; try --password
26  S->C  127.0.0.1:37842 -> 127.0.0.1:10700  proof
33  C->S  127.0.0.1:37852 -> 127.0.0.1:10700  hello
35  S->C  127.0.0.1:37852 -> 127.0.0.1:10700  hello
37  C->S  127.0.0.1:37852 -> 127.0.0.1:10700  proof
38  S->C  127.0.0.1:37852 -> 127.0.0.1:10700  proof
39  C->S  127.0.0.1:37852 -> 127.0.0.1:10700  100 Set Identity
40  S->C  127.0.0.1:37852 -> 127.0.0.1:10700  101 User List
41  S->C  127.0.0.1:37852 -> 127.0.0.1:10700  105 Server Info
42  C->S  127.0.0.1:37852 -> 127.0.0.1:10700  118 (unnamed; license report)
47  C->S  127.0.0.1:37866 -> 127.0.0.1:10700  hello
49  S->C  127.0.0.1:37866 -> 127.0.0.1:10700  hello
51  C->S  127.0.0.1:37866 -> 127.0.0.1:10700  proof
52  S->C  127.0.0.1:37866 -> 127.0.0.1:10700  proof
53  C->S  127.0.0.1:37866 -> 127.0.0.1:10700  100 Set Identity
54  S->C  127.0.0.1:37866 -> 127.0.0.1:10700  101 User List
55  S->C  127.0.0.1:37866 -> 127.0.0.1:10700  105 Server Info
56  S->C  127.0.0.1:37852 -> 127.0.0.1:10700  103 User Joined
58  C->S  127.0.0.1:37866 -> 127.0.0.1:10700  118 (unnamed; license report)
62  S->C  127.0.0.1:37852 -> 127.0.0.1:10700  104 User Departed
67  C->S  127.0.0.1:37868 -> 127.0.0.1:10700  hello
69  S->C  127.0.0.1:37868 -> 127.0.0.1:10700  hello
71  C->S  127.0.0.1:37868 -> 127.0.0.1:10700  proof
72  S->C  127.0.0.1:37868 -> 127.0.0.1:10700  proof
73  C->S  127.0.0.1:37868 -> 127.0.0.1:10700  100 Set Identity
74  S->C  127.0.0.1:37868 -> 127.0.0.1:10700  101 User List
75  S->C  127.0.0.1:37868 -> 127.0.0.1:10700  105 Server Info
76  S->C  127.0.0.1:37852 -> 127.0.0.1:10700  103 User Joined
77  C->S  127.0.0.1:37868 -> 127.0.0.1:10700  118 (unnamed; license report)
78  C->S  127.0.0.1:37868 -> 127.0.0.1:10700  150 Disconnect User
80  S->C  127.0.0.1:37852 -> 127.0.0.1:10700  151 Disconnect Message
82  S->C  127.0.0.1:37868 -> 127.0.0.1:10700  1 Status (reply)
83  S->C  127.0.0.1:37868 -> 127.0.0.1:10700  104 User Departed
92  C->S  127.0.0.1:37878 -> 127.0.0.1:10700  hello
94  S->C  127.0.0.1:37878 -> 127.0.0.1:10700  hello
96  C->S  127.0.0.1:37878 -> 127.0.0.1:10700  proof
97  S->C  127.0.0.1:37878 -> 127.0.0.1:10700  proof
98  C->S  127.0.0.1:37878 -> 127.0.0.1:10700  100 Set Identity
99  S->C  127.0.0.1:37878 -> 127.0.0.1:10700  101 User List
100  S->C  127.0.0.1:37878 -> 127.0.0.1:10700  105 Server Info
101  C->S  127.0.0.1:37878 -> 127.0.0.1:10700  118 (unnamed; license report)
108  C->S  127.0.0.1:35610 -> 127.0.0.1:10700  hello
110  S->C  127.0.0.1:35610 -> 127.0.0.1:10700  hello
112  C->S  127.0.0.1:35610 -> 127.0.0.1:10700  proof
113  S->C  127.0.0.1:35610 -> 127.0.0.1:10700  proof
114  C->S  127.0.0.1:35610 -> 127.0.0.1:10700  100 Set Identity
115  S->C  127.0.0.1:35610 -> 127.0.0.1:10700  101 User List
116  S->C  127.0.0.1:35610 -> 127.0.0.1:10700  105 Server Info
117  C->S  127.0.0.1:35610 -> 127.0.0.1:10700  118 (unnamed; license report)
```
