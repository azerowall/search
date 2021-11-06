use std::path::{PathBuf};
use std::net::{SocketAddr};

use serde::Deserialize;
use config::{Config, ConfigError, File, Environment};

const APP_NAME: &'static str = "search";

#[derive(Debug, Deserialize)]
pub struct Api {
    pub listen: SocketAddr,
}

#[derive(Debug, Deserialize)]
pub struct Search {
    pub data_dir: PathBuf,
    pub indexer_heap_size: u64,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub api: Api,
    pub search: Search,
}


impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let mut config = Config::default();

        config.merge(File::with_name("config/default").required(true))?;
        config.merge(Environment::with_prefix(APP_NAME))?;
        config.try_into()
    }
}