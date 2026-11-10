use openkdx_table::{Cache, Story, TableError, extract, from_env};

fn real_table(story: Story) -> Option<Vec<u8>> {
    if let Ok(Some(t)) = from_env(story) {
        return Some(t);
    }
    Cache::new(Cache::default_dir()).get(story).ok().flatten()
}

#[test]
fn story_constants_match_the_reference_spec() {
    assert_eq!(Story::Kdx.len(), 1739);
    assert_eq!(Story::Gconsole.len(), 3193);
    assert_eq!(Story::Kdx.file_name(), "kdx.bin");
    assert_eq!(Story::Gconsole.file_name(), "hgc.bin");
    assert_eq!(
        hex::encode(Story::Kdx.sha256()),
        "4fca7636e77e85fdde95862a6be66e12e5e031ad61add65c5e975d3daff4b9e7"
    );
    assert_eq!(
        hex::encode(Story::Gconsole.sha256()),
        "b43f22222971a57a0d8bdb3a5af2747c7ff511ea88f0ba8c24e38e9edc029c36"
    );
    assert!(
        Story::Kdx
            .anchor()
            .starts_with(b"This program including the following story")
    );
    assert!(
        Story::Gconsole
            .anchor()
            .starts_with(b"The following story is Copyright 2004")
    );
}

#[test]
fn extract_rejects_missing_anchor() {
    let junk = vec![0xA5u8; 4096];
    assert!(matches!(
        extract(Story::Kdx, &junk),
        Err(TableError::AnchorNotFound)
    ));
}

#[test]
fn extract_rejects_wrong_hash() {
    let mut hay = Story::Kdx.anchor().to_vec();
    hay.extend(std::iter::repeat_n(0u8, 4000));
    assert!(matches!(
        extract(Story::Kdx, &hay),
        Err(TableError::HashMismatch { .. })
    ));
}

#[test]
fn extract_accepts_real_story() {
    for story in [Story::Kdx, Story::Gconsole] {
        let Some(table) = real_table(story) else {
            eprintln!("skipped: no {} table available", story.file_name());
            continue;
        };
        let hay = [b"junk".as_slice(), &table, b"more junk"].concat();
        assert_eq!(extract(story, &hay).unwrap(), table);
    }
}
