use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tracing::{error, info, warn};

use crate::config::{DEFAULT_REFRESH_INTERVAL_MS, data_dir, db_path};
use crate::lsp::diagnostics::generate_diagnostics;
use crate::parser::github_actions::GitHubActionsParser;
use crate::parser::traits::Parser;
use crate::parser::types::{RegistryType, detect_parser_type};
use crate::version::cache::Cache;

pub struct Backend {
    client: Client,
    cache: Option<Arc<Mutex<Cache>>>,
    parsers: HashMap<RegistryType, Box<dyn Parser>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let cache = Self::initialize_cache();
        let parsers = Self::initialize_parsers();
        Self {
            client,
            cache,
            parsers,
        }
    }

    fn initialize_parsers() -> HashMap<RegistryType, Box<dyn Parser>> {
        let mut parsers: HashMap<RegistryType, Box<dyn Parser>> = HashMap::new();
        parsers.insert(
            RegistryType::GitHubActions,
            Box::new(GitHubActionsParser::new()),
        );
        parsers
    }

    fn initialize_cache() -> Option<Arc<Mutex<Cache>>> {
        let data_dir = data_dir();
        let db_path = db_path();

        // Create data directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            error!("Failed to create data directory {:?}: {}", data_dir, e);
            return None;
        }

        match Cache::new(&db_path, DEFAULT_REFRESH_INTERVAL_MS) {
            Ok(cache) => {
                info!("Cache initialized at {:?}", db_path);
                Some(Arc::new(Mutex::new(cache)))
            }
            Err(e) => {
                error!("Failed to initialize cache: {}", e);
                None
            }
        }
    }

    pub fn server_capabilities() -> ServerCapabilities {
        ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::INCREMENTAL),
                    ..Default::default()
                },
            )),
            ..Default::default()
        }
    }

    fn spawn_background_refresh(&self) {
        let Some(cache) = self.cache.clone() else {
            warn!("Cache not available, skipping background refresh");
            return;
        };

        tokio::spawn(async move {
            let Some(packages) = cache
                .lock()
                .unwrap()
                .get_packages_needing_refresh()
                .inspect_err(|e| error!("Failed to get packages needing refresh: {}", e))
                .ok()
            else {
                return;
            };

            if packages.is_empty() {
                info!("No packages need refresh");
            } else {
                info!("{} packages need refresh", packages.len());
                // TODO: Phase 7+ will implement actual refresh using registries
            }
        });
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "LSP server initializing")
            .await;
        Ok(InitializeResult {
            capabilities: Self::server_capabilities(),
            server_info: Some(ServerInfo {
                name: "version-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "LSP server initialized")
            .await;
        self.spawn_background_refresh();
    }

    async fn shutdown(&self) -> Result<()> {
        self.client
            .log_message(MessageType::INFO, "LSP server shutting down")
            .await;
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.as_str();
        let content = &params.text_document.text;

        self.client
            .log_message(MessageType::LOG, format!("Document opened: {}", uri))
            .await;

        let Some(parser_type) = detect_parser_type(uri) else {
            return;
        };

        let Some(parser) = self.parsers.get(&parser_type) else {
            return;
        };

        let Some(cache) = &self.cache else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    "Cache not available, skipping diagnostics",
                )
                .await;
            return;
        };

        let diagnostics = {
            let cache_guard = cache.lock().unwrap();
            generate_diagnostics(&**parser, &*cache_guard, content)
        };

        self.client
            .log_message(
                MessageType::LOG,
                format!("Publishing {} diagnostics for {}", diagnostics.len(), uri),
            )
            .await;

        self.client
            .publish_diagnostics(params.text_document.uri, diagnostics, None)
            .await;
    }
}
