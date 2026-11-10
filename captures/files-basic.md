# Capture: files-basic

Recorded 2026-11-10 in the lab (`lab/scenarios/files-basic.sh`).
Programs: KDX Server 1.620 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3).
Frames: 38.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Seed the server's base folder with tiny.txt (3 bytes, 'abc').
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'labuser'.
1. Start capturing loopback.
1. Connect → address '127.0.0.1', login 'admin', password (set); press Connect.
1. Open File Browser: the base lists 'Admin Dropbox' [DB], tiny.txt (3), 'Uploads' [UL].
1. Right-click tiny.txt → Download. The client opens a second connection for the transfer.
1. Verified: Downloads/tiny.txt arrived on the client (abc).
1. Click Refresh to list the base again.

## Server log

```
26/11/10 02:06:38 PM [] KDX Server started o
26/11/10 02:07:00 PM [1 unnamed] Incoming connection from 127.0.0.1:55866
26/11/10 02:07:02 PM [1 unnamed] Set Name: <labuser> (admin)
26/11/10 02:07:12 PM [2 unnamed] Incoming connection from 127.0.0.1:46914
26/11/10 02:07:12 PM [2 labuser] Download "tiny.txt".
26/11/10 02:07:12 PM [2 labuser] Transfer Disconnect
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/files-basic.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:55866 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:55866 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:55866 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:55866 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:55866 -> 127.0.0.1:10700  100 Set Identity
12  S->C  127.0.0.1:55866 -> 127.0.0.1:10700  101 User List
13  S->C  127.0.0.1:55866 -> 127.0.0.1:10700  105 Server Info
15  C->S  127.0.0.1:55866 -> 127.0.0.1:10700  400 Get File List
16  S->C  127.0.0.1:55866 -> 127.0.0.1:10700  401 File List (reply)
21  C->S  127.0.0.1:46914 -> 127.0.0.1:10700  hello
23  S->C  127.0.0.1:46914 -> 127.0.0.1:10700  hello
25  C->S  127.0.0.1:46914 -> 127.0.0.1:10700  proof
26  S->C  127.0.0.1:46914 -> 127.0.0.1:10700  proof
27  C->S  127.0.0.1:46914 -> 127.0.0.1:10700  412 Download File
28  S->C  127.0.0.1:46914 -> 127.0.0.1:10700  413 Download File (R) (reply)
29  S->C  127.0.0.1:46914 -> 127.0.0.1:10700  414 Download File (D) data header, 3 bytes
30  S->C  127.0.0.1:46914 -> 127.0.0.1:10700  transfer data, 3 bytes
31  S->C  127.0.0.1:46914 -> 127.0.0.1:10700  414 Download File (D) data header, 0 bytes
36  C->S  127.0.0.1:55866 -> 127.0.0.1:10700  400 Get File List
37  S->C  127.0.0.1:55866 -> 127.0.0.1:10700  401 File List (reply)
```
