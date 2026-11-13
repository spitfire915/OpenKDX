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
