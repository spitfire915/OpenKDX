use openkdx_proto::ChallengeTable;
use openkdx_table::{Cache, Story, from_env};

use crate::{ClientError, Result};

/// The KDX challenge table from `OPENKDX_KDX_TABLE` (base64) or the
/// `openkdx-table` cache. The real table is required: the original server
/// checks the challenge response against its own copy.
pub fn load_kdx_table() -> Result<ChallengeTable> {
    if let Ok(Some(bytes)) = from_env(Story::Kdx) {
        return Ok(ChallengeTable::new(bytes));
    }
    load_kdx_table_from(&Cache::new(Cache::default_dir()))
}

/// The KDX challenge table from a given cache only.
pub fn load_kdx_table_from(cache: &Cache) -> Result<ChallengeTable> {
    match cache.get(Story::Kdx) {
        Ok(Some(bytes)) => Ok(ChallengeTable::new(bytes)),
        _ => Err(ClientError::NoTable),
    }
}
