//#![allow(dead_code, unused_imports)]

mod config;
mod dto;
mod api;
mod auth;
mod access_control;
mod error;
mod index_config;
mod index;
mod index_manager;
mod query;

use crate::config::AppConfig;
use crate::auth::AuthService;
use crate::access_control::AccessControlSerivce;
use crate::index_manager::IndexManager;

pub use crate::error::Error;
pub type Result<T, E = crate::error::Error> = std::result::Result<T, E>;

/*
    TODO:
    IndexManager - RwLock внутри или снаружи?
    IndexWriter под Arc или весь LocalIndex под Arc?
    commit каждую секунду
    тестирование - setup/teardown
    конфиг
    dto
    ленивая инициализация IndexReader и IndexWriter
    разделение Scheme и LocalIndex - Scheme может храниться даже если самого индекса на этой ноде нет.
    primary key ?
    кластер: шардинг, репликация
    шифрование трафика api
    шифрование трафика кластера
    аутентификация/авторизация
    пользователи и права - а нужно ли?
*/


pub struct AppState {
    pub config: AppConfig,
    pub indicies: IndexManager,
    pub auth: AuthService,
    pub access_control: AccessControlSerivce,
}

impl AppState {
    pub fn from_config(config: AppConfig) -> crate::Result<Self> {
        let search_conf = config.search.clone();
        Ok(Self {
            config,
            indicies: IndexManager::new(search_conf)?,
            auth: AuthService::new_test(),
            access_control: AccessControlSerivce::new_test(),
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