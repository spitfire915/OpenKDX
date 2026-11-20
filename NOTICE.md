# Notice

**No Haxial material.** OpenKDX contains no source code, binaries, challenge-table text, artwork, or icons from Haxial Pty Ltd. It is an independent, clean-room reimplementation of the KDX network protocol and file formats, built from packet captures, published analysis, and static analysis of programs that users obtain themselves. The challenge tables that the protocol requires are Haxial's copyrighted text; OpenKDX fetches or imports them at runtime on the user's machine and never distributes them.

**The OpenKDX mark.** The OpenKDX logo is an original design that borrows the visual vocabulary of KDX (a glossy X, a chrome upper half, a coloured lower half; gold for the client, green for the server, purple for the tracker). It is not a copy, trace, or derivative of Haxial's artwork.

**Name.** "KDX" is used descriptively, to indicate that OpenKDX speaks the KDX protocol and reads KDX files. OpenKDX is not affiliated with, sponsored by, or endorsed by Haxial Pty Ltd or its former principals.

**Co-authorship.** OpenKDX is co-authored by its maintainer and Claude, an AI model by Anthropic. Commits from 2026-11-19 carry a `Co-Authored-By: Claude <noreply@anthropic.com>` trailer; earlier commits were produced the same way and are covered by this notice.

**Shared libraries.** From Phase 3b, the protocol and server libraries OpenKDX uses are the `kdx-*` crates of [scottanderson/kdx](https://github.com/scottanderson/kdx) (MIT OR Apache-2.0), developed in a fork and contributed upstream. OpenKDX holds the applications.
