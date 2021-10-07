use std::collections::HashMap;
use std::default;
use std::sync::Arc;

use serde::{Deserialize};

use actix_web::{
    error, web,
    App, HttpRequest, HttpResponse, HttpServer, Responder, Result as ActixResult
};

use crate::index::*;
use crate::index_manager::IndexManager;


pub async fn run_server() -> crate::Result<()> {
    let state = web::Data::new(State::new().await?);

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .configure(config_routes)
            
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
    .map_err(From::from)
}

fn config_routes(conf: &mut web::ServiceConfig) {
    conf
        .service(web::scope("/{index}")
            .service(web::resource("/")
                .route(web::put().to(create_index))
                .route(web::post().to(create_index))
                .route(web::delete().to(delete_index))
            )
            .service(web::resource("/_doc")
                .route(web::get().to(search_documents))
                .route(web::post().to(add_document))
                //.route(web::post().to(delete_by_term))
            )
        );
}

pub struct State {
    indicies: IndexManager,
}

impl State {
    pub async fn new() -> crate::Result<Self> {
        Ok(Self {
            indicies: IndexManager::load_from("/tmp/test".into()).await?,
        })
    }
}


async fn status(_req: HttpRequest) -> String {
    format!("Tantivy version: {}", tantivy::version_string())
}

async fn create_index(
    state: web::Data<State>,
    web::Path((index_name,)): web::Path<(String,)>,
    web::Json(conf): web::Json<IndexConfig>,
)-> ActixResult<String> {
    let res = state.indicies.create_index(index_name, conf).await
        .map(|_| "ok".into())
        .unwrap_or_else(|e| format!("error: {}", e.to_string()));
    Ok(res)
}

async fn delete_index(
    state: web::Data<State>,
    web::Path((index_name,)): web::Path<(String,)>,
) -> ActixResult<String> {
    let res = state.indicies.delete_index(index_name).await
        .map(|_| "ok".into())
        .unwrap_or_else(|e| format!("error: {}", e.to_string()));
    Ok(res)
}

#[derive(Deserialize)]
struct AddDocOptions {
    #[serde(default)]
    commit: bool
}

async fn add_document(
    state: web::Data<State>,
    web::Path((index_name,)): web::Path<(String,)>,
    query: web::Query<AddDocOptions>,
    body: web::Bytes,
) -> ActixResult<String> {
    let index = state.indicies.index(&index_name).await
        .map_err(error::ErrorInternalServerError)?;

    let doc = String::from_utf8(body.to_vec())
        .map_err(error::ErrorInternalServerError)?;
    let req = AddDocRequest {
        doc,
        commit: query.commit,
    };
    index.add_document(req).await
        .map_err(error::ErrorInternalServerError)?;
    Ok("ok".into())
}


async fn delete_by_term(
    _state: web::Data<State>,
    web::Path((_index_name,)): web::Path<(String,)>
) -> crate::Result<()> {
    todo!()
}


async fn search_documents(
    state: web::Data<State>,
    web::Path((index_name,)): web::Path<(String,)>,
    query: web::Query<SearchRequest>
) -> ActixResult<String> {
    let index = state.indicies.index(&index_name).await
        .map_err(error::ErrorInternalServerError)?;

    let docs = index.search(query.into_inner()).await
        .map_err(error::ErrorInternalServerError)?;

    let result = serde_json::to_string_pretty(&docs)
        .map_err(error::ErrorInternalServerError)?;

    Ok(result)
}

async fn debug_request(req: HttpRequest) -> String {
    format!("{:#?}", req)
}


