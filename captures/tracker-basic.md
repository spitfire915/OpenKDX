# Capture: tracker-basic

Recorded 2026-11-09 in the lab (`lab/scenarios/tracker-basic.sh`).
Programs: KDX Server 1.620 (Linux), KDX Tracker 1.110 (Linux), KDX Client 1.600 (Windows under Wine 6.0.3).
Frames: 20. Screenshot at the end: `tracker-basic.png`.

## Steps

1. Start KDX Tracker 1.110 (Linux) in the sandbox.
1. Start KDX Server 1.620 (Linux) in the sandbox; first-run setup creates admin 'admin', guest access enabled.
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'labuser'.
1. Connect → address '127.0.0.1', login 'admin', password (set); press Connect.
1. Commands → 127.0.0.1:10700 → Server Settings.
1. Switch the panel popup to 'Trackers'.
1. Add Tracker…: name 'Lab tracker', address 127.0.0.1, group 'General'; Save.
1. Tick 'Register with Trackers'; Save the server settings.
1. Quit the client.
1. Start capturing loopback.
1. Restart the server: it registers with the tracker on startup (one UDP datagram to 10800).
1. Start KDX Client 1.600 (Windows, under Wine); accept the licence; set user name 'trackeruser'.
1. Fresh client: Connect… with Type 'Tracker', address 127.0.0.1. The Tracker window lists the groups; 'General' has count 1.
1. Click 'General': the server list shows 'New Server'.

## Server log

```
26/11/09 10:31:45 PM [] KDX Server started o
```
