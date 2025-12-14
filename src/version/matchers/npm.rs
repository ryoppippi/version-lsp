//! npm version matcher
//!
//! Supports npm semver range specifications:
//! - `1.2.3` - exact match
//! - `^1.2.3` - compatible with version (>=1.2.3 <2.0.0)
//! - `~1.2.3` - approximately equivalent (>=1.2.3 <1.3.0)
//! - `>=1.2.3`, `>1.2.3`, `<=1.2.3`, `<1.2.3` - comparison operators
//! - `1.2.x`, `1.x`, `*` - wildcards

use semver::Version;

use crate::parser::types::RegistryType;
use crate::version::matcher::VersionMatcher;
use crate::version::semver::CompareResult;

pub struct NpmVersionMatcher;

/// Represents a parsed npm version range
#[derive(Debug)]
enum VersionRange {
    /// Exact version match
    Exact(Version),
    /// Caret range: ^1.2.3 means >=1.2.3 <2.0.0 (or special cases for 0.x)
    Caret(Version),
    /// Tilde range: ~1.2.3 means >=1.2.3 <1.3.0
    Tilde(Version),
    /// Greater than or equal
    Gte(Version),
    /// Greater than
    Gt(Version),
    /// Less than or equal
    Lte(Version),
    /// Less than
    Lt(Version),
    /// Any version: * matches all versions
    Any,
    /// Wildcard major: 1.x means >=1.0.0 <2.0.0
    WildcardMajor(u64),
    /// Wildcard minor: 1.2.x means >=1.2.0 <1.3.0
    WildcardMinor(u64, u64),
}

impl VersionRange {
    /// Parse a version specification string into a VersionRange
    fn parse(spec: &str) -> Option<Self> {
        let spec = spec.trim();

        if let Some(rest) = spec.strip_prefix(">=") {
            Version::parse(rest.trim()).ok().map(VersionRange::Gte)
        } else if let Some(rest) = spec.strip_prefix('>') {
            Version::parse(rest.trim()).ok().map(VersionRange::Gt)
        } else if let Some(rest) = spec.strip_prefix("<=") {
            Version::parse(rest.trim()).ok().map(VersionRange::Lte)
        } else if let Some(rest) = spec.strip_prefix('<') {
            Version::parse(rest.trim()).ok().map(VersionRange::Lt)
        } else if let Some(rest) = spec.strip_prefix('^') {
            Version::parse(rest.trim()).ok().map(VersionRange::Caret)
        } else if let Some(rest) = spec.strip_prefix('~') {
            Version::parse(rest.trim()).ok().map(VersionRange::Tilde)
        } else if spec == "*" {
            Some(VersionRange::Any)
        } else if let Some(range) = Self::parse_wildcard(spec) {
            Some(range)
        } else {
            Version::parse(spec).ok().map(VersionRange::Exact)
        }
    }

    /// Parse wildcard patterns like "1.x" or "1.2.x"
    fn parse_wildcard(spec: &str) -> Option<Self> {
        let parts: Vec<&str> = spec.split('.').collect();

        match parts.as_slice() {
            // 1.x or 1.X
            [major, x] if x.eq_ignore_ascii_case("x") => {
                major.parse::<u64>().ok().map(VersionRange::WildcardMajor)
            }
            // 1.2.x or 1.2.X
            [major, minor, x] if x.eq_ignore_ascii_case("x") => {
                let major = major.parse::<u64>().ok()?;
                let minor = minor.parse::<u64>().ok()?;
                Some(VersionRange::WildcardMinor(major, minor))
            }
            _ => None,
        }
    }

    /// Check if a version satisfies this range
    fn satisfies(&self, version: &Version) -> bool {
        match self {
            VersionRange::Exact(v) => version == v,
            VersionRange::Caret(v) => {
                if version < v {
                    return false;
                }
                // ^1.2.3 -> >=1.2.3 <2.0.0
                // ^0.2.3 -> >=0.2.3 <0.3.0
                // ^0.0.3 -> >=0.0.3 <0.0.4
                if v.major == 0 {
                    if v.minor == 0 {
                        // ^0.0.x: only patch must match
                        version.major == 0 && version.minor == 0 && version.patch == v.patch
                    } else {
                        // ^0.x.y: major and minor must match
                        version.major == 0 && version.minor == v.minor
                    }
                } else {
                    // ^x.y.z: major must match
                    version.major == v.major
                }
            }
            VersionRange::Tilde(v) => {
                // ~1.2.3 -> >=1.2.3 <1.3.0
                version >= v && version.major == v.major && version.minor == v.minor
            }
            VersionRange::Gte(v) => version >= v,
            VersionRange::Gt(v) => version > v,
            VersionRange::Lte(v) => version <= v,
            VersionRange::Lt(v) => version < v,
            VersionRange::Any => true,
            VersionRange::WildcardMajor(major) => version.major == *major,
            VersionRange::WildcardMinor(major, minor) => {
                version.major == *major && version.minor == *minor
            }
        }
    }

