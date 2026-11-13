//! `KDXTrackerServ.stg`: the tracker's settings, the `%KDXTSTG` blob of 711
//! and 712, scrambled with [`Stream::TRACKER_SETTINGS`]. Sections stay
//! opaque until Phase 5. See `docs/formats/tracker-stg.md`.

use openkdx_proto::msg::tracker::TrackerSettings;

use crate::error::Result;
use crate::scramble::{Stream, scramble, unscramble};
use crate::server_stg::check_header;
use crate::store::{LegacyDir, ServerFile};

pub const MAGIC: [u8; 8] = *b"%KDXTSTG";

pub fn decode_tracker_stg(bytes: &[u8]) -> Result<TrackerSettings> {
    let plain = unscramble(bytes, Stream::TRACKER_SETTINGS);
    check_header(&plain, MAGIC, 1, ServerFile::Tracker.name())?;
    Ok(TrackerSettings::decode(&plain)?)
}

pub fn encode_tracker_stg(settings: &TrackerSettings) -> Vec<u8> {
    scramble(&settings.to_bytes(), Stream::TRACKER_SETTINGS)
}

impl LegacyDir {
    pub fn load_tracker_settings(&self) -> Result<Option<TrackerSettings>> {
        self.read(ServerFile::Tracker)?
            .map(|b| decode_tracker_stg(&b))
            .transpose()
    }

    pub fn save_tracker_settings(&self, settings: &TrackerSettings) -> Result<()> {
        self.save(ServerFile::Tracker, &encode_tracker_stg(settings))
    }
}
