# `History/`

**Status:** read/write (append-only). Byte-exact over the lab file; our
lines are what the server writes.

The server's history log, when Server Settings → Server History has any
kind ticked (`server-stg.md`, main section 0x18). One file per server start,
in `History/` next to the server, named after the start time:

```
History/KDX 26-11-13 06-23-07 PM.txt       KDX yy-mm-dd hh-mm-ss AM|PM.txt
```

A new file also starts when the current one exceeds "Start new file when
size exceeds" (main section 0x1c) `[I]` — not reached in the lab.

Evidence `[O lab history 40–47]`: `fixtures/legacy/lab-2026-11-13/server/
History/`, written by Server 1.620 while the `openkdx` CLI logged in,
created an account, uploaded, listed files and changed settings.

## Lines

UTF-8 BOM, then one LF-terminated line per record, tab-separated:

| Column | Example | Notes |
|---|---|---|
| time | `26/11/13 06:23:09 PM` | `yy/mm/dd hh:mm:ss AM|PM`, the server's local time (UTC in the lab sandbox) |
| event | `Login` | the name 183 History Added gives the kind: `Login`, `Disconnected`, `Create Account`, `Upload File`, `Finished File Upload`, `Get File List`, `Get Server Settings`, `Set Server Settings`, … |
| nick | `openkdx` | |
| login | `admin` | |
| detail | `127.0.0.1:33172` | the address for logins and disconnects, the account for account events, the file name for transfers, empty otherwise |
| extra | `0` | always `0`; transfers add two more columns, bytes done and bytes total (`0 11` then `11 11`) |

The server does not write the `Server Started` record its 183 stream
carries; the first line of a file is the first record after the start.

`openkdx_legacy::history::{HistoryFile, HistoryLine, parse_history,
format_history_time, history_file_name}` and `LegacyDir::append_history`
(one file per `LegacyDir::history_started`, BOM on creation).
