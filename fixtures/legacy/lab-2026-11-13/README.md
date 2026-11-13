# Lab set 2026-11-13

Written by KDX Server 1.620 (Linux) in the lab, driven by
`lab/scenarios/legacy-samples.sh`: one change per snapshot, so consecutive
files differ in one field. Snapshot `NN` is the state *after* step `NN`.

## `server/stg/` — `KDXServer.stg`

| Snapshot | Change (through) |
|---|---|
| 00-fresh | first run: admin `admin`, guest on |
| 01-name | `openkdx server set-name "Legacy Lab"` (CLI) |
| 02-description | General Settings → Description `A description` (Windows client) |
| 03-address | General Settings → Address `lab.example` |
| 04-speed | General Settings → Total Outgoing Speed Limit on, 20 KB/sec |
| 05-altport | Port Numbers → Alternate Port 10701 |
| 06-greeting | Greeting → Enable Greeting, text `Welcome to the lab` |
| 07-history-login | Server History → Login |
| 08-history-disc | Server History → Disconnected |
| 09-history-size | Server History → start new file at 512 KB |
| 10-deny | Allow/Deny → add `Bad host` / `10.9.8.7`, deny |
| 11-allow-expiry | Allow/Deny → add `Good host` / `10.1.1.1`, allow, expires now |
| 12-tracker | Trackers → add `Lab tracker` / `127.0.0.1` (the expired allow entry is gone) |
| 13-register | Trackers → Register with Trackers |
| 14-ftport | Port Numbers → File Transfer Port 10702 |
| 15-tracker-full | Trackers → add `Second`, comments `tracker comment`, `10.0.0.2`, login `tlogin`, password `tpass`, groups `grp` |
| 16-deny-comment | Allow/Deny → add `Noted` / `10.5.5.5`, deny, comments `a comment` |

Section 7 of every `.stg` holds the server's three default chat rooms
(`Public Chat`, `Voice Chat`, `Admin Lounge`, with their one-line
descriptions): functional defaults the lab server generated, not Haxial
prose.

## `server/accounts/` — `Accounts.dat`

From a separate run (`acc-probe`, same server build), one CLI command each:

| Snapshot | Change |
|---|---|
| 20-fresh | first run |
| 21-u1-created | `accounts create u1 --new-password p1 --name "Name X" --class 1` |
| 22-u1-name-y | `accounts set u1 --name "Name Y"` |
| 23-u1-name-z | `accounts set u1 --name "Name Z"` |
| 24-u1-password | `accounts set u1 --new-password p2` |
| 25-u2-created | `accounts create u2 --new-password p2 --name Second --class 2` |
| 26-class3-renamed | `classes set 3 --name "Custom Renamed"` |
| 27-fa-pics | `folder-access create Pics --icon pictures` |
| 28-u1-deleted | `accounts delete u1` |
| 29-after-stop | server stopped |

## `tracker/` — `KDXTrackerServ.stg`

A fresh Tracker 1.110 after its first start.

## `server/news/` — `News.dat`, and `server/News Archives/`

`legacy-samples.sh`, after deleting the stock welcome post (snapshot 30,
not committed: it is Haxial's text):

| Snapshot | Change |
|---|---|
| 31-news-empty | `news delete "General News" 1` |
| 32-news-one-post | `news post "General News" --subject First --body "Hello from the lab."` |
| 33-news-two-groups | `news create-group "Second Group" --description "Another group" --size-kib 8` |
| 34-news-post-in-second | `news post "Second Group" --subject "In second" --body "Line one.⏎Line two."` |
| 35-news-deleted | `news delete "General News" 2` |
| 36-news-size-4 | `news set-group-size "Second Group" 4` |
| 37-news-archived | four posts of 1500 `x` into `Second Group`; the archive file under `News Archives/Second Group/` is from this step |
