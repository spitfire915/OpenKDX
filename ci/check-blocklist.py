#!/usr/bin/env python3
"""Fail if any tracked file is, or contains, known Haxial material.

Reads ci/haxial-blocklist.txt (see its header for the format). Every file in
`git ls-files` is hashed whole and compared; entries that carry an anchor are
also searched for inside each file, and `length` bytes from each anchor hit
are hashed, so a story pasted into a larger file under any name is caught.
Base64 encodings of a story are caught too (all three alignments).

With `--range BASE HEAD`, every blob introduced by any commit in that range is
scanned as well, so a story added and removed within a pull request cannot
reach the history of main.
"""
import base64
import hashlib
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_BLOCKLIST = ROOT / "ci" / "haxial-blocklist.txt"


@dataclass(frozen=True)
class Entry:
    sha256: str
    length: int | None
    anchor: bytes | None
    label: str


def load_blocklist(path: Path) -> list[Entry]:
    entries = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        sha, length, anchor, label = line.split(None, 3)
        entries.append(
            Entry(
                sha256=sha.lower(),
                length=None if length == "-" else int(length),
                anchor=None if anchor == "-" else bytes.fromhex(anchor),
                label=label,
            )
        )
    return entries


def tracked_files(repo_root: Path) -> list[Path]:
    out = subprocess.run(
        ["git", "-C", str(repo_root), "ls-files", "-z"],
        check=True,
        capture_output=True,
    ).stdout
    return [Path(p) for p in out.decode().split("\0") if p]


def _b64_variants(anchor: bytes) -> list[bytes]:
    """Base64 of the anchor at each of the three input alignments, trimmed of
    the partial first and last groups so the text matches wherever it falls."""
    out = []
    for shift in range(3):
        enc = base64.b64encode(b"\x00" * shift + anchor)
        head = 4 if shift else 0
        out.append(enc[head : len(enc) - 4])
    return out


def _find_story(data: bytes, e: Entry) -> bool:
    start = 0
    while (i := data.find(e.anchor, start)) != -1:
        if hashlib.sha256(data[i : i + e.length]).hexdigest() == e.sha256:
            return True
        start = i + 1
    return False


def _find_story_b64(data: bytes, e: Entry) -> bool:
    for frag in _b64_variants(e.anchor):
        start = 0
        while (i := data.find(frag, start)) != -1:
            # Walk back to a 4-char boundary of the base64 run and decode enough.
            run_start = i
            while run_start > 0 and data[run_start - 1 : run_start] in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=":
                run_start -= 1
            run_end = i
            while run_end < len(data) and data[run_end : run_end + 1] in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=":
                run_end += 1
            run = data[run_start:run_end]
            try:
                decoded = base64.b64decode(run + b"=" * (-len(run) % 4), validate=False)
            except Exception:
                decoded = b""
            if decoded and _find_story(decoded, e):
                return True
            start = i + 1
    return False


def scan_file(data: bytes, entries: list[Entry]) -> str | None:
    whole = hashlib.sha256(data).hexdigest()
    for e in entries:
        if whole == e.sha256:
            return e.label
    for e in entries:
        if e.anchor is None or e.length is None:
            continue
        if _find_story(data, e) or _find_story_b64(data, e):
            return e.label
    return None


def check_blocklist(repo_root: Path, blocklist: Path = DEFAULT_BLOCKLIST) -> list[tuple[Path, str]]:
    entries = load_blocklist(blocklist)
    hits = []
    for rel in tracked_files(repo_root):
        full = repo_root / rel
        if not full.is_file():
            continue
        label = scan_file(full.read_bytes(), entries)
        if label is not None:
            hits.append((rel, label))
    return hits


def check_range(repo_root: Path, base: str, head: str, blocklist: Path = DEFAULT_BLOCKLIST) -> list[tuple[str, str]]:
    """Scan every blob introduced by commits in base..head. Returns (sha, label)."""
    entries = load_blocklist(blocklist)
    out = subprocess.run(
        ["git", "-C", str(repo_root), "rev-list", "--objects", f"{base}..{head}"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    hits = []
    seen = set()
    for line in out.splitlines():
        sha = line.split(" ", 1)[0]
        if sha in seen:
            continue
        seen.add(sha)
        kind = subprocess.run(["git", "-C", str(repo_root), "cat-file", "-t", sha], capture_output=True, text=True).stdout.strip()
        if kind != "blob":
            continue
        data = subprocess.run(["git", "-C", str(repo_root), "cat-file", "blob", sha], capture_output=True).stdout
        label = scan_file(data, entries)
        if label is not None:
            hits.append((sha, label))
    return hits


def main() -> int:
    args = sys.argv[1:]
    hits = [(str(p), l) for p, l in check_blocklist(ROOT)]
    if len(args) == 3 and args[0] == "--range":
        hits += [(f"blob {sha} in {args[1]}..{args[2]}", l) for sha, l in check_range(ROOT, args[1], args[2])]
    for what, label in hits:
        print(f"check-blocklist: {what}: matches blocklisted {label}", file=sys.stderr)
    if hits:
        return 1
    print("check-blocklist: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
