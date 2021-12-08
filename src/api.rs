use actix_web::error::JsonPayloadError;
use serde::Deserialize;

use actix_web::middleware::Logger;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};

use actix_web_httpauth::middleware::HttpAuthentication;

use actix_cors::Cors;
use serde_json::json;

use crate::access_control::Permission;
use crate::auth::{self, AddUserReq, User};
use crate::dto::*;
use crate::index_config::IndexConfig;
use crate::AppState;

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
    conf.service(web::resource("/").route(web::get().to(status)))
        .service(
            web::scope("/_users")
                .service(
                    web::resource("/")
                        .route(web::post().to(add_user))
                        .route(web::get().to(list_users)),
                )
                .service(web::resource("/{user}").route(web::delete().to(remove_user))),
        )
        .service(
            web::resource("/{index}")
                .route(web::put().to(create_index))
                .route(web::delete().to(delete_index)),
        )
        .service(
            web::scope("/{index}")
                .route("/", web::post().to(add_document))
                .route("/_search", web::get().to(search_documents))
                .route("/_delete_by_term", web::post().to(delete_by_term)),
        );
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

async fn add_user(
    state: web::Data<AppState>,
    _user: User,
    web::Json(new_user): web::Json<AddUserReq>,
) -> crate::Result<HttpResponse> {
    state.auth.add_user(new_user)?;
    Ok(HttpResponse::Ok().into())
}

async fn remove_user(
    state: web::Data<AppState>,
    _user: User,
    web::Path(user_name): web::Path<String>,
) -> crate::Result<HttpResponse> {
    state.auth.remove_user(&user_name)?;
    Ok(HttpResponse::Ok().into())
}

async fn list_users(state: web::Data<AppState>, _user: User) -> crate::Result<HttpResponse> {
    let users = state.auth.list_users()?;
    Ok(HttpResponse::Ok().json(users))
}

async fn create_index(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    web::Json(index_conf): web::Json<IndexConfig>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_index_access(&user, &index_name, &Permission::WRITE)?;

    state.indices.create_index(index_name, &index_conf).await?;
    Ok(HttpResponse::Ok().into())
}

async fn delete_index(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_index_access(&user, &index_name, &Permission::WRITE)?;

    state.indices.delete_index(&index_name).await?;
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
    state
        .access_control
        .check_index_access(&user, &index_name, &Permission::WRITE)?;

    let index = state.indices.index(&index_name).await?;

    let doc = String::from_utf8(body.to_vec())?;
    let req = AddDocReq {
        doc,
        commit: query.commit,
    };
    index.add_document(req).await?;
    Ok(HttpResponse::Ok().into())
}

async fn delete_by_term(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    req: web::Query<DeleteByTermReq>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_index_access(&user, &index_name, &Permission::WRITE)?;

    let index = state.indices.index(&index_name).await?;

    index.delete_by_term(req.into_inner()).await?;

    Ok(HttpResponse::Ok().finish())
}

async fn search_documents(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    query: web::Query<SearchReq>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_index_access(&user, &index_name, &Permission::WRITE)?;

    let index = state.indices.index(&index_name).await?;

    let docs = index.search(query.into_inner()).await?;

    Ok(HttpResponse::Ok().json(docs))
}
