<p align="center">
  <img src="assets/logo/client.svg" width="240" alt="OpenKDX Client">
  <img src="assets/logo/server.svg" width="240" alt="OpenKDX Server">
  <img src="assets/logo/tracker.svg" width="240" alt="OpenKDX Tracker">
</p>

# OpenKDX

A modern, memory-safe reimplementation of Haxial KDX — the early-2000s encrypted BBS-style chat, news, and file-sharing system — in Rust, wire-compatible with the original programs so old and new can share a network.

**Status:** Phase 1 complete — the protocol core. `openkdx-proto` speaks the whole KDX 1.x wire format (handshake, Twofish framing, tracker registration, every message type), `openkdx-codec` frames it for async I/O, and `openkdx-dissect` decodes lab captures: all seven captures of the original programs decode and re-encode byte for byte. No client, server, or tracker is usable yet; Phase 2 (the client library and a CLI) is next. See the [phases](docs/superpowers/specs/2026-11-08-openkdx-design.md#8-phases).

| Program | Binary | What it is |
|---|---|---|
| Client | `openkdx` | A desktop client with a faithful "Classic" mode (Haxial's own window system, one OS window per function, the KDX Dock, original `.hap` appearance files) and a "Modern" mode that docks the same windows as tabs in one unified window. |
| Server | `openkdxd` | A headless daemon that serves an existing KDX Server directory (`Accounts.dat`, `KDXServer.stg`, `Bases/`, news) unchanged, administered from any KDX client. |
| Tracker | `openkdx-tracker` | A tracker that original and OpenKDX servers register with and original and OpenKDX clients browse. |

## Compatibility

OpenKDX targets the last Haxial releases: **KDX Server 1.620**, **KDX Client 1.600**, and **KDX Tracker 1.110**. Every claim of compatibility is verified by differential testing against those programs running in a sandboxed lab.

The protocol reference OpenKDX builds on is [scottanderson/kdx](https://github.com/scottanderson/kdx/tree/main/docs/protocol). Where our captures or analysis differ, the deltas are documented in `docs/protocol/`.

## What is here

| Crate | Status | What it does |
|---|---|---|
| [`openkdx-proto`](crates/openkdx-proto) | done | The wire format, with no I/O: handshake, session encryption and framing, UDP tracker registration, and a typed struct for every message (1–1107, 4000–4201). |
| [`openkdx-codec`](crates/openkdx-codec) | done | `tokio_util::codec` framing for the handshake and the session, with streaming file-transfer data. |
| [`openkdx-dissect`](crates/openkdx-dissect) | done | Decodes KDX connections in pcap/pcapng captures; `--audit` proves each payload re-encodes byte-exact. |
| [`openkdx-table`](crates/openkdx-table) | done | Obtains the challenge tables at runtime ([how](docs/setup/challenge-tables.md)); they are never shipped. |
| `openkdx-client`, `openkdx-server`, `openkdx-tracker`, `openkdx-legacy`, `openkdx-ui`, `openkdx-app`, `openkdxd` | planned | Phases 2–10. |

The [lab](lab/README.md) runs the original programs in a sandbox and records the [captures](captures/README.md) that every protocol claim is checked against. Try the dissector on one:

```sh
cargo run -p openkdx-dissect -- -p openkdx-lab --audit captures/chat-basic.pcapng
```

## Documentation

- [Design](docs/superpowers/specs/2026-11-08-openkdx-design.md) — intent, goals, constraints, architecture, phases.
- [Protocol notes](docs/protocol/README.md) — what we confirmed, and where our captures differ from the reference spec.
- [Captures](captures/README.md) — the lab scenarios and what each one shows.
- [Contributing](CONTRIBUTING.md) — checks, commit style, and the legal rules.
- [NOTICE](NOTICE.md) — what this project does and does not contain.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
