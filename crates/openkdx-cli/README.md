# openkdx-cli

A thin command-line KDX client over `kdx-client` (the kdx fork's client
library, pinned in `Cargo.toml`): log in, chat, send
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

Exit codes: 0 ok; 1 usage or local error; 2 login failed, no challenge
table, or server error; 3 disconnected by the server; 4 timeout (`--timeout`
bounds each request). `--json` prints one object per line. The KDX challenge
table comes from `OPENKDX_KDX_TABLE` or the `openkdx-table` cache
(`openkdx-table fetch` fills it); see `crates/openkdx-table`.

`openkdx run` keeps one `kdx_client::Client` across the lines it reads; the
lab scenarios depend on that. The tests in `tests/cli.rs` run the binary
against `kdx_client::mock`.
