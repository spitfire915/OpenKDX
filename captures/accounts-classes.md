# Capture: accounts-classes

Recorded 2026-11-13 in the lab (`lab/scenarios/accounts-classes.sh`).
Programs: KDX Server 1.620 (Linux), OpenKDX client (openkdx CLI, 05965e2).
Frames: 38.

## Steps

1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start capturing loopback.
1. One admin session runs: classes list (505/506), classes show 1 (502/503), accounts list (512/513), accounts create tester (507 → Status 0x1fb), accounts show tester (509/510), accounts set tester --name 'Tester 2' (511 → Status 0x1ff), accounts show, accounts delete tester (508 → 515), accounts list.

## Server log

```
26/11/13 02:53:47 PM [] KDX Server started o
26/11/13 02:53:48 PM [1 unnamed] Incoming connection from 127.0.0.1:46784
26/11/13 02:53:48 PM [1 unnamed] Set Name: <openkdx> (admin)
26/11/13 02:53:48 PM [1 openkdx] Disconnected
```

## Client transcript

```
$ openkdx -l 'admin' --nick openkdx run --keep-going
> classes list
index  name            speed_limit  color   privileges
0      Administrator   0            ff0000  unnamed-bit-2,can-login,download,upload,create-aliases,move,rename,delete,set-file-folder-attributes,allow-simultaneous-downloads,view-folder-access,modify-folder-access,join-chat,send-chat,send-messages,broadcast,create-private-chat,set-chat-topic,create-modify-chats,get-user-info,disconnect-users,ban-users,view-accounts,create-accounts,modify-accounts,change-own-password,use-any-name,modify-server-settings,view-server-history,shutdown-server,disable-encryption,view-news,post-news,delete-news-messages,create-modify-newsgroups,see-logins-in-news,view-processes
1      User            0            0000ff  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,broadcast,create-private-chat,set-chat-topic,get-user-info,disconnect-users,change-own-password,use-any-name,view-news,post-news,see-logins-in-news
2      Guest           0            000000  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,use-any-name,view-news
3      Custom Class 1  0            000000  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,use-any-name,view-news
4      Custom Class 2  0            000000  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,use-any-name,view-news
5      Custom Class 3  0            000000  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,use-any-name,view-news
6      Custom Class 4  0            000000  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,use-any-name,view-news
7      Custom Class 5  0            000000  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,use-any-name,view-news
8      Custom Class 6  0            000000  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,use-any-name,view-news
9      Custom Class 7  0            000000  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,use-any-name,view-news
10     Custom Class 8  0            000000  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,use-any-name,view-news
11     Custom Class 9  0            000000  unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,use-any-name,view-news
> classes show 1
index: 1
name: User
speed_limit: 0
flood_protection: 60
privileges: unnamed-bit-2,can-login,download,upload,join-chat,send-chat,send-messages,broadcast,create-private-chat,set-chat-topic,get-user-info,disconnect-users,change-own-password,use-any-name,view-news,post-news,see-logins-in-news
editor: 
> accounts list
login  name           class  created              last_login
admin  Administrator  0      2026-11-13 14:53:39  2026-11-13 14:53:48
guest  Guest          2      2026-11-13 14:53:39  1900-01-01 00:00:00
> accounts create tester --new-password t --name Tester --class 1 --privileges can-login,download
created tester
> accounts show tester
login: tester
name: Tester
class: 1
has_password: true
privileges: can-login,download
privilege_mask: can-login,download
created: 2026-11-13 14:53:48
modified: 2026-11-13 14:53:48
last_login: 1900-01-01 00:00:00
editor: admin
> accounts set tester --name "Tester 2"
> accounts show tester
login: tester
name: Tester 2
class: 1
has_password: true
privileges: can-login,download
privilege_mask: can-login,download
created: 2026-11-13 14:53:48
modified: 2026-11-13 14:53:48
last_login: 1900-01-01 00:00:00
editor: admin
> accounts delete tester
> accounts list
login  name           class  created              last_login
admin  Administrator  0      2026-11-13 14:53:39  2026-11-13 14:53:48
guest  Guest          2      2026-11-13 14:53:39  1900-01-01 00:00:00
```

## Messages

Decoded by `openkdx-dissect --inventory -p openkdx-lab captures/accounts-classes.pcapng`; checked by `crates/openkdx-dissect/tests/captures.rs`.

```
4  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  hello
6  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  hello
8  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  proof
9  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  proof
10  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  100 Set Identity
11  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  101 User List
12  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  105 Server Info
13  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  118 (unnamed; license report)
15  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  505 Get Account Class List
17  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  506 Account Class List (reply)
18  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  502 Get Account Class
19  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  503 Account Class Info (reply)
20  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  512 Get Account List
21  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  513 Account List (reply)
22  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  507 Create Account
23  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  1 Status (reply)
24  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  509 Get Account
25  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  510 Account Info (reply)
26  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  509 Get Account
27  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  510 Account Info (reply)
28  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  511 Set Account
29  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  1 Status (reply)
30  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  509 Get Account
31  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  510 Account Info (reply)
32  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  508 Delete Account
33  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  515 Account Deleted
34  C->S  127.0.0.1:46784 -> 127.0.0.1:10700  512 Get Account List
35  S->C  127.0.0.1:46784 -> 127.0.0.1:10700  513 Account List (reply)
```
