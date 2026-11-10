# Capture: news-basic

Recorded 2026-11-10 in the lab (`lab/scenarios/news-basic.sh`).
Programs: KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3).
Frames: 27.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'labuser'.
1. Start capturing loopback.
1. Connect → address '127.0.0.1', login 'admin', password (set); press Connect.
1. Open News; expand the server node (shows 'General News', count 1).
1. Click 'General News' to list and read its messages in the lower pane.
1. Click 'Post Message…'; subject 'Lab post', body 'Posted from the lab scenario.'; press Send.
1. Click 'General News' again to re-read (count should now be 2).

## Server log

```
26/11/10 02:04:24 PM [] KDX Server started o
26/11/10 02:04:45 PM [1 unnamed] Incoming connection from 127.0.0.1:48510
26/11/10 02:04:47 PM [1 unnamed] Set Name: <labuser> (admin)
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/news-basic.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:48510 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:48510 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:48510 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:48510 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:48510 -> 127.0.0.1:10700  100 Set Identity
12  S->C  127.0.0.1:48510 -> 127.0.0.1:10700  101 User List
13  S->C  127.0.0.1:48510 -> 127.0.0.1:10700  105 Server Info
16  C->S  127.0.0.1:48510 -> 127.0.0.1:10700  605 Get Newsgroup List
17  S->C  127.0.0.1:48510 -> 127.0.0.1:10700  606 Newsgroup List (reply)
19  C->S  127.0.0.1:48510 -> 127.0.0.1:10700  607 Get News Messages
20  S->C  127.0.0.1:48510 -> 127.0.0.1:10700  608 News Messages (reply)
22  C->S  127.0.0.1:48510 -> 127.0.0.1:10700  611 Post News Message
23  S->C  127.0.0.1:48510 -> 127.0.0.1:10700  613 News Message Posted
25  C->S  127.0.0.1:48510 -> 127.0.0.1:10700  609 Get News Message
26  S->C  127.0.0.1:48510 -> 127.0.0.1:10700  610 News Message (reply)
```
