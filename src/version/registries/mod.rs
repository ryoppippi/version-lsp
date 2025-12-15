//! Registry implementations for fetching package versions

pub mod crates_io;
pub mod github;
pub mod npm;

pub use crates_io::CratesIoRegistry;
pub use github::GitHubRegistry;
pub use npm::NpmRegistry;