    /// Get the base version from this range (for comparison purposes)
    /// Returns None for Any (*) since any version is acceptable
    fn base_version(&self) -> Option<Version> {
        match self {
            VersionRange::Exact(v)
            | VersionRange::Caret(v)
            | VersionRange::Tilde(v)
            | VersionRange::Gte(v)
            | VersionRange::Gt(v)
            | VersionRange::Lte(v)
            | VersionRange::Lt(v) => Some(v.clone()),
            VersionRange::Any => None,
            VersionRange::WildcardMajor(major) => Some(Version::new(*major, 0, 0)),
            VersionRange::WildcardMinor(major, minor) => Some(Version::new(*major, *minor, 0)),
        }
    }
}

impl VersionMatcher for NpmVersionMatcher {
    fn registry_type(&self) -> RegistryType {
        RegistryType::Npm
    }

    fn version_exists(&self, version_spec: &str, available_versions: &[String]) -> bool {
        let Some(range) = VersionRange::parse(version_spec) else {
            return false;
        };

        available_versions.iter().any(|v| {
            Version::parse(v)
                .map(|ver| range.satisfies(&ver))
                .unwrap_or(false)
        })
    }

    fn compare_to_latest(&self, current_version: &str, latest_version: &str) -> CompareResult {
        let Some(range) = VersionRange::parse(current_version) else {
            return CompareResult::Invalid;
        };

        let Ok(latest) = Version::parse(latest_version) else {
            return CompareResult::Invalid;
        };

        // Check if latest is within the range
        if range.satisfies(&latest) {
            return CompareResult::Latest;
        }

        // For Any (*), if not satisfied (which can't happen), treat as Latest
        let Some(base) = range.base_version() else {
            return CompareResult::Latest;
        };

        if base < latest {
            CompareResult::Outdated
        } else {
            CompareResult::Newer
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // version_exists tests - exact match
    #[rstest]
    #[case("1.0.0", vec!["1.0.0", "2.0.0"], true)]
    #[case("1.0.0", vec!["1.0.1", "2.0.0"], false)]
    fn version_exists_exact_match(
        #[case] version_spec: &str,
        #[case] available: Vec<&str>,
        #[case] expected: bool,
    ) {
        let available: Vec<String> = available.into_iter().map(|s| s.to_string()).collect();
        assert_eq!(
            NpmVersionMatcher.version_exists(version_spec, &available),
            expected
        );
    }

    // version_exists tests - caret (^) range
    #[rstest]
    // ^1.2.3 matches >=1.2.3 <2.0.0
    #[case("^1.2.3", vec!["1.2.3", "1.3.0", "2.0.0"], true)]
    #[case("^1.2.3", vec!["1.9.9", "2.0.0"], true)]
    #[case("^1.2.3", vec!["1.2.2", "2.0.0"], false)]
    #[case("^1.2.3", vec!["2.0.0", "3.0.0"], false)]
    // ^0.2.3 matches >=0.2.3 <0.3.0 (special case for 0.x)
    #[case("^0.2.3", vec!["0.2.3", "0.2.9"], true)]
    #[case("^0.2.3", vec!["0.3.0", "1.0.0"], false)]
    // ^0.0.3 matches >=0.0.3 <0.0.4 (special case for 0.0.x)
    #[case("^0.0.3", vec!["0.0.3"], true)]
    #[case("^0.0.3", vec!["0.0.4", "0.1.0"], false)]
    fn version_exists_caret_range(
        #[case] version_spec: &str,
        #[case] available: Vec<&str>,
        #[case] expected: bool,
    ) {
        let available: Vec<String> = available.into_iter().map(|s| s.to_string()).collect();
        assert_eq!(
            NpmVersionMatcher.version_exists(version_spec, &available),
            expected
        );
    }

    // version_exists tests - tilde (~) range
    #[rstest]
    // ~1.2.3 matches >=1.2.3 <1.3.0
    #[case("~1.2.3", vec!["1.2.3", "1.2.9"], true)]
    #[case("~1.2.3", vec!["1.3.0", "2.0.0"], false)]
    #[case("~1.2.3", vec!["1.2.2"], false)]
    fn version_exists_tilde_range(
        #[case] version_spec: &str,
        #[case] available: Vec<&str>,
        #[case] expected: bool,
    ) {
        let available: Vec<String> = available.into_iter().map(|s| s.to_string()).collect();
        assert_eq!(
            NpmVersionMatcher.version_exists(version_spec, &available),
            expected
        );
    }

    // version_exists tests - comparison operators
    #[rstest]
    #[case(">=1.0.0", vec!["1.0.0", "2.0.0"], true)]
    #[case(">=1.0.0", vec!["0.9.9"], false)]
    #[case(">1.0.0", vec!["1.0.1", "2.0.0"], true)]
    #[case(">1.0.0", vec!["1.0.0", "0.9.9"], false)]
    #[case("<=1.0.0", vec!["1.0.0", "0.9.0"], true)]
    #[case("<=1.0.0", vec!["1.0.1", "2.0.0"], false)]
    #[case("<1.0.0", vec!["0.9.9", "0.1.0"], true)]
    #[case("<1.0.0", vec!["1.0.0", "2.0.0"], false)]
    fn version_exists_comparison_operators(
        #[case] version_spec: &str,
        #[case] available: Vec<&str>,
        #[case] expected: bool,
    ) {
        let available: Vec<String> = available.into_iter().map(|s| s.to_string()).collect();
        assert_eq!(
            NpmVersionMatcher.version_exists(version_spec, &available),
            expected
        );
    }

    // version_exists tests - wildcards
    #[rstest]
    // * matches any version
    #[case("*", vec!["1.0.0", "2.0.0"], true)]
    #[case("*", vec!["0.0.1"], true)]
    // 1.x matches >=1.0.0 <2.0.0
    #[case("1.x", vec!["1.0.0", "1.9.9"], true)]
    #[case("1.x", vec!["0.9.9", "2.0.0"], false)]
    #[case("1.X", vec!["1.5.0"], true)]
    // 1.2.x matches >=1.2.0 <1.3.0
    #[case("1.2.x", vec!["1.2.0", "1.2.9"], true)]
    #[case("1.2.x", vec!["1.1.9", "1.3.0"], false)]
    #[case("1.2.X", vec!["1.2.5"], true)]
    fn version_exists_wildcards(
        #[case] version_spec: &str,
        #[case] available: Vec<&str>,
        #[case] expected: bool,
    ) {
        let available: Vec<String> = available.into_iter().map(|s| s.to_string()).collect();
        assert_eq!(
            NpmVersionMatcher.version_exists(version_spec, &available),
            expected
        );
    }

    // compare_to_latest tests
    #[rstest]
    // Exact version comparison
    #[case("1.0.0", "1.0.0", CompareResult::Latest)]
    #[case("1.0.0", "2.0.0", CompareResult::Outdated)]
    #[case("2.0.0", "1.0.0", CompareResult::Newer)]
    // Range spec - compare base version to latest
    #[case("^1.0.0", "1.9.9", CompareResult::Latest)]
    #[case("^1.0.0", "2.0.0", CompareResult::Outdated)]
    #[case("~1.2.0", "1.2.9", CompareResult::Latest)]
    #[case("~1.2.0", "1.3.0", CompareResult::Outdated)]
    // Wildcards
    #[case("*", "999.0.0", CompareResult::Latest)]
    #[case("1.x", "1.9.9", CompareResult::Latest)]
    #[case("1.x", "2.0.0", CompareResult::Outdated)]
    #[case("1.2.x", "1.2.9", CompareResult::Latest)]
    #[case("1.2.x", "1.3.0", CompareResult::Outdated)]
    // Invalid versions
    #[case("invalid", "1.0.0", CompareResult::Invalid)]
    #[case("1.0.0", "invalid", CompareResult::Invalid)]
    fn compare_to_latest_returns_expected(
        #[case] current: &str,
        #[case] latest: &str,
        #[case] expected: CompareResult,
    ) {
        assert_eq!(
            NpmVersionMatcher.compare_to_latest(current, latest),
            expected
        );
    }
}
