#!/usr/bin/env python3
"""Check crate manifests against the design's dependency rules.

- openkdx-proto must not depend on any async runtime, network, or UI crate.
- openkdx-legacy must not depend on any network or UI crate.
- every crate declares license = "MIT OR Apache-2.0" and edition = "2024"
  (directly or via workspace inheritance).

Exit 1 with the offending crate on failure.
"""
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FORBIDDEN = {
    "openkdx-proto": {"tokio", "reqwest", "egui", "eframe", "hyper", "async-std"},
    "openkdx-legacy": {"tokio", "reqwest", "egui", "eframe", "hyper", "async-std"},
}


def crate_manifests():
    crates = ROOT / "crates"
    if not crates.is_dir():
        return []
    return sorted(p / "Cargo.toml" for p in crates.iterdir() if (p / "Cargo.toml").is_file())


def check(manifest: Path) -> list[str]:
    data = tomllib.loads(manifest.read_text())
    pkg = data.get("package", {})
    name = pkg.get("name", manifest.parent.name)
    problems = []

    def inherited_or(value, key):
        if isinstance(value, dict) and value.get("workspace") is True:
            ws = tomllib.loads((ROOT / "Cargo.toml").read_text())
            return ws.get("workspace", {}).get("package", {}).get(key)
        return value

    if inherited_or(pkg.get("license"), "license") != "MIT OR Apache-2.0":
        problems.append(f"{name}: license is not 'MIT OR Apache-2.0'")
    if inherited_or(pkg.get("edition"), "edition") != "2024":
        problems.append(f"{name}: edition is not 2024")

    deps = set(data.get("dependencies", {})) | set(data.get("build-dependencies", {}))
    for bad in sorted(deps & FORBIDDEN.get(name, set())):
        problems.append(f"{name}: must not depend on {bad}")
    return problems


def main() -> int:
    manifests = crate_manifests()
    if not manifests:
        print("check-deps: no crates found under crates/", file=sys.stderr)
        return 1
    problems = [p for m in manifests for p in check(m)]
    for p in problems:
        print(f"check-deps: {p}", file=sys.stderr)
    if problems:
        return 1
    print(f"check-deps: ok ({len(manifests)} crates)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
