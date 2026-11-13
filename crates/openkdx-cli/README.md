# openkdx-cli

A thin command-line KDX client over `openkdx-client`: log in, chat, send
private messages, read and post news, list and transfer files, manage
accounts, classes and folder access, administer a server, browse a tracker.
It drives the lab's differential tests and capture scenarios.

The binary is named `openkdx` (design §8). The design (§5.3) reserves that
name for the GUI client; when `openkdx-app` ships as `openkdx` (Phase 7) this
binary becomes `openkdx-cli`.

```
openkdx -s HOST[:PORT] -l LOGIN -p PASSWORD <COMMAND>
openkdx run < commands.txt      # many commands on one session
```

Exit codes: 0 ok; 1 usage or local error; 2 login failed or server error;
3 disconnected by the server; 4 timeout. `--json` prints one object per line.
The KDX challenge table comes from `OPENKDX_KDX_TABLE` or the `openkdx-table`
cache; see `crates/openkdx-table`.
