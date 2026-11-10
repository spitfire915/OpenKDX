# Capture: chat-basic

Recorded 2026-11-10 in the lab (`lab/scenarios/chat-basic.sh`).
Programs: KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3).
Frames: 29.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'labuser'.
1. Start capturing loopback.
1. Connect → address '127.0.0.1', login 'admin', password (set); press Connect.
1. Open Chat; expand the server in the Chat List (shows Admin Lounge, Public Chat, Voice Chat).
1. Click 'Public Chat' to join it; the window becomes 'Chat: 127.0.0.1:10700' with a tab per server.
1. Type 'hello from the lab' in the input box and press Return.
1. Type '/me waves' and press Return (action format).
1. Click 'Topic:'; in the Set topic dialog type 'lab topic' and press Set.

## Server log

```
26/11/10 02:03:27 PM [] KDX Server started o
26/11/10 02:03:49 PM [1 unnamed] Incoming connection from 127.0.0.1:37182
26/11/10 02:03:51 PM [1 unnamed] Set Name: <labuser> (admin)
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/chat-basic.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:37182 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:37182 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:37182 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:37182 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:37182 -> 127.0.0.1:10700  100 Set Identity
12  S->C  127.0.0.1:37182 -> 127.0.0.1:10700  101 User List
13  S->C  127.0.0.1:37182 -> 127.0.0.1:10700  105 Server Info
15  C->S  127.0.0.1:37182 -> 127.0.0.1:10700  214 Get Chat List
16  S->C  127.0.0.1:37182 -> 127.0.0.1:10700  215 Chat List (reply)
18  C->S  127.0.0.1:37182 -> 127.0.0.1:10700  202 Join Chat
19  S->C  127.0.0.1:37182 -> 127.0.0.1:10700  203 Chat Window Info (reply)
21  C->S  127.0.0.1:37182 -> 127.0.0.1:10700  207 Send Chat Text
22  S->C  127.0.0.1:37182 -> 127.0.0.1:10700  208 Receive Chat Text
24  C->S  127.0.0.1:37182 -> 127.0.0.1:10700  207 Send Chat Text
25  S->C  127.0.0.1:37182 -> 127.0.0.1:10700  208 Receive Chat Text
27  C->S  127.0.0.1:37182 -> 127.0.0.1:10700  209 Set Chat Topic
28  S->C  127.0.0.1:37182 -> 127.0.0.1:10700  210 Chat Topic Changed
```
