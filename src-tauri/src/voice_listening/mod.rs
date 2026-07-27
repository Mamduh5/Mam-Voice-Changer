mod cli;
mod manifest;
mod package;
mod ratings;
mod summary;

#[cfg(test)]
mod tests;

pub use cli::cli_main;
pub use manifest::{
    ListeningClip, ListeningManifest, ListeningStudy, ListeningTransform,
    LISTENING_MANIFEST_SCHEMA_VERSION,
};
pub use package::{prepare_package, PackageSummary, PrepareOptions};
pub use ratings::{validate_ratings_file, RatingsValidation};
pub use summary::{summarize_ratings, ListeningSummary};

pub(crate) const PACKAGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const MINIMUM_CATEGORY_TRIALS: usize = 3;
