"""Tests for lab/gh-secret.py: the sealed box round-trips against a generated keypair."""
import base64
import importlib.util
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("gh_secret", HERE.parent / "lab" / "gh-secret.py")
gh_secret = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gh_secret)

try:
    from nacl.public import PrivateKey, SealedBox
except ImportError:  # pragma: no cover
    PrivateKey = None


@unittest.skipIf(PrivateKey is None, "pynacl not installed")
class SealTest(unittest.TestCase):
    def test_seal_roundtrip(self):
        private = PrivateKey.generate()
        public_b64 = base64.b64encode(bytes(private.public_key)).decode()
        sealed = gh_secret.seal(public_b64, b"hello table")
        plain = SealedBox(private).decrypt(base64.b64decode(sealed))
        self.assertEqual(plain, b"hello table")

    def test_seal_output_is_base64_text(self):
        private = PrivateKey.generate()
        public_b64 = base64.b64encode(bytes(private.public_key)).decode()
        sealed = gh_secret.seal(public_b64, b"x")
        base64.b64decode(sealed, validate=True)
        self.assertIsInstance(sealed, str)


class ArgsTest(unittest.TestCase):
    def test_repo_and_name_are_required(self):
        with self.assertRaises(SystemExit):
            gh_secret.parse_args(["spitfire915/kdx"])

    def test_parse_args(self):
        args = gh_secret.parse_args(["spitfire915/kdx", "KDX_CHALLENGE_TABLE"])
        self.assertEqual((args.repo, args.name), ("spitfire915/kdx", "KDX_CHALLENGE_TABLE"))


if __name__ == "__main__":
    unittest.main()
