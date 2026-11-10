<p align="center">
  <img src="assets/logo/client.svg" width="240" alt="OpenKDX Client">
  <img src="assets/logo/server.svg" width="240" alt="OpenKDX Server">
  <img src="assets/logo/tracker.svg" width="240" alt="OpenKDX Tracker">
</p>

# OpenKDX

A modern, memory-safe reimplementation of Haxial KDX — the early-2000s encrypted BBS-style chat, news, and file-sharing system — in Rust, wire-compatible with the original programs so old and new can share a network.

**Status:** Phase 0 — repository, CI, and lab setup. Nothing is usable yet.

| Program | Binary | What it is |
|---|---|---|
| Client | `openkdx` | A desktop client with a faithful "Classic" mode (Haxial's own window system, one OS window per function, the KDX Dock, original `.hap` appearance files) and a "Modern" mode that docks the same windows as tabs in one unified window. |
| Server | `openkdxd` | A headless daemon that serves an existing KDX Server directory (`Accounts.dat`, `KDXServer.stg`, `Bases/`, news) unchanged, administered from any KDX client. |
| Tracker | `openkdx-tracker` | A tracker that original and OpenKDX servers register with and original and OpenKDX clients browse. |

## Compatibility

OpenKDX targets the last Haxial releases: **KDX Server 1.620**, **KDX Client 1.600**, and **KDX Tracker 1.110**. Every claim of compatibility is verified by differential testing against those programs running in a sandboxed lab.

The protocol reference OpenKDX builds on is [scottanderson/kdx](https://github.com/scottanderson/kdx/tree/main/docs/protocol). Where our captures or analysis differ, the deltas are documented in `docs/protocol/`.

## Documentation

- [Design](docs/superpowers/specs/2026-11-08-openkdx-design.md) — intent, goals, constraints, architecture, phases.
- [NOTICE](NOTICE.md) — what this project does and does not contain.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
