#!/usr/bin/env python3
"""Set a GitHub Actions repository secret from stdin.

    base64 -w0 ~/haxial/tables/kdx.bin | python3 -I lab/gh-secret.py spitfire915/kdx KDX_CHALLENGE_TABLE

Runs on the lab machine so the value never leaves it. The token is `GITHUB_TOKEN` or, failing
that, `git credential fill` (the maintainer's fine-grained PAT). Needs `pynacl`.
"""
import argparse
import base64
import json
import os
import subprocess
import sys
import urllib.request

API = "https://api.github.com"


def parse_args(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("repo", help="owner/name")
    parser.add_argument("name", help="secret name")
    return parser.parse_args(argv)


def seal(public_key_b64: str, value: bytes) -> str:
    """Encrypts `value` for the repository public key as GitHub expects (libsodium sealed box)."""
    from nacl.public import PublicKey, SealedBox

    box = SealedBox(PublicKey(base64.b64decode(public_key_b64)))
    return base64.b64encode(box.encrypt(value)).decode()


def token() -> str:
    env = os.environ.get("GITHUB_TOKEN")
    if env:
        return env
    out = subprocess.run(
        ["git", "credential", "fill"],
        input="protocol=https\nhost=github.com\n",
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    for line in out.splitlines():
        if line.startswith("password="):
            return line[len("password="):]
    raise SystemExit("gh-secret: no GitHub credential")


def request(method: str, path: str, tok: str, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(API + path, data=data, method=method)
    req.add_header("Authorization", f"Bearer {tok}")
    req.add_header("Accept", "application/vnd.github+json")
    with urllib.request.urlopen(req) as resp:
        raw = resp.read()
        return resp.status, (json.loads(raw) if raw else None)


def main(argv=None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    value = sys.stdin.buffer.read().strip()
    if not value:
        raise SystemExit("gh-secret: empty value on stdin")
    tok = token()
    _, key = request("GET", f"/repos/{args.repo}/actions/secrets/public-key", tok)
    status, _ = request(
        "PUT",
        f"/repos/{args.repo}/actions/secrets/{args.name}",
        tok,
        {"encrypted_value": seal(key["key"], value), "key_id": key["key_id"]},
    )
    print(f"gh-secret: {args.repo} {args.name}: {'created' if status == 201 else 'updated'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
