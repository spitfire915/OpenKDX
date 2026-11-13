//! `Bases/` folder-name conventions: a trailing ` [UL]`, ` [DB]` or
//! ` [<item>]` binds a folder to the Uploads, Drop Box, or a named Folder
//! Access item. See `docs/formats/bases.md`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseTag {
    Uploads,
    DropBox,
    Item(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseName {
    pub display: Vec<u8>,
    pub tag: Option<BaseTag>,
}

pub fn parse_base_name(name: &[u8]) -> BaseName {
    if let Some(stripped) = name.strip_suffix(b"]")
        && let Some(open) = stripped.iter().rposition(|&b| b == b'[')
        && open >= 2
        && stripped[open - 1] == b' '
    {
        let display = &stripped[..open - 1];
        let item = &stripped[open + 1..];
        if !item.is_empty() && !display.is_empty() {
            let tag = match item {
                b"UL" => BaseTag::Uploads,
                b"DB" => BaseTag::DropBox,
                other => BaseTag::Item(other.to_vec()),
            };
            return BaseName {
                display: display.to_vec(),
                tag: Some(tag),
            };
        }
    }
    BaseName {
        display: name.to_vec(),
        tag: None,
    }
}

pub fn format_base_name(name: &BaseName) -> Vec<u8> {
    let mut out = name.display.clone();
    match &name.tag {
        None => {}
        Some(BaseTag::Uploads) => out.extend_from_slice(b" [UL]"),
        Some(BaseTag::DropBox) => out.extend_from_slice(b" [DB]"),
        Some(BaseTag::Item(item)) => {
            out.extend_from_slice(b" [");
            out.extend_from_slice(item);
            out.push(b']');
        }
    }
    out
}
