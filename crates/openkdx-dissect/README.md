# openkdx-dissect

A lab tool that decodes KDX connections in a pcap or pcapng capture and prints the decrypted, decoded messages. Also a library (`capture`, `connection`, `audit`, `text`) that the other crates' tests use to replay captured streams.

```
openkdx-dissect [-p PASSWORD]... [--audit] [--hex] [--inventory] [--export DIR] [--table FILE | --no-table] CAPTURE
```

- `-p` gives candidate account passwords; the empty (guest) password is always tried. A transfer connection is keyed from its parent's 105 Server Info automatically.
- `--audit` re-encodes every payload and reports `exact`, `MISMATCH`, `undecoded`, or `unknown type`; the exit code is 2 if anything is not exact.
- `--inventory` prints the `## Messages` section for a capture's notes in `captures/`.
- The KDX challenge table (from the `openkdx-table` cache or `OPENKDX_KDX_TABLE`) lets it check the proofs' challenge bytes; without one, only the key proofs are checked.

Depends on `openkdx-proto` and `openkdx-table` (without its `fetch` feature).

Part of [OpenKDX](../../README.md). Licensed under MIT OR Apache-2.0.
