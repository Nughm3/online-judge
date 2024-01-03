use std::path::Path;

use landlock::{
    path_beneath_rules, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetError,
    RulesetStatus,
};
use thiserror::Error;

const LANDLOCK_ABI: landlock::ABI = landlock::ABI::V3;
const LIBRARY_PATHS: &[&str] = &["/lib", "/usr/lib", "/usr/local/lib", "/nix/store"];

#[derive(Debug, Error)]
pub enum LandlockError {
    #[error(transparent)]
    Ruleset(#[from] RulesetError),
    #[error("kernel does not support landlock")]
    Unsupported,
}

pub fn restrict_thread(dir: impl AsRef<Path>) -> Result<(), LandlockError> {
    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(LANDLOCK_ABI))?
        .create()?
        .add_rules(path_beneath_rules(
            std::iter::once(dir),
            AccessFs::from_all(LANDLOCK_ABI),
        ))?
        .add_rules(path_beneath_rules(
            LIBRARY_PATHS.iter(),
            AccessFs::from_read(LANDLOCK_ABI),
        ))?
        .restrict_self()?;

    match status.ruleset {
        RulesetStatus::FullyEnforced => Ok(()),
        RulesetStatus::PartiallyEnforced => {
            tracing::warn!("landlock ruleset only partially enforced");
            Ok(())
        }
        RulesetStatus::NotEnforced => Err(LandlockError::Unsupported),
    }
}
