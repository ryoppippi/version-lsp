//! Registry implementations for fetching package versions

pub mod github;
pub mod npm;

pub use github::GitHubRegistry;
pub use npm::NpmRegistry;
