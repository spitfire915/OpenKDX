# OpenKDX — Design

**Status:** draft for approval
**Date:** 2026-11-08
**Repository:** https://github.com/spitfire915/OpenKDX
**Destination in repo:** `docs/superpowers/specs/2026-11-08-openkdx-design.md`

## 1. Intent

Haxial KDX (2001–2004) is a BBS-style encrypted client/server system — chat, private messages, news, file sharing with resume, accounts and classes, trackers, DCC voice, remote administration. Its developer disappeared two decades ago, leaving only 32-bit binaries for Mac OS 9/X (PPC), Windows, and Linux. A community still runs it through Wine wrappers, i386 compatibility libraries, and Docker images.

OpenKDX is a complete, modern, memory-safe reimplementation of the KDX client, server, and tracker in Rust, **wire-compatible with the original programs** (Server 1.620, Client 1.600, Tracker 1.110), so that old and new can be mixed on the same network and the community can migrate one host at a time. It is "nostalgia brought into the modern age": the protocol, the data formats, and above all the *feel* of the original are preserved; the implementation is new, open, and built to outlive any single maintainer.

## 2. Goals

1. **Compatibility.** An OpenKDX client works against an original server; an original client works against an OpenKDX server; both work with either tracker. Verified continuously by differential testing against the original binaries.
2. **Completeness.** Every feature the original client and server exposed: chat rooms, private messages, news, files (browse, search, download, upload, resume, folders, aliases, drop boxes, upload folders), accounts, classes, folder access, server settings, connection monitor, server history, process monitor, broadcast, disconnect/ban, remote shutdown, trackers, **IRC gateway**, DCC, voice, view/control display, and the client side of **GConsole** (the `%HGC` handshake with its own challenge table, messages 4000–4201, port 10900), so the client can still reach a Haxial GConsole host.
2a. **A first-class tracker.** The tracker is a shipped program with the same standing as client and server: UDP registration from original and OpenKDX servers, TCP browse from original and OpenKDX clients, groups, accounts, settings and statistics (700–721), its own Docker image and service units, and its own logo variant. The community still runs trackers today.
3. **Faithful classic UI.** A "Classic" mode that reproduces Haxial's self-drawn window system — the button bar, one OS-level window per function, the KDX Dock for minimised windows, the Window Switcher, Small Button Bar — and loads original `.hap` appearance files and sound lists.
4. **Modern unified UI.** A "Modern" mode with the identical look that docks every function into one window with tabs.
5. **Old data just works.** Point the server at an existing KDX Server directory (`Accounts.dat`, `KDXServer.stg`, `Bases/`, news, `History/`, `News Archives/`) and it serves it unchanged. The client imports an existing `KDX.stg` and uses existing `Appearances/` and `Sound Lists/` folders.
6. **Headless server** that runs under systemd, launchd, Docker, or a terminal, with the original's log line format on stdout.
7. **Open source**, MIT OR Apache-2.0, on GitHub, with prebuilt releases for macOS (arm64, x86_64), Windows (x86_64), Linux (x86_64, arm64), and a Docker image for the server.

## 3. Non-goals

- A new or "improved" protocol. We speak KDX 1.x exactly as the originals do. Protocol extensions, if ever, are a separate project after 1.0.
- Hotline or Wired compatibility, or any bridge beyond the IRC gateway the original server already had.
- Mobile or web clients.
- Running or wrapping the original binaries in production. The lab uses them for testing only.
- Reproducing Haxial's licensing/registration nag behaviour.

## 4. Anti-goals (what would make this a failure even if it works)

- Shipping anything of Haxial's: binaries, the challenge-table stories, `.hap` artwork, icons, the original logo. The repository must be clean-room with respect to Haxial's copyrighted material.
- A client that "looks modern" but loses the texture of the original: the bevels, the black panes, the user bars, the WonderLight, the sounds.
- A server that silently rewrites an operator's old data so they cannot roll back to `KDXServer.lexe`.
- A codebase only one person can understand or build.

