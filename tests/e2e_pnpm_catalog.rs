//! pnpm catalog (pnpm-workspace.yaml) E2E tests

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
async fn publishes_outdated_version_warning_for_single_catalog() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::PnpmCatalog,
        &[("lodash", vec!["4.17.19", "4.17.20", "4.17.21"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::PnpmCatalog)
        .with_versions("lodash", vec!["4.17.19", "4.17.20", "4.17.21"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::PnpmCatalog,
        create_test_resolver(RegistryType::PnpmCatalog, registry),
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

    // 5. didOpen with outdated version in single catalog
    let pnpm_workspace = r#"catalog:
  lodash: 4.17.20
"#;

    service
        .call(create_did_open_notification(
            "file:///test/pnpm-workspace.yaml",
            pnpm_workspace,
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
async fn publishes_outdated_version_warning_for_named_catalogs() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::PnpmCatalog,
        &[("react", vec!["17.0.2", "18.2.0", "18.3.1"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::PnpmCatalog)
        .with_versions("react", vec!["17.0.2", "18.2.0", "18.3.1"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::PnpmCatalog,
        create_test_resolver(RegistryType::PnpmCatalog, registry),
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

    // 5. didOpen with outdated version in named catalogs
    let pnpm_workspace = r#"catalogs:
  react18:
    react: 18.2.0
"#;

    service
        .call(create_did_open_notification(
            "file:///test/pnpm-workspace.yaml",
            pnpm_workspace,
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
        "Update available: 18.2.0 -> 18.3.1"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_diagnostics_for_latest_version() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::PnpmCatalog,
        &[("lodash", vec!["4.17.20", "4.17.21"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::PnpmCatalog)
        .with_versions("lodash", vec!["4.17.20", "4.17.21"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::PnpmCatalog,
        create_test_resolver(RegistryType::PnpmCatalog, registry),
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
    let pnpm_workspace = r#"catalog:
  lodash: 4.17.21
"#;

    service
        .call(create_did_open_notification(
            "file:///test/pnpm-workspace.yaml",
            pnpm_workspace,
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
        RegistryType::PnpmCatalog,
        &[("lodash", vec!["4.17.20", "4.17.21"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::PnpmCatalog)
        .with_versions("lodash", vec!["4.17.20", "4.17.21"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::PnpmCatalog,
        create_test_resolver(RegistryType::PnpmCatalog, registry),
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
    let pnpm_workspace = r#"catalog:
  lodash: 999.0.0
"#;

    service
        .call(create_did_open_notification(
            "file:///test/pnpm-workspace.yaml",
            pnpm_workspace,
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
