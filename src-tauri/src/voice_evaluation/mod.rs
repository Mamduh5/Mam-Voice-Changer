pub(crate) mod analysis;
mod cli;
mod fixtures;
pub(crate) mod manifest;
mod report;
mod runner;
pub(crate) mod world;

pub use cli::cli_main;
pub use fixtures::generate_example;
pub use manifest::{EvaluationManifest, EvaluationRenderer, MANIFEST_SCHEMA_VERSION};
pub(crate) use report::PitchAnalysisMetadata;
pub use report::{EvaluationReport, REPORT_SCHEMA_VERSION};
pub use runner::{evaluate_manifest_file, EvaluationOptions};
