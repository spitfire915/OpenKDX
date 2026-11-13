use openkdx_legacy::{BaseName, BaseTag, format_base_name, parse_base_name};

#[test]
fn parse_ul_db_and_item_suffixes() {
    assert_eq!(
        parse_base_name(b"Uploads [UL]"),
        BaseName {
            display: b"Uploads".to_vec(),
            tag: Some(BaseTag::Uploads)
        }
    );
    assert_eq!(
        parse_base_name(b"Admin Dropbox [DB]"),
        BaseName {
            display: b"Admin Dropbox".to_vec(),
            tag: Some(BaseTag::DropBox)
        }
    );
    assert_eq!(
        parse_base_name(b"Pics [Pictures]"),
        BaseName {
            display: b"Pics".to_vec(),
            tag: Some(BaseTag::Item(b"Pictures".to_vec()))
        }
    );
    assert_eq!(
        parse_base_name(b"Plain"),
        BaseName {
            display: b"Plain".to_vec(),
            tag: None
        }
    );
    // A bracket without the separating space is part of the name.
    assert_eq!(
        parse_base_name(b"x[UL]"),
        BaseName {
            display: b"x[UL]".to_vec(),
            tag: None
        }
    );
    assert_eq!(
        parse_base_name(b" [UL]"),
        BaseName {
            display: b" [UL]".to_vec(),
            tag: None
        }
    );
}

#[test]
fn format_is_inverse() {
    for name in [
        &b"Uploads [UL]"[..],
        b"Admin Dropbox [DB]",
        b"Pics [Pictures]",
        b"Plain",
        b"a [b] [c]",
    ] {
        assert_eq!(format_base_name(&parse_base_name(name)), name, "{name:?}");
    }
}