## 5. Constraints

### 5.1 Protocol and crypto are fixed by compatibility

- Handshake: cleartext HELLO/PROOF with the `0x4878` XOR stream; proof = Twofish-256(account_key) over SHA-1 of a key/nonce interleave; account_key = SHA-1(password) stretched to 32 bytes with an FNV-1 tail, or a fixed constant for guest.
- Session: 16-byte header (txid, type, flags, u48 length), payload padded to 16, Twofish-256 **ECB** with the account key.
- Challenge tables: a 1739-byte "KDX story" (all programs) and a 3193-byte "GConsole story" (clients only). Both are Haxial's copyrighted prose. They must be **fetched or imported at runtime** (from an archive on the Internet Archive or a user-supplied Haxial binary/zip), cached under the user's data directory, and never committed. Tests read them from an environment variable or the cache.
- Ports: 10700 TCP server (plus optional alternate and file-transfer ports), 10800 UDP/TCP tracker, 10600 TCP DCC, 10900 GConsole.
- Reference: the protocol spec at https://github.com/scottanderson/kdx/tree/main/docs/protocol (MIT/Apache-2.0), which documents the handshake, framing, every message type 1–1107 and 4000–4201, encodings, privileges, transfers, tracker, DCC, and GConsole, with evidence tags. **This spec is authoritative unless our own captures or disassembly contradict it**, in which case we document the discrepancy and, where appropriate, contribute it upstream.

Known gaps in that spec, to be closed by our own reverse engineering: voice data (1003) is documented statically only; the user icon bitmap format and the **server icon (banner, ≤152×40) format and carrying message** are not pinned down; the view-display pixel format is only captured from a Windows server.

### 5.2 On-disk formats are fixed by goal 5

Not documented anywhere; to be reverse-engineered from the original binaries and from the user's real data directories:

| Side | File / folder | Contents |
|---|---|---|
| Server | `Accounts.dat` | accounts, classes, privilege bitmaps, 32-byte derived keys |
| Server | `KDXServer.stg` | settings: name, description, address, ports, greeting, server icon, speed limit, allow/deny list with expiries, tracker registrations, history logging |
| Server | `Bases/<name>/…` | shared files; `[UL]`, `[DB]`, `[<item>]` folder-name suffixes bind Folder Access items; drop-box owner |
| Server | (newsgroup store) | live newsgroups, messages, per-group access list and archive size |
| Server | `News Archives/` | archived news as text |
| Server | `History/` | tab-separated history logs |
| Server | catalog | generated file catalog used by Search |
| Client | `KDX.stg` | identity (name, description, fg/bg colour, 32×32 icon), address book, settings, window positions, registration |
| Client | `Appearances/*.hap` | appearance (theme) files |
| Client | `Sound Lists/*` | KDXSoundListEdit output |

Rule (our call): the original files **are the live store**. The server reads them at startup and, on every change, re-serialises them **in the original format**, so `KDXServer.lexe` can be pointed back at the directory at any time and see every account, setting, and news message created under OpenKDX. On first start the server copies each original file once to `<name>.orig` as an untouched backup. Files OpenKDX adds for its own needs (for example a faster index) live alongside and are ignored by the original. This makes `openkdx-legacy` a full reader *and* writer for `Accounts.dat`, `KDXServer.stg`, and the news store; `History/` and `News Archives/` are append-only in the original format.

### 5.3 Legal boundary

- No Haxial binaries, stories, artwork, or icons in the repository, ever. CI fails if a known Haxial SHA-256 appears in the tree.
- Ghidra projects and decompiled output stay local; only our notes, scripts, and address landmarks are committed.
- The OpenKDX mark is an original design in the KDX visual vocabulary (glossy X, chrome upper half, coloured lower half; **green = server, gold = client, purple = tracker**; open steel ring at the centre, bevelled single-notch tips). It is not a trace of Haxial's mark. The README says so.
- "KDX" is used descriptively to indicate protocol compatibility. The project name is OpenKDX; binaries are `openkdx`, `openkdxd`, `openkdx-tracker`.

