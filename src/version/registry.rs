//! Registry trait for fetching package versions from various sources

use crate::parser::types::RegistryType;
use crate::version::error::RegistryError;
use crate::version::types::PackageVersions;
use std::future::Future;

/// Trait for fetching package versions from a registry
pub trait Registry: Send + Sync {
    /// Returns the type of registry this implementation handles
    fn registry_type(&self) -> RegistryType;

    /// Fetches all versions for a package from the registry
    ///
    /// # Arguments
    /// * `package_name` - The name of the package (e.g., "actions/checkout" for GitHub Actions)
    ///
    /// # Returns
    /// * `Ok(PackageVersions)` - List of versions, ordered from newest to oldest
    /// * `Err(RegistryError)` - If the fetch fails
    fn fetch_all_versions(
        &self,
        package_name: &str,
    ) -> impl Future<Output = Result<PackageVersions, RegistryError>> + Send;
}
