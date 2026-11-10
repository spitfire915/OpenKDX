# openkdx-table

The KDX handshake proves knowledge of a challenge table built into the Haxial programs. The tables are Haxial's copyrighted text and are never shipped; this crate fetches the smallest archive that holds each one, or imports it from a user-supplied file, verifies it by SHA-256, and caches it under the user's data directory.

Features: `fetch` (default) pulls in `reqwest` for downloading; `default-features = false` keeps import, cache, and verification only, for crates that must not carry an HTTP client.

Part of [OpenKDX](../../README.md). Licensed under MIT OR Apache-2.0.
