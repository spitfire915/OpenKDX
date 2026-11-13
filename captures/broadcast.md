# Capture: broadcast

Recorded 2026-11-13 in the lab (`lab/scenarios/broadcast.sh`).
Programs: KDX Server 1.620 (Linux), OpenKDX client (openkdx CLI, 05965e2).
Frames: 67.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start capturing loopback.
1. A guest session ('listener') logs in and stays to print what arrives.
1. The admin sends: server broadcast 'hello all' (154).
1. Both the guest and the other admin session print the 155 Broadcast; so does nobody else on the sending session, which closes at once.

## Server log

```
26/11/13 02:54:31 PM [] KDX Server started o
26/11/13 02:54:32 PM [1 unnamed] Incoming connection from 127.0.0.1:44198
26/11/13 02:54:32 PM [1 unnamed] Set Name: <listener> (guest)
26/11/13 02:54:34 PM [2 unnamed] Incoming connection from 127.0.0.1:44204
26/11/13 02:54:34 PM [2 unnamed] Set Name: <announcer> (admin)
26/11/13 02:54:35 PM [3 unnamed] Incoming connection from 127.0.0.1:44210
26/11/13 02:54:35 PM [3 unnamed] Set Name: <speaker> (admin)
26/11/13 02:54:35 PM [3 speaker] Disconnected
26/11/13 02:54:37 PM [2 announcer] Disconnected
26/11/13 02:54:40 PM [1 listener] Disconnected
```

## Client transcript

```
$ openkdx -l 'admin' --nick announcer login --stay 3
name: New Server
description: 
version: 1.620
address: 
user_id: 2
users: 2
transfer_port: 0
can_modify_settings: 1
$ openkdx -l 'admin' --nick speaker server broadcast hello all
user_id: 3
nick: speaker
user_id: 3
nick: speaker
text: hello all
user_id: 3
nick: #3
$ openkdx -l '' --nick listener login --stay 8
name: New Server
description: 
version: 1.620
address: 
user_id: 1
users: 1
transfer_port: 0
can_modify_settings: 0
user_id: 2
nick: announcer
user_id: 3
nick: speaker
user_id: 3
nick: speaker
text: hello all
user_id: 3
nick: #3
user_id: 2
nick: #2
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/broadcast.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:44198 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:44198 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:44198 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:44198 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:44198 -> 127.0.0.1:10700  100 Set Identity
11  S->C  127.0.0.1:44198 -> 127.0.0.1:10700  101 User List
12  S->C  127.0.0.1:44198 -> 127.0.0.1:10700  105 Server Info
13  C->S  127.0.0.1:44198 -> 127.0.0.1:10700  118 (unnamed; license report)
18  C->S  127.0.0.1:44204 -> 127.0.0.1:10700  hello
20  S->C  127.0.0.1:44204 -> 127.0.0.1:10700  hello
22  C->S  127.0.0.1:44204 -> 127.0.0.1:10700  proof
23  S->C  127.0.0.1:44204 -> 127.0.0.1:10700  proof
24  C->S  127.0.0.1:44204 -> 127.0.0.1:10700  100 Set Identity
25  S->C  127.0.0.1:44204 -> 127.0.0.1:10700  101 User List
26  S->C  127.0.0.1:44204 -> 127.0.0.1:10700  105 Server Info
27  S->C  127.0.0.1:44198 -> 127.0.0.1:10700  103 User Joined
28  C->S  127.0.0.1:44204 -> 127.0.0.1:10700  118 (unnamed; license report)
34  C->S  127.0.0.1:44210 -> 127.0.0.1:10700  hello
36  S->C  127.0.0.1:44210 -> 127.0.0.1:10700  hello
38  C->S  127.0.0.1:44210 -> 127.0.0.1:10700  proof
39  S->C  127.0.0.1:44210 -> 127.0.0.1:10700  proof
40  C->S  127.0.0.1:44210 -> 127.0.0.1:10700  100 Set Identity
41  S->C  127.0.0.1:44210 -> 127.0.0.1:10700  101 User List
42  S->C  127.0.0.1:44210 -> 127.0.0.1:10700  105 Server Info
43  S->C  127.0.0.1:44198 -> 127.0.0.1:10700  103 User Joined
45  S->C  127.0.0.1:44204 -> 127.0.0.1:10700  103 User Joined
47  C->S  127.0.0.1:44210 -> 127.0.0.1:10700  118 (unnamed; license report)
48  C->S  127.0.0.1:44210 -> 127.0.0.1:10700  154 Send Broadcast Message
50  S->C  127.0.0.1:44198 -> 127.0.0.1:10700  155 Rcv Broadcast Message
52  S->C  127.0.0.1:44204 -> 127.0.0.1:10700  155 Rcv Broadcast Message
53  S->C  127.0.0.1:44210 -> 127.0.0.1:10700  155 Rcv Broadcast Message
56  S->C  127.0.0.1:44198 -> 127.0.0.1:10700  104 User Departed
58  S->C  127.0.0.1:44204 -> 127.0.0.1:10700  104 User Departed
63  S->C  127.0.0.1:44198 -> 127.0.0.1:10700  104 User Departed
```
