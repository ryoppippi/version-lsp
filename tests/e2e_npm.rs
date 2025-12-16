//! npm (package.json) E2E tests

mod helper;

use std::collections::HashMap;

use tower::Service;
use tower_lsp::LspService;
use tower_lsp::lsp_types::*;

use helper::{
    MockRegistry, create_did_open_notification, create_initialize_request,
    create_initialized_notification, create_test_cache, create_test_resolver,
    spawn_notification_collector, wait_for_notification,
};
use version_lsp::lsp::backend::Backend;
use version_lsp::lsp::resolver::PackageResolver;
use version_lsp::parser::types::RegistryType;

#[tokio::test(flavor = "multi_thread")]
async fn publishes_outdated_version_warning() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::Npm,
        &[("lodash", vec!["4.17.19", "4.17.20", "4.17.21"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::Npm)
        .with_versions("lodash", vec!["4.17.19", "4.17.20", "4.17.21"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::Npm,
        create_test_resolver(RegistryType::Npm, registry),
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

    // 5. didOpen with outdated version
    let package_json = r#"{
  "name": "test-project",
  "dependencies": {
    "lodash": "4.17.20"
  }
}"#;

    service
        .call(create_did_open_notification(
            "file:///test/package.json",
            package_json,
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
        "Update available: 4.17.20 -> 4.17.21"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_diagnostics_for_latest_version() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    let (_temp_dir, cache) =
        create_test_cache(RegistryType::Npm, &[("lodash", vec!["4.17.20", "4.17.21"])]);

    // 2. Setup mock Registry and resolver
    let registry =
        MockRegistry::new(RegistryType::Npm).with_versions("lodash", vec!["4.17.20", "4.17.21"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::Npm,
        create_test_resolver(RegistryType::Npm, registry),
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

    // 5. didOpen with latest version
    let package_json = r#"{
  "name": "test-project",
  "dependencies": {
    "lodash": "4.17.21"
  }
}"#;

    service
        .call(create_did_open_notification(
            "file:///test/package.json",
            package_json,
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
    let (_temp_dir, cache) =
        create_test_cache(RegistryType::Npm, &[("lodash", vec!["4.17.20", "4.17.21"])]);

    // 2. Setup mock Registry and resolver
    let registry =
        MockRegistry::new(RegistryType::Npm).with_versions("lodash", vec!["4.17.20", "4.17.21"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::Npm,
        create_test_resolver(RegistryType::Npm, registry),
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
    let package_json = r#"{
  "name": "test-project",
  "dependencies": {
    "lodash": "999.0.0"
  }
}"#;

    service
        .call(create_did_open_notification(
            "file:///test/package.json",
            package_json,
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
        "Version 999.0.0 not found in registry"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn caret_range_is_latest_when_satisfied() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    // caret range ^4.17.0 satisfies latest 4.17.21
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::Npm,
        &[("lodash", vec!["4.17.0", "4.17.20", "4.17.21"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::Npm)
        .with_versions("lodash", vec!["4.17.0", "4.17.20", "4.17.21"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::Npm,
        create_test_resolver(RegistryType::Npm, registry),
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
    let package_json = r#"{
  "name": "test-project",
  "dependencies": {
    "lodash": "^4.17.0"
  }
}"#;

    service
        .call(create_did_open_notification(
            "file:///test/package.json",
            package_json,
        ))
        .await
        .unwrap();

    // 6. Receive publishDiagnostics notification - should be empty (latest 4.17.21 satisfies ^4.17.0)
    let notification =
        wait_for_notification(&mut notification_rx, "textDocument/publishDiagnostics")
            .await
            .expect("Expected publishDiagnostics notification");
    let params: PublishDiagnosticsParams =
        serde_json::from_value(notification.params().unwrap().clone()).unwrap();
    assert!(params.diagnostics.is_empty());
}
