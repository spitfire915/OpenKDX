# openkdx-client

An async library for writing KDX clients, on tokio and the Phase 1 crates.

- `Session::connect(addr, &Credentials, ConnectOptions)` logs in as a
  registered Client 1.600 (hello, proof, 100 Set Identity, 118) and tracks
  the user list, greeting, and 213 Must Join Chat.
- `Connection`: requests correlated by txid and reply flag, unsolicited
  messages on a broadcast `Event` stream, `Disconnected` with the server's
  151 when it closes.
- `impl Session` blocks per area: `chat`, `pm`, `news`, `files`, `accounts`
  (accounts, classes, folder access), `admin` (settings, broadcast,
  disconnect/ban, history, connection monitor, shutdown).
- `Transfer`: a second connection keyed by the 105 session key; `download`
  and `upload` with resume on the codec's data mode.
- `Tracker`: groups and servers over TCP.

Tests: mock-server tests run anywhere (`cargo test -p openkdx-client`);
`tests/lab.rs` runs against KDX Server 1.620 and Tracker 1.110 when
`OPENKDX_LAB_SERVER` is set (`lab/differential.sh`). DCC and voice come in
Phase 10.

Part of [OpenKDX](../../README.md). Licensed under MIT OR Apache-2.0.
