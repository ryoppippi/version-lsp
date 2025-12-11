use tempfile::TempDir;
use version_lsp::version::cache::Cache;

#[tokio::test]
async fn new_creates_required_tables() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let cache = Cache::new(&db_path, 86400).await.unwrap();

    assert!(cache.table_exists("packages").await.unwrap());
    assert!(cache.table_exists("versions").await.unwrap());
}
