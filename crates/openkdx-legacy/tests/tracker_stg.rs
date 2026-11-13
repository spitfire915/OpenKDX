use openkdx_legacy::tracker_stg::{decode_tracker_stg, encode_tracker_stg};
use openkdx_legacy::{LegacyDir, LegacyError, ServerFile};
use openkdx_proto::msg::tracker::TrackerSettings;

fn fixture() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/legacy/lab-2026-11-13/tracker/KDXTrackerServ.stg"
    ))
    .unwrap()
}

#[test]
fn every_tracker_stg_fixture_round_trips() {
    let bytes = fixture();
    let s = decode_tracker_stg(&bytes).unwrap();
    assert_eq!(encode_tracker_stg(&s), bytes);
    // Sections stay opaque until Phase 5, but their shape is known.
    assert_eq!(
        s.sections[TrackerSettings::SECTION_GENERAL]
            .as_ref()
            .unwrap()
            .len(),
        0x5c
    );
    assert_eq!(
        s.sections[TrackerSettings::SECTION_GROUPS]
            .as_ref()
            .unwrap()
            .len(),
        0x124
    );
    assert_eq!(
        s.sections[TrackerSettings::SECTION_ADMIN]
            .as_ref()
            .unwrap()
            .len(),
        0x9c
    );
    assert!(s.sections[3].is_none());
    assert_eq!(s.sections[1].as_deref(), Some(&[][..]));
    let general = s.sections[0].as_ref().unwrap();
    assert!(general.windows(15).any(|w| w == b"New TrackerServ"));
}

#[test]
fn tracker_stg_wrong_magic_and_version() {
    // Plain bytes with the wrong magic are not even the right scrambling.
    assert!(matches!(
        decode_tracker_stg(&[0u8; 0x78]),
        Err(LegacyError::BadMagic { .. })
    ));
    let mut plain = openkdx_legacy::scramble::unscramble(
        &fixture(),
        openkdx_legacy::scramble::Stream::TRACKER_SETTINGS,
    );
    plain[9] = 2;
    let bytes = openkdx_legacy::scramble::scramble(
        &plain,
        openkdx_legacy::scramble::Stream::TRACKER_SETTINGS,
    );
    assert!(matches!(
        decode_tracker_stg(&bytes),
        Err(LegacyError::UnsupportedVersion {
            file: "KDXTrackerServ.stg",
            version: 2
        })
    ));
}

#[test]
fn legacy_dir_loads_and_saves_tracker_settings() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("KDXTrackerServ.stg"), fixture()).unwrap();
    let store = LegacyDir::open(dir.path()).unwrap();
    let mut s = store.load_tracker_settings().unwrap().unwrap();
    s.sections[3] = Some(vec![1, 2, 3, 4]);
    store.save_tracker_settings(&s).unwrap();
    let again = store.load_tracker_settings().unwrap().unwrap();
    assert_eq!(again.sections[3].as_deref(), Some(&[1, 2, 3, 4][..]));
    assert_eq!(again, s);
    assert!(
        store
            .path(ServerFile::Tracker)
            .with_extension("stg.orig")
            .exists()
    );
}
