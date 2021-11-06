use serde::Deserialize;

use actix_web::{
    error, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result as ActixResult,
};
use actix_web::middleware::Logger;

use actix_web_httpauth::middleware::HttpAuthentication;

use actix_cors::Cors;

use crate::auth::{self, AuthService, User};
use crate::access_control::{AccessControlSerivce, Permission};
use crate::index::*;
use crate::index_manager::IndexManager;



pub struct AppState {
    pub indicies: IndexManager,
    pub auth: AuthService,
    pub access_control: AccessControlSerivce,
}

impl AppState {
    pub async fn new() -> crate::Result<Self> {
        Ok(Self {
            indicies: IndexManager::load_from("/tmp/test".into()).await?,
            auth: AuthService::new_test(),
            access_control: AccessControlSerivce::new_test(),
        })
    }
}


pub async fn run_server() -> crate::Result<()> {
    let state = web::Data::new(AppState::new().await?);

    HttpServer::new(move || {
        App::new()
            .wrap(HttpAuthentication::basic(auth::validator))
            .wrap(Logger::default())
            .wrap(Cors::permissive())
            .app_data(state.clone())
            .configure(config_routes)
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
        .map_err(From::from)
}

fn config_routes(conf: &mut web::ServiceConfig) {
    conf.service(
        web::scope("/{index}")
            .service(
                web::resource("/")
                    .route(web::put().to(create_index))
                    .route(web::post().to(create_index))
                    .route(web::delete().to(delete_index)),
            )
            .service(
                web::resource("/_doc")
                    .route(web::get().to(search_documents))
                    .route(web::post().to(add_document)), //.route(web::post().to(delete_by_term))
            ),
    )
    .route("/", web::get().to(status));
}

async fn status(req: HttpRequest, user: User) -> String {
    format!("Tantivy version: {}\nuser: {:?}\nreq: {:#?}",
        tantivy::version_string(), user, req)
}

async fn create_index(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    web::Json(conf): web::Json<IndexConfig>,
) -> ActixResult<String> {
    state.access_control.check_index_access(&user, &index_name, &Permission::WRITE)?;

    let res = state
        .indicies
        .create_index(index_name, conf)
        .await
        .map(|_| "ok".into())
        .unwrap_or_else(|e| format!("error: {}", e.to_string()));
    Ok(res)
}

async fn delete_index(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
) -> ActixResult<String> {
    state.access_control.check_index_access(&user, &index_name, &Permission::WRITE)?;
    
    let res = state
        .indicies
        .delete_index(index_name)
        .await
        .map(|_| "ok".into())
        .unwrap_or_else(|e| format!("error: {}", e.to_string()));
    Ok(res)
}

#[derive(Deserialize)]
struct AddDocOptions {
    #[serde(default)]
    commit: bool,
}

async fn add_document(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    query: web::Query<AddDocOptions>,
    body: web::Bytes,
) -> ActixResult<String> {
    state.access_control.check_index_access(&user, &index_name, &Permission::WRITE)?;

    let index = state
        .indicies
        .index(&index_name)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let doc = String::from_utf8(body.to_vec()).map_err(error::ErrorInternalServerError)?;
    let req = AddDocRequest {
        doc,
        commit: query.commit,
    };
    index
        .add_document(req)
        .await
        .map_err(error::ErrorInternalServerError)?;
    Ok("ok".into())
}

async fn delete_by_term(
    _state: web::Data<AppState>,
    web::Path((_index_name,)): web::Path<(String,)>,
) -> crate::Result<()> {
    todo!()
}

async fn search_documents(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    query: web::Query<SearchRequest>,
) -> ActixResult<String> {
    state.access_control.check_index_access(&user, &index_name, &Permission::WRITE)?;

    let index = state
        .indicies
        .index(&index_name)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let docs = index
        .search(query.into_inner())
        .await
        .map_err(error::ErrorInternalServerError)?;

    let result = serde_json::to_string_pretty(&docs).map_err(error::ErrorInternalServerError)?;

    Ok(result)
}
