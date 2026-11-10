# openkdx-codec

`tokio_util::codec` framing for KDX connections on top of `openkdx-proto`.

- `HandshakeCodec`: the peer's hello and 44-byte proof, in order; encodes ours.
- `SessionCodec`: the encrypted 16-byte header and padded payload blocks. It yields `Event::Message` for ordinary messages and, for 414/417 transfer data, `Event::DataStart`, `Event::Data` chunks, and `Event::DataEnd`, without buffering a whole fork. Encoding takes `Item::Message`, `Item::DataHeader`, and `Item::Data`. The receive cap is checked before any payload is buffered. `with_scramble(Scramble::Hgc)` serves a GConsole peer.

Part of [OpenKDX](../../README.md). Licensed under MIT OR Apache-2.0.
