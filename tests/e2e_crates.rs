//! crates.io (Cargo.toml) E2E tests

mod helper;

use std::collections::HashMap;

use tower::Service;
use tower_lsp::lsp_types::*;
use tower_lsp::LspService;

use helper::{
    create_did_open_notification, create_initialize_request, create_initialized_notification,
    create_test_cache, create_test_resolver, MockRegistry, spawn_notification_collector,
    wait_for_notification,
};
use version_lsp::lsp::backend::Backend;
use version_lsp::lsp::resolver::PackageResolver;
use version_lsp::parser::types::RegistryType;

#[tokio::test(flavor = "multi_thread")]
async fn publishes_outdated_version_warning() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    // Using tilde requirement ~1.0.100 which means >=1.0.100 <1.1.0
    // Latest is 1.1.0 which is outside the range, so it's outdated
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::CratesIo,
        &[("serde", vec!["1.0.0", "1.0.100", "1.1.0"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::CratesIo)
        .with_versions("serde", vec!["1.0.0", "1.0.100", "1.1.0"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::CratesIo,
        create_test_resolver(RegistryType::CratesIo, registry),
    )]);

    // 3. Create LspService
    let (mut service, socket) =
        LspService::build(|client| Backend::build(client, cache.clone(), resolvers)).finish();

    let mut notification_rx = spawn_notification_collector(socket);

    // 4. Initialize
    service.call(create_initialize_request(1)).await.unwrap();
    service
        .call(create_initialized_notification())
        .await
        .unwrap();

    // 5. didOpen with tilde requirement (outdated because latest 1.1.0 is outside ~1.0.100)
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "~1.0.100"
"#;

    service
        .call(create_did_open_notification(
            "file:///test/Cargo.toml",
            cargo_toml,
        ))
        .await
        .unwrap();

    // 6. Receive publishDiagnostics notification
    let notification =
        wait_for_notification(&mut notification_rx, "textDocument/publishDiagnostics")
            .await
            .expect("Expected publishDiagnostics notification");

    let params: PublishDiagnosticsParams =
        serde_json::from_value(notification.params().unwrap().clone()).unwrap();
    assert_eq!(params.diagnostics.len(), 1);
    assert_eq!(
        params.diagnostics[0].severity,
        Some(DiagnosticSeverity::WARNING)
    );
    assert_eq!(
        params.diagnostics[0].message,
        "Update available: ~1.0.100 -> 1.1.0"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_diagnostics_for_latest_version() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::CratesIo,
        &[("serde", vec!["1.0.100", "1.0.200"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::CratesIo)
        .with_versions("serde", vec!["1.0.100", "1.0.200"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::CratesIo,
        create_test_resolver(RegistryType::CratesIo, registry),
    )]);

    // 3. Create LspService
    let (mut service, socket) =
        LspService::build(|client| Backend::build(client, cache.clone(), resolvers)).finish();

    let mut notification_rx = spawn_notification_collector(socket);

    // 4. Initialize
    service.call(create_initialize_request(1)).await.unwrap();
    service
        .call(create_initialized_notification())
        .await
        .unwrap();

    // 5. didOpen with latest version (caret requirement that includes latest)
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0.200"
"#;

    service
        .call(create_did_open_notification(
            "file:///test/Cargo.toml",
            cargo_toml,
        ))
        .await
        .unwrap();

    // 6. Receive publishDiagnostics notification - should be empty
    let notification =
        wait_for_notification(&mut notification_rx, "textDocument/publishDiagnostics")
            .await
            .expect("Expected publishDiagnostics notification");
    let params: PublishDiagnosticsParams =
        serde_json::from_value(notification.params().unwrap().clone()).unwrap();
    assert!(params.diagnostics.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn publishes_error_for_nonexistent_version() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::CratesIo,
        &[("serde", vec!["1.0.100", "1.0.200"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::CratesIo)
        .with_versions("serde", vec!["1.0.100", "1.0.200"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::CratesIo,
        create_test_resolver(RegistryType::CratesIo, registry),
    )]);

    // 3. Create LspService
    let (mut service, socket) =
        LspService::build(|client| Backend::build(client, cache.clone(), resolvers)).finish();

    let mut notification_rx = spawn_notification_collector(socket);

    // 4. Initialize
    service.call(create_initialize_request(1)).await.unwrap();
    service
        .call(create_initialized_notification())
        .await
        .unwrap();

    // 5. didOpen with nonexistent version
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "=999.0.0"
"#;

    service
        .call(create_did_open_notification(
            "file:///test/Cargo.toml",
            cargo_toml,
        ))
        .await
        .unwrap();

    // 6. Receive publishDiagnostics notification - should have ERROR diagnostic
    let notification =
        wait_for_notification(&mut notification_rx, "textDocument/publishDiagnostics")
            .await
            .expect("Expected publishDiagnostics notification");
    let params: PublishDiagnosticsParams =
        serde_json::from_value(notification.params().unwrap().clone()).unwrap();
    assert_eq!(params.diagnostics.len(), 1);
    assert_eq!(
        params.diagnostics[0].severity,
        Some(DiagnosticSeverity::ERROR)
    );
    assert_eq!(
        params.diagnostics[0].message,
        "Version =999.0.0 not found in registry"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn caret_range_is_latest_when_satisfied() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    // Cargo's default requirement (no prefix) is caret-like: 1.0.0 means >=1.0.0 <2.0.0
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::CratesIo,
        &[("serde", vec!["1.0.0", "1.0.100", "1.0.200"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::CratesIo)
        .with_versions("serde", vec!["1.0.0", "1.0.100", "1.0.200"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::CratesIo,
        create_test_resolver(RegistryType::CratesIo, registry),
    )]);

    // 3. Create LspService
    let (mut service, socket) =
        LspService::build(|client| Backend::build(client, cache.clone(), resolvers)).finish();

    let mut notification_rx = spawn_notification_collector(socket);

    // 4. Initialize
    service.call(create_initialize_request(1)).await.unwrap();
    service
        .call(create_initialized_notification())
        .await
        .unwrap();

    // 5. didOpen with caret range that includes latest
    // "1.0.0" in Cargo means ^1.0.0, which satisfies 1.0.200
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0.0"
"#;

    service
        .call(create_did_open_notification(
            "file:///test/Cargo.toml",
            cargo_toml,
        ))
        .await
        .unwrap();

    // 6. Receive publishDiagnostics notification - should be empty (latest 1.0.200 satisfies 1.0.0)
    let notification =
        wait_for_notification(&mut notification_rx, "textDocument/publishDiagnostics")
            .await
            .expect("Expected publishDiagnostics notification");
    let params: PublishDiagnosticsParams =
        serde_json::from_value(notification.params().unwrap().clone()).unwrap();
    assert!(params.diagnostics.is_empty());
}

/// Test [workspace.dependencies] format
#[tokio::test(flavor = "multi_thread")]
async fn workspace_dependencies_outdated_warning() {
    // 1. Setup real Cache with test data
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::CratesIo,
        &[("prost", vec!["0.12.0", "0.13.0", "0.14.0", "0.14.1"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::CratesIo)
        .with_versions("prost", vec!["0.12.0", "0.13.0", "0.14.0", "0.14.1"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::CratesIo,
        create_test_resolver(RegistryType::CratesIo, registry),
    )]);

    // 3. Create LspService
    let (mut service, socket) =
        LspService::build(|client| Backend::build(client, cache.clone(), resolvers)).finish();

    let mut notification_rx = spawn_notification_collector(socket);

    // 4. Initialize
    service.call(create_initialize_request(1)).await.unwrap();
    service
        .call(create_initialized_notification())
        .await
        .unwrap();

    // 5. didOpen with workspace.dependencies format
    // "0.13" means ^0.13.0, which does NOT satisfy 0.14.1 (0.x caret semantics)
    let cargo_toml = r#"[workspace]
members = ["crates/*"]

[workspace.dependencies]
prost = "0.13"
"#;

    service
        .call(create_did_open_notification(
            "file:///test/Cargo.toml",
            cargo_toml,
        ))
        .await
        .unwrap();

    // 6. Receive publishDiagnostics notification - should have WARNING
    let notification =
        wait_for_notification(&mut notification_rx, "textDocument/publishDiagnostics")
            .await
            .expect("Expected publishDiagnostics notification");
    let params: PublishDiagnosticsParams =
        serde_json::from_value(notification.params().unwrap().clone()).unwrap();
    assert_eq!(params.diagnostics.len(), 1);
    assert_eq!(
        params.diagnostics[0].severity,
        Some(DiagnosticSeverity::WARNING)
    );
    assert_eq!(
        params.diagnostics[0].message,
        "Update available: 0.13 -> 0.14.1"
    );
}
