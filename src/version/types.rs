//! Common types for version management

/// Collection of versions for a package
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageVersions {
    /// List of versions, ordered from newest to oldest
    pub versions: Vec<String>,
}

impl PackageVersions {
    /// Creates a new PackageVersions with the given versions
    pub fn new(versions: Vec<String>) -> Self {
        Self { versions }
    }

    /// Returns the latest (first) version, if any
    pub fn latest(&self) -> Option<&str> {
        self.versions.first().map(|s| s.as_str())
    }

    /// Returns true if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }
}
