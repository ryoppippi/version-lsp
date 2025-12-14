//! Common types for version management

use std::collections::HashMap;

/// Collection of versions for a package
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageVersions {
    /// List of versions, ordered from newest to oldest
    pub versions: Vec<String>,
    /// Dist tags mapping tag names to versions (e.g., "latest" -> "4.17.21")
    pub dist_tags: HashMap<String, String>,
}

impl PackageVersions {
    /// Creates a new PackageVersions with the given versions
    pub fn new(versions: Vec<String>) -> Self {
        Self {
            versions,
            dist_tags: HashMap::new(),
        }
    }

    /// Creates a new PackageVersions with versions and dist tags
    pub fn with_dist_tags(versions: Vec<String>, dist_tags: HashMap<String, String>) -> Self {
        Self {
            versions,
            dist_tags,
        }
    }

    /// Returns the latest (first) version, if any
    pub fn latest(&self) -> Option<&str> {
        self.versions.first().map(|s| s.as_str())
    }

    /// Returns true if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    /// Resolve a dist tag to its version
    pub fn resolve_dist_tag(&self, tag: &str) -> Option<&str> {
        self.dist_tags.get(tag).map(|s| s.as_str())
    }
}
