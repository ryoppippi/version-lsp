use std::path::PathBuf;

/// Default refresh interval in milliseconds (24 hours)
pub const DEFAULT_REFRESH_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000;

/// Returns the path to the data directory for version-lsp.
/// Uses $XDG_DATA_HOME/version-lsp if XDG_DATA_HOME is set,
/// otherwise falls back to ~/.local/share/version-lsp,
/// or ./version-lsp if neither is available.
pub fn data_dir() -> PathBuf {
    data_dir_with_env(std::env::var("XDG_DATA_HOME").ok(), dirs::home_dir())
}

/// Returns the path to the database file.
pub fn db_path() -> PathBuf {
    data_dir().join("versions.db")
}

/// Returns the path to the log file.
pub fn log_path() -> PathBuf {
    data_dir().join("version-lsp.log")
}

fn data_dir_with_env(xdg_data_home: Option<String>, home_dir: Option<PathBuf>) -> PathBuf {
    let data_dir = xdg_data_home
        .map(PathBuf::from)
        .or_else(|| home_dir.map(|home| home.join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));

    data_dir.join("version-lsp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_with_env_uses_xdg_data_home_when_set() {
        let path = data_dir_with_env(
            Some("/tmp/test-data".to_string()),
            Some(PathBuf::from("/home/user")),
        );

        assert_eq!(path, PathBuf::from("/tmp/test-data/version-lsp"));
    }

    #[test]
    fn data_dir_with_env_falls_back_to_home_local_share() {
        let path = data_dir_with_env(None, Some(PathBuf::from("/home/user")));

        assert_eq!(path, PathBuf::from("/home/user/.local/share/version-lsp"));
    }

    #[test]
    fn data_dir_with_env_falls_back_to_current_dir_when_no_dirs_available() {
        let path = data_dir_with_env(None, None);
        assert_eq!(path, PathBuf::from("./version-lsp"));
    }
}
