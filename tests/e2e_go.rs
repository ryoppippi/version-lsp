//! Go (go.mod) E2E tests

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
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::GoProxy,
        &[("golang.org/x/text", vec!["v0.12.0", "v0.13.0", "v0.14.0"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::GoProxy)
        .with_versions("golang.org/x/text", vec!["v0.12.0", "v0.13.0", "v0.14.0"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::GoProxy,
        create_test_resolver(RegistryType::GoProxy, registry),
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
    let go_mod = r#"module example.com/myapp

go 1.21

require golang.org/x/text v0.12.0
"#;

    service
        .call(create_did_open_notification("file:///test/go.mod", go_mod))
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
        "Update available: v0.12.0 -> v0.14.0"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_diagnostics_for_latest_version() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::GoProxy,
        &[("golang.org/x/text", vec!["v0.13.0", "v0.14.0"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::GoProxy)
        .with_versions("golang.org/x/text", vec!["v0.13.0", "v0.14.0"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::GoProxy,
        create_test_resolver(RegistryType::GoProxy, registry),
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
    let go_mod = r#"module example.com/myapp

go 1.21

require golang.org/x/text v0.14.0
"#;

    service
        .call(create_did_open_notification("file:///test/go.mod", go_mod))
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
        RegistryType::GoProxy,
        &[("golang.org/x/text", vec!["v0.13.0", "v0.14.0"])],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::GoProxy)
        .with_versions("golang.org/x/text", vec!["v0.13.0", "v0.14.0"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::GoProxy,
        create_test_resolver(RegistryType::GoProxy, registry),
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
    let go_mod = r#"module example.com/myapp

go 1.21

require golang.org/x/text v999.0.0
"#;

    service
        .call(create_did_open_notification("file:///test/go.mod", go_mod))
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
        "Version v999.0.0 not found in registry"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn require_block_publishes_outdated_version_warning() {
    // 1. Setup real Cache with test data (oldest first, newest last)
    let (_temp_dir, cache) = create_test_cache(
        RegistryType::GoProxy,
        &[
            ("golang.org/x/text", vec!["v0.12.0", "v0.14.0"]),
            ("golang.org/x/net", vec!["v0.19.0", "v0.20.0"]),
        ],
    );

    // 2. Setup mock Registry and resolver
    let registry = MockRegistry::new(RegistryType::GoProxy)
        .with_versions("golang.org/x/text", vec!["v0.12.0", "v0.14.0"])
        .with_versions("golang.org/x/net", vec!["v0.19.0", "v0.20.0"]);

    let resolvers: HashMap<RegistryType, PackageResolver> = HashMap::from([(
        RegistryType::GoProxy,
        create_test_resolver(RegistryType::GoProxy, registry),
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

    // 5. didOpen with require block containing outdated versions
    let go_mod = r#"module example.com/myapp

go 1.21

require (
	golang.org/x/text v0.12.0
	golang.org/x/net v0.19.0
)
"#;

    service
        .call(create_did_open_notification("file:///test/go.mod", go_mod))
        .await
        .unwrap();

    // 6. Receive publishDiagnostics notification
    let notification =
        wait_for_notification(&mut notification_rx, "textDocument/publishDiagnostics")
            .await
            .expect("Expected publishDiagnostics notification");

    let params: PublishDiagnosticsParams =
        serde_json::from_value(notification.params().unwrap().clone()).unwrap();
    assert_eq!(params.diagnostics.len(), 2);

    // Both diagnostics should be warnings about outdated versions
    for diag in &params.diagnostics {
        assert_eq!(diag.severity, Some(DiagnosticSeverity::WARNING));
        assert!(diag.message.starts_with("Update available:"));
    }
}
