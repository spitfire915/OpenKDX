import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("check_deps", Path(__file__).resolve().parent / "check-deps.py")
check_deps = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_deps)


def meta(nodes):
    """nodes: {pkg_id: [dep_ids]} → a minimal cargo-metadata-shaped dict."""
    return {
        "packages": [{"id": i, "name": i.split("#")[0]} for i in nodes],
        "resolve": {"nodes": [{"id": i, "deps": [{"pkg": d, "dep_kinds": [{"kind": None}]} for d in ds]} for i, ds in nodes.items()]},
    }


class TransitiveTest(unittest.TestCase):
    def test_direct_forbidden_dep_is_found(self):
        m = meta({"openkdx-proto#1": ["tokio#1"], "tokio#1": []})
        self.assertEqual(["tokio"], check_deps.forbidden_transitive(m, "openkdx-proto", {"tokio"}))

    def test_forbidden_dep_through_another_crate_is_found(self):
        m = meta({"openkdx-proto#1": ["openkdx-codec#1"], "openkdx-codec#1": ["tokio#1"], "tokio#1": []})
        self.assertEqual(["tokio"], check_deps.forbidden_transitive(m, "openkdx-proto", {"tokio"}))

    def test_dev_only_edges_are_ignored(self):
        m = meta({"openkdx-proto#1": [], "tokio#1": []})
        m["resolve"]["nodes"][0]["deps"] = [{"pkg": "tokio#1", "dep_kinds": [{"kind": "dev"}]}]
        self.assertEqual([], check_deps.forbidden_transitive(m, "openkdx-proto", {"tokio"}))

    def test_clean_graph_passes(self):
        m = meta({"openkdx-proto#1": ["sha2#1"], "sha2#1": []})
        self.assertEqual([], check_deps.forbidden_transitive(m, "openkdx-proto", {"tokio", "egui"}))


class FeatureGateTest(unittest.TestCase):
    def test_reqwest_in_no_default_tree_is_a_problem(self):
        tree = "openkdx-table v0.0.1\nreqwest v0.12.0\nsha2 v0.10.0\n"
        self.assertEqual(
            ["openkdx-table: without default features still depends on reqwest"],
            check_deps.check_feature_gate("openkdx-table", "reqwest", tree),
        )

    def test_clean_no_default_tree_passes(self):
        tree = "openkdx-table v0.0.1\nsha2 v0.10.0\n"
        self.assertEqual([], check_deps.check_feature_gate("openkdx-table", "reqwest", tree))


class LockfileTest(unittest.TestCase):
    GIT = 'git+https://github.com/spitfire915/kdx?rev=abc#abc'

    def test_lock_with_fork_rev_passes(self):
        lock = f'[[package]]\nname = "kdx-proto"\nversion = "0.1.0"\nsource = "{self.GIT}"\n'
        self.assertEqual([], check_deps.patched_kdx_packages(lock))

    def test_check_deps_rejects_patched_lock(self):
        lock = ('[[package]]\nname = "kdx-proto"\nversion = "0.1.0"\n\n'
                f'[[package]]\nname = "kdx-codec"\nversion = "0.1.0"\nsource = "{self.GIT}"\n')
        self.assertEqual(["kdx-proto"], check_deps.patched_kdx_packages(lock))

    def test_retired_member_dependency_is_a_problem(self):
        meta = {"packages": [{"id": "a", "name": "openkdx-cli"}, {"id": "b", "name": "openkdx-client"}],
                "workspace_members": ["a"],
                "resolve": {"nodes": [{"id": "a", "deps": [{"pkg": "b", "dep_kinds": [{"kind": None}]}]}, {"id": "b", "deps": []}]}}
        self.assertEqual(["openkdx-cli: depends on retired crate openkdx-client"], check_deps.check_retired(meta))

    def test_retired_crate_still_a_member_is_allowed(self):
        meta = {"packages": [{"id": "a", "name": "openkdx-cli"}, {"id": "b", "name": "openkdx-client"}],
                "workspace_members": ["a", "b"],
                "resolve": {"nodes": [{"id": "a", "deps": [{"pkg": "b", "dep_kinds": [{"kind": None}]}]}, {"id": "b", "deps": []}]}}
        self.assertEqual([], check_deps.check_retired(meta))


if __name__ == "__main__":
    unittest.main()