### 5.4 Platforms and toolchain

- Rust, stable, 2024 edition; one Cargo workspace.
- Async networking on tokio.
- GUI: **egui/eframe** with multi-viewport for OS-level windows (decided; see §6).
- Audio (voice): `cpal` for capture/playback; codec per what the binaries reveal.
- Crypto: `twofish`, `sha1`, from RustCrypto; no OpenSSL dependency.
- Supported OS: macOS 12+, Windows 10+, Linux (glibc, X11/Wayland).

## 6. Approaches considered

### 6.1 Starting point

| Option | Trade-off | Decision |
|---|---|---|
| Build on scottanderson/kdx | Fastest; inherits a CLI/dissector-centred design and a solo maintainer's pace; no server, no GUI | Not chosen |
| **Own project, their spec as reference** | Our architecture and scope; still avoids re-deriving the protocol; same licence so cross-pollination is easy | **Chosen** |
| Reverse-engineer from scratch | Only justified by distrust of the existing work, which is well evidenced | Not chosen |

### 6.2 GUI toolkit

| Option | Classic fidelity | OS-level windows | Modern polish | Memory safety | Risk |
|---|---|---|---|---|---|
| **egui / eframe** | Easy: painter API draws bevels, bitmaps, pixel fonts directly; immediate mode suits a hand-drawn toolkit | Yes (multi-viewport) | Good, utilitarian | All Rust | Weaker text layout and accessibility; big lists need virtualisation |
| iced | Possible via custom widgets, more ceremony | Yes | Strongest | All Rust | Younger API, breaking changes |
| Tauri + web UI | Easy in CSS, but webview rendering differs per OS | Yes | Best | Rust core, JS UI | Breaks pixel-faithful promise; undermines all-Rust goal |

**Decision: egui.** Haxial's toolkit was itself a hand-drawn, bitmap-heavy UI; egui is the closest Rust analogue. iced is the fallback if Modern mode later needs more than egui gives.

### 6.3 Server shape

