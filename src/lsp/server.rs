use crate::log::init;
use tower_lsp::{LspService, Server};
use tracing::info;

use crate::lsp::backend::Backend;

pub async fn run_server() -> anyhow::Result<()> {
    init()?;

    info!("Starting version-lsp server");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;

    info!("version-lsp server stopped");
    Ok(())
}
