use actix_web::{web, HttpResponse};

use crate::index_config::IndexConfig;
use crate::security::{authc::User, authz::SystemPrivileges};
use crate::AppState;

pub async fn create_index(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
    web::Json(index_conf): web::Json<IndexConfig>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_system(&user, SystemPrivileges::MANAGE_INDICES)?;
    state.indices.create_index(index_name, &index_conf).await?;
    Ok(HttpResponse::Ok().into())
}

pub async fn delete_index(
    state: web::Data<AppState>,
    user: User,
    web::Path((index_name,)): web::Path<(String,)>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_system(&user, SystemPrivileges::MANAGE_INDICES)?;
    state.indices.delete_index(&index_name).await?;
    Ok(HttpResponse::Ok().into())
}
