//! pnpm catalog version matcher
//!
//! Uses the same version matching logic as npm since pnpm catalogs use npm registry.

use crate::parser::types::RegistryType;
use crate::version::matcher::VersionMatcher;
use crate::version::matchers::npm::{npm_compare_to_latest, npm_version_exists};
use crate::version::semver::CompareResult;

/// pnpm catalog version matcher
/// Uses the same logic as npm since pnpm catalogs use npm registry
pub struct PnpmCatalogMatcher;

impl VersionMatcher for PnpmCatalogMatcher {
    fn registry_type(&self) -> RegistryType {
        RegistryType::PnpmCatalog
    }

    fn version_exists(&self, version_spec: &str, available_versions: &[String]) -> bool {
        npm_version_exists(version_spec, available_versions)
    }

    fn compare_to_latest(&self, current_version: &str, latest_version: &str) -> CompareResult {
        npm_compare_to_latest(current_version, latest_version)
    }
}
