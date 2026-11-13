//! Server paths as strings: names joined by `/`.

use openkdx_proto::msg::common::{FilePath, PathName};

/// `"a/b c/d.txt"` → three names; `""` or `"/"` → the root. Empty segments
/// are dropped, so a name cannot contain `/` (nor can it on the server).
pub fn parse_path(s: &str) -> FilePath {
    FilePath {
        names: s
            .split('/')
            .filter(|seg| !seg.is_empty())
            .map(|seg| PathName::new(seg.as_bytes()))
            .collect(),
    }
}

/// Names joined by `/`, lossily decoded.
pub fn path_string(path: &FilePath) -> String {
    path.to_string()
}

pub fn join(parent: &FilePath, name: &[u8]) -> FilePath {
    let mut names = parent.names.clone();
    names.push(PathName::new(name));
    FilePath { names }
}
