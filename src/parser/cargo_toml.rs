//! Cargo.toml parser

use crate::parser::traits::{ParseError, Parser};
use crate::parser::types::{PackageInfo, RegistryType};
use tracing::warn;

/// Parser for Cargo.toml files
pub struct CargoTomlParser;

impl CargoTomlParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CargoTomlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser for CargoTomlParser {
    fn parse(&self, content: &str) -> Result<Vec<PackageInfo>, ParseError> {
        let mut parser = tree_sitter::Parser::new();
        let language = tree_sitter_toml_ng::LANGUAGE;
        parser.set_language(&language.into()).map_err(|e| {
            warn!("Failed to set TOML language for tree-sitter: {}", e);
            ParseError::TreeSitter(e.to_string())
        })?;

        let tree = parser.parse(content, None).ok_or_else(|| {
            warn!("Failed to parse TOML content");
            ParseError::ParseFailed("Failed to parse TOML".to_string())
        })?;

        let root = tree.root_node();
        let mut results = Vec::new();

        self.extract_dependencies(root, content, &mut results);

        Ok(results)
    }
}

impl CargoTomlParser {
    /// Dependency table names to extract
    const DEPENDENCY_TABLES: [&'static str; 3] =
        ["dependencies", "dev-dependencies", "build-dependencies"];

    /// Extract dependencies from all dependency tables
    fn extract_dependencies(
        &self,
        root: tree_sitter::Node,
        content: &str,
        results: &mut Vec<PackageInfo>,
    ) {
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if child.kind() == "table" {
                self.process_table(child, content, results);
            }
        }
    }

    /// Process a TOML table node
    fn process_table(
        &self,
        table_node: tree_sitter::Node,
        content: &str,
        results: &mut Vec<PackageInfo>,
    ) {
        // Get the table header (e.g., [dependencies])
        let Some(header) = table_node.child(0) else {
            return;
        };

        if header.kind() != "[" {
            return;
        }

        // Find the table name
        let mut cursor = table_node.walk();
        let mut table_name: Option<String> = None;

        for child in table_node.children(&mut cursor) {
            if child.kind() == "bare_key" || child.kind() == "dotted_key" {
                table_name = Some(content[child.byte_range()].to_string());
                break;
            }
        }

        let Some(name) = table_name else {
            return;
        };

        if !Self::DEPENDENCY_TABLES.contains(&name.as_str()) {
            return;
        }

        // Process all pairs (key = value) in this table
        let mut cursor = table_node.walk();
        for child in table_node.children(&mut cursor) {
            if child.kind() == "pair" {
                self.extract_package_from_pair(child, content, results);
            }
        }
    }

    /// Extract package info from a key-value pair
    fn extract_package_from_pair(
        &self,
        pair_node: tree_sitter::Node,
        content: &str,
        results: &mut Vec<PackageInfo>,
    ) {
        let mut cursor = pair_node.walk();
        let mut package_name: Option<String> = None;
        let mut version_info: Option<(String, usize, usize, usize, usize)> = None;

        for child in pair_node.children(&mut cursor) {
            match child.kind() {
                "bare_key" | "dotted_key" => {
                    package_name = Some(content[child.byte_range()].to_string());
                }
                "string" => {
                    // Simple version: serde = "1.0"
                    let text = &content[child.byte_range()];
                    let version = text
                        .trim()
                        .trim_start_matches('"')
                        .trim_end_matches('"')
                        .to_string();
                    let start_point = child.start_position();
                    // Adjust for quotes
                    version_info = Some((
                        version,
                        child.start_byte() + 1,
                        child.end_byte() - 1,
                        start_point.row,
                        start_point.column + 1,
                    ));
                }
                "inline_table" => {
                    // Inline table: serde = { version = "1.0", features = ["derive"] }
                    version_info = self.extract_version_from_inline_table(child, content);
                }
                _ => {}
            }
        }

        if let (Some(name), Some((version, start_offset, end_offset, line, column))) =
            (package_name, version_info)
        {
            results.push(PackageInfo {
                name,
                version,
                commit_hash: None,
                registry_type: RegistryType::CratesIo,
                start_offset,
                end_offset,
                line,
                column,
            });
        }
    }

