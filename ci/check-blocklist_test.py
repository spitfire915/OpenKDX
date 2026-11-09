import hashlib
import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("check_blocklist", HERE / "check-blocklist.py")
check_blocklist = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_blocklist)


def git(repo, *args):
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def make_repo(files: dict[str, bytes]) -> Path:
    repo = Path(tempfile.mkdtemp())
    git(repo, "init", "-q")
    git(repo, "config", "user.email", "t@example.com")
    git(repo, "config", "user.name", "t")
    for name, data in files.items():
        p = repo / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(data)
    git(repo, "add", "-A")
    git(repo, "commit", "-qm", "x")
    return repo


STORY = b"Once upon a lab there was a fake story " * 4  # 156 bytes


def blocklist_for(data: bytes, label: str) -> Path:
    sha = hashlib.sha256(data).hexdigest()
    anchor = data[:32].hex()
    bl = Path(tempfile.mkdtemp()) / "bl.txt"
    bl.write_text(f"# test\n{sha}  {len(data)}  {anchor}  {label}\n")
    return bl


class BlocklistTest(unittest.TestCase):
    def test_whole_file_match_is_reported(self):
        repo = make_repo({"a/b.bin": STORY})
        hits = check_blocklist.check_blocklist(repo, blocklist_for(STORY, "fake"))
        self.assertEqual([(Path("a/b.bin"), "fake")], hits)

    def test_clean_repo_has_no_hits(self):
        repo = make_repo({"ok.txt": b"nothing to see"})
        hits = check_blocklist.check_blocklist(repo, blocklist_for(STORY, "fake"))
        self.assertEqual([], hits)

    def test_embedded_match_is_reported(self):
        repo = make_repo({"renamed.dat": os.urandom(100) + STORY + os.urandom(50)})
        hits = check_blocklist.check_blocklist(repo, blocklist_for(STORY, "fake"))
        self.assertEqual([(Path("renamed.dat"), "fake")], hits)


if __name__ == "__main__":
    unittest.main()
