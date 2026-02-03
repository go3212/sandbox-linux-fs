use std::env;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AppConfig {
    pub api_key: String,
    pub host: String,
    pub port: u16,
    pub data_dir: String,
    pub default_max_repo_size: u64,
    pub max_upload_size: u64,
    pub snapshot_interval_secs: u64,
    pub ttl_sweep_interval_secs: u64,
    pub command_timeout_secs: u64,
    pub command_max_output_bytes: usize,
    pub cache_max_bytes: u64,
    pub max_concurrent_commands: usize,
    pub log_level: String,
    pub cors_allowed_origins: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            api_key: env::var("API_KEY").expect("API_KEY must be set"),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()
                .expect("PORT must be a number"),
            data_dir: env::var("DATA_DIR").unwrap_or_else(|_| "/data".into()),
            default_max_repo_size: parse_env("DEFAULT_MAX_REPO_SIZE", 1_073_741_824),
            max_upload_size: parse_env("MAX_UPLOAD_SIZE", 104_857_600),
            snapshot_interval_secs: parse_env("SNAPSHOT_INTERVAL_SECS", 300),
            ttl_sweep_interval_secs: parse_env("TTL_SWEEP_INTERVAL_SECS", 60),
            command_timeout_secs: parse_env("COMMAND_TIMEOUT_SECS", 30),
            command_max_output_bytes: parse_env("COMMAND_MAX_OUTPUT_BYTES", 10_485_760),
            cache_max_bytes: parse_env("CACHE_MAX_BYTES", 268_435_456),
            max_concurrent_commands: parse_env("MAX_CONCURRENT_COMMANDS", 10),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()),
            cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "*".into()),
        }
    }

    pub fn repos_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.data_dir).join("repos")
    }

    pub fn metadata_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.data_dir).join("metadata")
    }

    pub fn snapshot_path(&self) -> std::path::PathBuf {
        self.metadata_dir().join("snapshot.bin")
    }

    pub fn wal_dir(&self) -> std::path::PathBuf {
        self.metadata_dir().join("wal")
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