    /// Extract version from an inline table: { version = "1.0", ... }
    fn extract_version_from_inline_table(
        &self,
        table_node: tree_sitter::Node,
        content: &str,
    ) -> Option<(String, usize, usize, usize, usize)> {
        let mut cursor = table_node.walk();

        for child in table_node.children(&mut cursor) {
            if child.kind() == "pair" {
                let mut pair_cursor = child.walk();
                let mut is_version_key = false;

                for pair_child in child.children(&mut pair_cursor) {
                    match pair_child.kind() {
                        "bare_key" => {
                            let key = &content[pair_child.byte_range()];
                            is_version_key = key == "version";
                        }
                        "string" if is_version_key => {
                            let text = &content[pair_child.byte_range()];
                            let version = text
                                .trim()
                                .trim_start_matches('"')
                                .trim_end_matches('"')
                                .to_string();
                            let start_point = pair_child.start_position();
                            return Some((
                                version,
                                pair_child.start_byte() + 1,
                                pair_child.end_byte() - 1,
                                start_point.row,
                                start_point.column + 1,
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extracts_dependencies() {
        let parser = CargoTomlParser::new();
        let content = r#"[package]
name = "my-app"
version = "0.1.0"

[dependencies]
serde = "1.0.0"
"#;
        let result = parser.parse(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            PackageInfo {
                name: "serde".to_string(),
                version: "1.0.0".to_string(),
                commit_hash: None,
                registry_type: RegistryType::CratesIo,
                start_offset: 69,
                end_offset: 74,
                line: 5,
                column: 9,
            }
        );
    }

    #[test]
    fn parse_extracts_dev_dependencies() {
        let parser = CargoTomlParser::new();
        let content = r#"[package]
name = "my-app"

[dev-dependencies]
mockall = "0.14"
"#;
        let result = parser.parse(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            PackageInfo {
                name: "mockall".to_string(),
                version: "0.14".to_string(),
                commit_hash: None,
                registry_type: RegistryType::CratesIo,
                start_offset: 57,
                end_offset: 61,
                line: 4,
                column: 11,
            }
        );
    }

    #[test]
    fn parse_extracts_build_dependencies() {
        let parser = CargoTomlParser::new();
        let content = r#"[package]
name = "my-app"

[build-dependencies]
cc = "1.0"
"#;
        let result = parser.parse(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            PackageInfo {
                name: "cc".to_string(),
                version: "1.0".to_string(),
                commit_hash: None,
                registry_type: RegistryType::CratesIo,
                start_offset: 54,
                end_offset: 57,
                line: 4,
                column: 6,
            }
        );
    }

    #[test]
    fn parse_extracts_inline_table_version() {
        let parser = CargoTomlParser::new();
        let content = r#"[package]
name = "my-app"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
"#;
        let result = parser.parse(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "serde");
        assert_eq!(result[0].version, "1.0");
    }

    #[test]
    fn parse_extracts_all_dependency_types() {
        let parser = CargoTomlParser::new();
        let content = r#"[package]
name = "my-app"

[dependencies]
serde = "1.0"

[dev-dependencies]
mockall = "0.14"

[build-dependencies]
cc = "1.0"
"#;
        let result = parser.parse(content).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "serde");
        assert_eq!(result[1].name, "mockall");
        assert_eq!(result[2].name, "cc");
    }

    #[test]
    fn parse_returns_empty_for_no_dependencies() {
        let parser = CargoTomlParser::new();
        let content = r#"[package]
name = "my-app"
version = "0.1.0"
"#;
        let result = parser.parse(content).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_handles_version_requirements() {
        let parser = CargoTomlParser::new();
        let content = r#"[dependencies]
serde = "^1.0"
tokio = "~1.35"
anyhow = ">=1.0"
thiserror = "=2.0"
"#;
        let result = parser.parse(content).unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].version, "^1.0");
        assert_eq!(result[1].version, "~1.35");
        assert_eq!(result[2].version, ">=1.0");
        assert_eq!(result[3].version, "=2.0");
    }
}
