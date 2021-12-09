use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::dto::*;
use crate::security::{authc::User, authz::IndexPrivileges};
use crate::AppState;

#[derive(Deserialize)]
pub struct AddDocOptions {
    #[serde(default)]
    commit: bool,
}

pub async fn add_document(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    query: web::Query<AddDocOptions>,
    body: web::Bytes,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_index(&user, &index_name, IndexPrivileges::WRITE)?;

    let index = state.indices.index(&index_name).await?;
    let doc = String::from_utf8(body.to_vec())?;
    let req = AddDocReq {
        doc,
        commit: query.commit,
    };
    index.add_document(req).await?;

    Ok(HttpResponse::Ok().into())
}

pub async fn delete_by_term(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    req: web::Query<DeleteByTermReq>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_index(&user, &index_name, IndexPrivileges::WRITE)?;

    let index = state.indices.index(&index_name).await?;
    index.delete_by_term(req.into_inner()).await?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn search_documents(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    query: web::Query<SearchReq>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_index(&user, &index_name, IndexPrivileges::READ)?;

    let index = state.indices.index(&index_name).await?;
    let docs = index.search(query.into_inner()).await?;

    Ok(HttpResponse::Ok().json(docs))
}
