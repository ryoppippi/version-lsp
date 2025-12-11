use std::sync::{Arc, Mutex};

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tracing::{error, info, warn};

use crate::config::{DEFAULT_REFRESH_INTERVAL_MS, data_dir, db_path};
use crate::version::cache::Cache;

pub struct Backend {
    client: Client,
    cache: Option<Arc<Mutex<Cache>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let cache = Self::initialize_cache();
        Self { client, cache }
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
        info!("LSP server initializing");
        Ok(InitializeResult {
            capabilities: Self::server_capabilities(),
            server_info: Some(ServerInfo {
                name: "version-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        info!("LSP server initialized");
        self.spawn_background_refresh();
    }

    async fn shutdown(&self) -> Result<()> {
        info!("LSP server shutting down");
        Ok(())
    }
}
