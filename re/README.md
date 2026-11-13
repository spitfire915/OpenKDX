# re/ — reverse-engineering helpers

Ghidra runs headless on the lab machine, over the Haxial binaries that live
only there. This directory holds what may be committed: the setup script,
our scripts, and `landmarks.md`, a table of function addresses with what
each one does. Ghidra projects, decompiled output and anything that quotes
the binaries stay on the lab (design spec §5.3).

```
re/setup-ghidra.sh      installs Ghidra 12.2 + Temurin JDK 25 under ~/ghidra, imports and
                        analyses the five binaries into project ~/ghidra/projects/openkdx
re/ghidra-run.sh        runs one of our scripts against one program:
                        re/ghidra-run.sh KDXServer.lexe StringsXrefs.java Accounts.dat
re/scripts/StringsXrefs.java   defined strings containing a needle, with referencing functions
re/scripts/Landmarks.java      entry, size, callers and callees of named functions; or every
                               function that calls a libc file-writing routine
re/landmarks.md         the committed findings
```

Ghidra 12 has no Jython; scripts are Java. Haxial's programs keep their
strings in a length-prefixed table addressed by index, so string references
rarely resolve to code: `StringsXrefs` also scans for the string's address
as a bare immediate, and the lab's `strace -k` is the fallback for finding a
file's writer.

Evidence tag for a landmark in `docs/`: `[S server FUN_08060268]`
(binary: `server`, `tracker`, `client`, `appedit`, `sledit`).

Phase 3 pinned every server-side format from lab diffs alone; none needed a
landmark, so `landmarks.md` starts short.