| Option | Decision |
|---|---|
| Headless daemon, original log format, administered from the client | **Chosen** (the original's "GUI" was a log window) |
| Daemon plus its own GUI | Not chosen; nothing the log and the client's Connection Monitor / Server History don't give |

### 6.4 Client display modes

| Option | Decision |
|---|---|
| Classic only (OS windows) | Insufficient: modern users expect a single window |
| Modern only (tabs) | Insufficient: loses the fidelity the community wants |
| **Both, one renderer, two "window hosts"** | **Chosen** |

## 7. Architecture

### 7.1 Workspace

```
openkdx/
  crates/
    openkdx-proto      wire format: handshake, XOR stream, Twofish framing, header,
                       typed payloads for every message, encodings, privileges. No I/O.
    openkdx-codec      tokio-util codecs for handshake and session framing.
    openkdx-legacy     Haxial on-disk formats: Accounts.dat, .stg (server and client),
                       .hap, sound lists, news store, history, catalog. Read and write.
    openkdx-table      challenge tables: fetch, import, cache, verify by SHA-256.
    openkdx-client     async client library: Connection, Session, transfers, DCC, tracker browse.
    openkdx-server     server library: sessions, accounts, files, news, chat, admin, trackers, IRC gateway.
    openkdx-tracker    tracker library and binary.
    openkdx-ui         the Haxial-look widget set on egui: window chrome, bevels, panes,
                       lists, user bars, WonderLight, scrollbars, theme engine (.hap).
    openkdx-app        the GUI client binary: Classic and Modern hosts over openkdx-ui.
    openkdxd           the server binary (CLI, config, logging, service integration).
    openkdx-dissect    pcap/pcapng decoder for captures (lab tool).
  lab/                 sandboxed runner for the original binaries + capture scripts
  re/                  Ghidra headless scripts, address landmarks (no decompiled output)
  docs/
    superpowers/specs/ this document and successors
    protocol/          our deltas to the reference spec; file format specs
    formats/           Accounts.dat, .stg, .hap, sound list, news store, history, catalog
  ci/                  lint, docs checks, Haxial-material blocklist
```

### 7.2 Client UI model

One widget tree per *function* (Button Bar, Connect, Address Book, Messages, File Transfers, Files, News, Chat, User List, Commands, Settings, Accounts, Account, Folder Access, Server Settings, Server History, Connection Monitor, Process Monitor, View Display, Voice Controls, Voice Info, User Info, Invite, Disconnect User, Create Folder, Shutdown Server, Broadcast, Get Info, Search, Tracker browse). Each function renders into a *window host*:

- **Classic host:** every function is an egui viewport, i.e. a real OS window, drawn edge-to-edge with Haxial chrome (X at left, striped grip = window menu, centred title, `+` zoom and `–` minimise at right). Minimise sends the window to the **KDX Dock**, a KDX-drawn strip; restore returns it to its saved position. Window Switcher (F1 / ⌘⇧W). Small Button Bar mode. Context menus everywhere, as the original. On macOS a native menu bar with File/Edit/Window.
- **Modern host:** one viewport; the Button Bar is a fixed left panel; each function opens as a tab in the main area; the KDX Dock is the strip under the Button Bar where minimised tabs park. Same chrome, same panes, same theme.

The user toggles the host in Settings → Appearance; the choice persists; switching does not lose state.

Multi-server: Chat, Files, News, and User List show all connections in one window (tabs or a server tree), exactly as the original. The Button Bar banner shows the server icon of the connection whose window last had focus.

### 7.3 Theme engine

`.hap` files are parsed by `openkdx-legacy` into a `Theme`: palette (chrome, panes, highlight frame, title bar, text colours), fonts, and bitmaps (banner default, button faces, window background, icons). `openkdx-ui` draws from a `Theme`. Modern mode uses the same `Theme`. Anything in a `.hap` the renderer cannot honour is listed in `docs/formats/hap.md` under "Not supported". A built-in default theme reproduces the stock Haxial grey-and-red look from original drawings (ours), not from extracted bitmaps.

### 7.4 Server

Single process, tokio. Listener per configured port. Session actor per connection; shared state for users, chats, news, accounts, folder access, bans, trackers. File transfers on secondary connections attached by session ID (login name `\0` + ID). Catalog generation and search. IRC gateway per chat room. UDP registration with trackers. Log lines in the original format to stdout; optional `History/` logs in the original tab-separated format. First-run interactive setup identical in wording to the original.

### 7.5 Lab and verification

`lab/` runs Server 1.620 and Tracker 1.110 (Linux x86 builds) and Client 1.600 (Wine) or Client 1.620 beta 5 (Linux) in a bwrap sandbox on **a Linux machine the user controls, which also serves as a self-hosted GitHub Actions runner**. The Haxial binaries and challenge tables live only on that machine (fetched once from preterhuman.net / the Internet Archive, verified by SHA-256); CI jobs that need them are labelled to run there, so differential tests gate every pull request. Hosted runners run everything that needs no Haxial material. The lab captures loopback traffic and stores `captures/<scenario>.pcapng` with a `.md` describing the steps. `openkdx-dissect` decodes captures. Every protocol claim we add is tagged with evidence (capture + frame, or binary + address).

Differential tests, run in CI where the lab can run and locally otherwise:
- OpenKDX client ⇄ original server: login (admin, guest), chat, PM, news, files (list, get, put, resume), accounts, tracker browse.
- Original client ⇄ OpenKDX server: the same, driven by scripted GUI steps or recorded captures replayed.
- Original server/client ⇄ OpenKDX tracker.
- Legacy formats: read an original `Accounts.dat`/`.stg`/news store, write it back, byte-compare; add an account with the original server, read it with ours.

## 8. Phases

Each phase is planned in detail (superpowers:writing-plans) when it starts. Later phases may reorder.

0. **Repository and lab.** Workspace skeleton, CI, licence files, NOTICE, Haxial blocklist. Lab running with the user's binaries; first captures. Challenge-table fetch/import.
1. **Protocol core.** `openkdx-proto`, `openkdx-codec`, `openkdx-table`, `openkdx-dissect`. Decode every capture byte-exact; re-encode byte-exact.
2. **Client library + CLI.** `openkdx-client` and a thin `openkdx` CLI for login, chat, PM, news, files, accounts, tracker. Passes differential tests against the original server.
3. **Legacy formats.** `openkdx-legacy`: `Accounts.dat`, `KDXServer.stg`, news store, history, catalog, `KDX.stg`, `.hap`, sound lists. Documented in `docs/formats/`. Round-trip tests on the user's data.
4. **Server.** `openkdx-server` + `openkdxd` serving an old data directory. Original client passes differential tests against it. Docker image, systemd/launchd units.
5. **Tracker.** `openkdx-tracker`. Original server registers; original client browses.
6. **UI toolkit.** `openkdx-ui`: Haxial chrome and widgets on egui; theme engine; default theme; `.hap` loading.
7. **Client, Classic mode.** `openkdx-app` with every window, KDX Dock, Window Switcher, Small Button Bar, sounds, address book, transfers, multi-server.
8. **Client, Modern mode.** Tabbed host over the same functions.
9. **Admin features.** Accounts, classes, folder access, server settings (incl. server icon), history, connection monitor, process monitor, broadcast, disconnect/ban, shutdown, catalog, IRC gateway.
10. **DCC, voice, view display, GConsole.** Reverse-engineer what the reference spec lacks; implement. GConsole needs the second challenge table (clients only) and the `%HGC` message set; verified against a Haxial GConsole binary in the lab if one can be obtained, else against captures.
11. **Release 1.0.** Signed/notarised builds, Docker, docs, migration guide for `.lexe` operators.

**Phase 0 complete:** 2026-11-09, branch `phase0/repo-and-lab` (PR #1). Repository, CI (hosted ×4 + self-hosted lab), `openkdx-table`, lab runner, seven captures, fixtures, docs.

**Phase 2 complete:** 2026-11-13, branch `phase2/client-cli` (PR #3). `openkdx-client` (sessions, chat, PM, news, files and transfers with resume, accounts/classes/folder access, administration, tracker browse) and the `openkdx` CLI; sixteen differential tests against Server 1.620 and Tracker 1.110 in the lab CI; eight new captures (upload with resume, accounts and classes, folder access, server settings, broadcast, disconnect/ban, server history, connection monitor), all byte-exact; nine more deltas and confirmations in `docs/protocol/README.md`.

**Phase 3 complete:** 2026-11-13, branch `phase3/legacy-formats` (PR #4). `openkdx-legacy` reads and writes `Accounts.dat`, `KDXServer.stg`, `News.dat`, `History/`, `News Archives/`, `Catalog.dat`, `KDXTrackerServ.stg` and `KDX.stg` byte for byte (56 lab-written fixtures), with the §5.2 live store (`.orig` once, atomic rewrites); the scrambling of the three obfuscated files is an LCG XOR stream per program; `lab/legacy.sh` restarts Server 1.620 and Client 1.600 on files OpenKDX wrote on every push. Documented in `docs/formats/`. `.hap` followed in PR #5 (byte-exact over the 111 files of Haxial's appearance pack, lab-only). Still open: sound lists (no source file; the editor's save dialog lists no volumes under Wine) and round trips over the maintainer's real early-2000s directories, hence the formats' status "verified (lab)".

**Phase 1 complete:** 2026-11-10, branch `phase1/protocol-core` (PR #2). `openkdx-proto` (handshake, session framing, UDP registration, every message type 1–1107 and 4000–4201 typed), `openkdx-codec`, `openkdx-dissect`; all seven captures decode and re-encode byte-exact, with four deltas to the reference spec recorded in `docs/protocol/README.md`. MSRV 1.88; fetching feature-gated in `openkdx-table`.

## 9. Decisions already taken (by the user unless marked "our call")

- New project; scottanderson/kdx spec is the reference.
- Rust throughout; egui for the GUI.
- Both client and server (and tracker — our call) in scope; "complete" means feature parity.
- Classic mode uses real OS windows and a KDX Dock; fidelity matters.
- Modern mode keeps the identical look, unified window, tabs.
- Theme files (`.hap`) load with documented limitations; Modern mode uses the same theme.
- Server banner is pushed by the server and follows the focused connection.
- Server is headless; log format preserved; administered from the client.
- Old server directories are served in place; originals are never overwritten.
- MIT OR Apache-2.0; project name OpenKDX; crate prefix `openkdx-`.
- Logo: original design in the KDX vocabulary, fused X, chrome over colour; green server, gold client, purple tracker. Lockup: mark at left; "OPEN" (small, letterspaced, role colour) above a chrome "KDX", with the role word (`Client`, `Server`, `Tracker`; monospaced, role colour) beneath, all left-aligned.
- Our call: `openkdx-legacy` as one shared crate for all Haxial file formats, used by client and server.
- Our call: tracker ships with 1.0 because the original tracker is also a dead 32-bit binary.
- Our call: voice and view display come last and must not block earlier releases.
- Our call: the original files are the live store and are rewritten in the original format (§5.2), with one-time `.orig` backups.
- Our call: the OpenKDX client presents itself on the wire exactly as a **registered Client 1.600** (version bytes, 118 report), so original servers never send licence reminders. Nothing in the handshake depends on Haxial's registration data.
- Our call: on Linux, Classic mode's window placement and KDX Dock restore require X11 or XWayland, because native Wayland does not let applications position top-level windows. Under native Wayland the client still runs, but documents that positions are not restored and defaults to Modern mode.
- User: the lab and differential-test runner live on a Linux machine the user controls (§7.5).
- User: IRC gateway and GConsole (client side) are in scope for 1.0.
- User: the tracker is a first-class deliverable with its own purple logo variant.

## 10. Inputs the user provides

- Original binaries, from https://preterhuman.net/gethotlinekdx.php and https://kdx.technowiki.info/downloads.php (the latter also hosts unlocked builds and the two editors below): Client 1.600 Win/Mac, Client 1.620 beta 5 Linux, Server 1.600 Win/Mac (GUI and CLI), Server 1.620 Linux, Tracker 1.110 all platforms.
- **Appearance Editor 1.240** (Win/Mac) and **KDXSoundListEdit 1.000** (Win/Mac): the authoring tools for `.hap` and sound-list files. Used in Phase 3 to generate controlled samples (change one field, diff the output) and as Ghidra targets for the format readers/writers. Both are Haxial material and stay on the lab machine.
- A real server data directory from an early-2000s deployment: `Accounts.dat`, `KDXServer.stg`, `Bases/`, news, `History/`, `News Archives/`, with a server icon set.
- A real client directory: `KDX.stg`, several `.hap` files, sound lists.
- Screenshots of every original window for pixel reference (several already collected).

## 11. Left to the builder

- Exact internal layout of `openkdx-proto` types, error handling, and the actor model in the server.
- The newsgroup store, catalog, `.hap`, and sound-list formats: whatever the bytes say.
- Virtualised list rendering strategy in egui for large file lists and chat transcripts.
- How the Classic host persists window positions per display configuration.
- Packaging: cargo-dist or hand-rolled; code signing and notarisation setup.
- Whether voice uses the original codec (once identified) or needs a shim.

## 12. Open questions for the user

None. The builder check raised five; two were answered by the user (lab location, IRC/GConsole scope) and three were settled as our calls (live-store semantics, client identity on the wire, Wayland limitation), all recorded in §9. The user then added GConsole back and made the tracker a first-class deliverable.
