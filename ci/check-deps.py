#!/usr/bin/env python3
"""Check the workspace against the design's dependency rules (spec §7.1).

- openkdx-proto must not depend, directly or transitively, on any async
  runtime, network, or UI crate.
- openkdx-legacy must not depend, directly or transitively, on any network or
  UI crate.
- every crate declares license = "MIT OR Apache-2.0" and edition = "2024"
  (directly or via workspace inheritance).

Transitive edges come from `cargo metadata`, normal and build dependencies
only (dev-dependencies do not ship). Exit 1 with the offending crate on failure.
"""
import json
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

NETWORK_OR_RUNTIME = {"tokio", "reqwest", "hyper", "async-std", "smol", "mio", "openkdx-codec", "openkdx-table", "openkdx-client", "openkdx-server", "openkdx-tracker"}
UI = {"egui", "eframe", "egui-winit", "winit", "wgpu", "openkdx-ui", "openkdx-app"}
FORBIDDEN = {
    "openkdx-proto": NETWORK_OR_RUNTIME | UI,
    "openkdx-legacy": NETWORK_OR_RUNTIME | UI,
}


def crate_manifests():
    crates = ROOT / "crates"
    if not crates.is_dir():
        return []
    return sorted(p / "Cargo.toml" for p in crates.iterdir() if (p / "Cargo.toml").is_file())


def check_manifest(manifest: Path) -> list[str]:
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
    return problems


def forbidden_transitive(metadata: dict, crate: str, forbidden: set[str]) -> list[str]:
    """Names of forbidden packages reachable from `crate` over normal/build edges."""
    name_of = {p["id"]: p["name"] for p in metadata["packages"]}
    deps_of = {}
    for node in metadata["resolve"]["nodes"]:
        edges = []
        for d in node.get("deps", []):
            kinds = d.get("dep_kinds") or [{"kind": None}]
            if any(k.get("kind") in (None, "build") for k in kinds):
                edges.append(d["pkg"])
        deps_of[node["id"]] = edges
    roots = [i for i, n in name_of.items() if n == crate]
    seen, stack, found = set(), list(roots), set()
    while stack:
        cur = stack.pop()
        for dep in deps_of.get(cur, []):
            if dep in seen:
                continue
            seen.add(dep)
            if name_of.get(dep) in forbidden:
                found.add(name_of[dep])
            stack.append(dep)
    return sorted(found)


def cargo_metadata() -> dict:
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return json.loads(out)


# Crates whose optional dependency must vanish without default features.
FEATURE_GATED = {"openkdx-table": "reqwest"}


def cargo_tree_no_default(crate: str) -> str:
    return subprocess.run(
        ["cargo", "tree", "-p", crate, "--no-default-features", "-e", "normal", "--prefix", "none", "--locked"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def check_feature_gate(crate: str, gated: str, tree: str) -> list[str]:
    """`tree` is `cargo tree --prefix none` output: one `name vX.Y.Z` per line."""
    names = {line.split()[0] for line in tree.splitlines() if line.strip()}
    if gated in names:
        return [f"{crate}: without default features still depends on {gated}"]
    return []


def main() -> int:
    manifests = crate_manifests()
    if not manifests:
        print("check-deps: no crates found under crates/", file=sys.stderr)
        return 1
    problems = [p for m in manifests for p in check_manifest(m)]
    meta = cargo_metadata()
    for crate, forbidden in FORBIDDEN.items():
        for bad in forbidden_transitive(meta, crate, forbidden):
            problems.append(f"{crate}: must not depend (even transitively) on {bad}")
    for crate, gated in FEATURE_GATED.items():
        problems.extend(check_feature_gate(crate, gated, cargo_tree_no_default(crate)))
    for p in problems:
        print(f"check-deps: {p}", file=sys.stderr)
    if problems:
        return 1
    print(f"check-deps: ok ({len(manifests)} crates, transitive graph checked)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
