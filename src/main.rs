#[tokio::main]
async fn main() -> anyhow::Result<()> {
    version_lsp::lsp::server::run_server().await
}
