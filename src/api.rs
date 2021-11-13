use actix_web::error::{JsonPayloadError};
use serde::Deserialize;

use actix_web::{
    web, App, HttpRequest, HttpResponse, HttpServer,
};
use actix_web::middleware::Logger;

use actix_web_httpauth::middleware::HttpAuthentication;

use actix_cors::Cors;
use serde_json::json;

use crate::AppState;
use crate::auth::{self, User};
use crate::access_control::{Permission};
use crate::index_config::IndexConfig;
use crate::dto::*;


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
                .app_data(web::JsonConfig::default().error_handler(error_handler))
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

fn error_handler(err: JsonPayloadError, _req: &HttpRequest) -> actix_web::Error {
    crate::error::value_parsing_err(err).into()
}

async fn status() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "tantivy_version": tantivy::version_string(),
        "tagline": "You Know, for Diploma!",
    }))
}

async fn create_index(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    web::Json(index_conf): web::Json<IndexConfig>,
) -> crate::Result<HttpResponse> {
    state.access_control.check_index_access(&user, &index_name, &Permission::WRITE)?;

    state
        .indicies
        .create_index(index_name, &index_conf, &state.config.search)
        .await?;
    Ok(HttpResponse::Ok().into())
}

async fn delete_index(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
) -> crate::Result<HttpResponse> {
    state.access_control.check_index_access(&user, &index_name, &Permission::WRITE)?;
    
    state
        .indicies
        .delete_index(index_name)
        .await?;
    Ok(HttpResponse::Ok().into())
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
) -> crate::Result<HttpResponse> {
    state.access_control.check_index_access(&user, &index_name, &Permission::WRITE)?;

    let index = state
        .indicies
        .index(&index_name)
        .await?;

    let doc = String::from_utf8(body.to_vec())?;
    let req = AddDocReq {
        doc,
        commit: query.commit,
    };
    index
        .add_document(req)
        .await?;
    Ok(HttpResponse::Ok().into())
}

async fn _delete_by_term(
    _state: web::Data<AppState>,
    web::Path((_index_name,)): web::Path<(String,)>,
) -> crate::Result<()> {
    todo!()
}

async fn search_documents(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    query: web::Query<SearchReq>,
) -> crate::Result<HttpResponse> {
    state.access_control.check_index_access(&user, &index_name, &Permission::WRITE)?;

    let index = state
        .indicies
        .index(&index_name)
        .await?;

    let docs = index
        .search(query.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(docs))
}
