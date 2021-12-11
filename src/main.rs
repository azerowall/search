mod api;
mod config;
mod dto;
mod error;
mod index;
mod index_config;
mod index_manager;
mod query;
mod security;
mod utils;

use crate::config::AppConfig;
use crate::index_manager::IndexManager;
use crate::security::{authc::AuthService, authz::PermissionsStorage};

pub use crate::error::Error;
pub type Result<T, E = crate::error::Error> = std::result::Result<T, E>;

/*
    TODO:
    commit каждую секунду
    тестирование - setup/teardown
    ленивая инициализация IndexReader и IndexWriter
    разделение Scheme и LocalIndex - Scheme может храниться даже если самого индекса на этой ноде нет.
    primary key ?
    кластер: шардинг, репликация
    шифрование трафика api
    шифрование трафика кластера
*/

pub struct AppState {
    pub config: AppConfig,
    pub indices: IndexManager,
    pub auth: AuthService,
    pub access_control: PermissionsStorage,
}

impl AppState {
    pub fn from_config(config: AppConfig) -> crate::Result<Self> {
        let search_conf = config.search.clone();
        let authc = AuthService::new(config.search.data_dir.join("users.json"))?;
        let authz = PermissionsStorage::new(config.search.data_dir.join("permissions.json"))?;
        Ok(Self {
            config,
            indices: IndexManager::new(search_conf)?,
            auth: authc,
            access_control: authz,
        })
    }
}

#[actix_web::main]
async fn main() -> crate::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    pretty_env_logger::init();

    let config = AppConfig::new()?;
    log::debug!("App config:\n{:#?}", &config);
    log::info!("API server at http://{}", config.api.listen);

    let state = AppState::from_config(config)?;
    api::run_server(state).await?;
    Ok(())
}
