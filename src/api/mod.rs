mod document;
mod index;
mod security;

use actix_cors::Cors;
use actix_web::error::JsonPayloadError;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde_json::json;

use crate::security::authc::authentication_handler;
use crate::AppState;
use document::{add_document, delete_by_term, search_documents};
use index::{create_index, delete_index};
use security::{add_user, list_users, remove_user};

pub async fn run_server(state: AppState) -> crate::Result<()> {
    let state = web::Data::new(state);

    HttpServer::new({
        let state = state.clone();
        move || {
            App::new()
                .wrap(HttpAuthentication::basic(authentication_handler))
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
