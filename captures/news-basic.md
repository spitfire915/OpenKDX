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
