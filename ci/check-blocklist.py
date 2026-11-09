#!/usr/bin/env python3
"""Fail if any tracked file is, or contains, known Haxial material.

Reads ci/haxial-blocklist.txt (see its header for the format). Every file in
`git ls-files` is hashed whole and compared; entries that carry an anchor are
also searched for inside each file, and `length` bytes from each anchor hit
are hashed, so a story pasted into a larger file under any name is caught.
"""
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


def scan_file(data: bytes, entries: list[Entry]) -> str | None:
    whole = hashlib.sha256(data).hexdigest()
    for e in entries:
        if whole == e.sha256:
            return e.label
    for e in entries:
        if e.anchor is None or e.length is None:
            continue
        start = 0
        while (i := data.find(e.anchor, start)) != -1:
            if hashlib.sha256(data[i : i + e.length]).hexdigest() == e.sha256:
                return e.label
            start = i + 1
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


def main() -> int:
    hits = check_blocklist(ROOT)
    for path, label in hits:
        print(f"check-blocklist: {path}: matches blocklisted {label}", file=sys.stderr)
    if hits:
        return 1
    print("check-blocklist: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
