use serde::Deserialize;

use actix_web::{
    error, web, App, HttpRequest, HttpServer, Result,
};
use actix_web::middleware::Logger;

use actix_web_httpauth::middleware::HttpAuthentication;

use actix_cors::Cors;

use crate::AppState;
use crate::auth::{self, User};
use crate::access_control::{Permission};
use crate::index::*;


pub async fn run_server(state: AppState) -> crate::Result<()> {
    let state = web::Data::new(state);

    HttpServer::new({
        let state = state.clone();
        move || {
            App::new()
                .wrap(HttpAuthentication::basic(auth::validator))
                .wrap(Logger::default())
                .wrap(Cors::permissive())
                .app_data(state.clone())
                .configure(config_routes)
        }
    })
        .bind(state.config.api.listen)?
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
    web::Json(index_conf): web::Json<IndexConfig>,
) -> Result<String> {
    state.access_control.check_index_access(&user, &index_name, &Permission::WRITE)?;

    let res = state
        .indicies
        .create_index(index_name, index_conf, &state.config.search)
        .await
        .map(|_| "ok".into())
        .unwrap_or_else(|e| format!("error: {}", e.to_string()));
    Ok(res)
}

async fn delete_index(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
) -> Result<String> {
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
) -> Result<String> {
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

async fn _delete_by_term(
    _state: web::Data<AppState>,
    web::Path((_index_name,)): web::Path<(String,)>,
) -> Result<()> {
    todo!()
}

async fn search_documents(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    query: web::Query<SearchRequest>,
) -> Result<String> {
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
