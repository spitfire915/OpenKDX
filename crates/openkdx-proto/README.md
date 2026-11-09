# openkdx-proto

The wire format of the KDX protocol: the XOR'd handshake, Twofish framing, the 16-byte message header, typed payloads for every message, string and record encodings, and the privilege bits. This crate does no I/O and depends on no async runtime, network, or UI library, so any tool can use it.

Part of [OpenKDX](../../README.md). Licensed under MIT OR Apache-2.0.
