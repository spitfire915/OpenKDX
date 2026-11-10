# openkdx-proto

The wire format of the KDX protocol, with no I/O: the XOR'd handshake (`handshake`, `account`, `challenge`, `xor`), Twofish-256 ECB framing and the header scramble (`session`), the tracker's UDP registration (`registration`), every message type's name (`message_type`), and a typed struct per message (`msg`, one module per page of the reference spec) behind the `Payload` trait and the `Message` enum.

Every decoded payload re-encodes to exactly the bytes it came from: `ignored_*`, `extra`, and `padding` bytes are kept as fields. Trailing bytes a struct does not account for fail decoding, which is how spec errors surface. The challenge tables are Haxial's text and reach this crate only as bytes (`openkdx-table` obtains them).

Depends on no async runtime, network, or UI library, so any tool can use it.

Part of [OpenKDX](../../README.md). Licensed under MIT OR Apache-2.0.
