use actix_web::{web, HttpResponse};

use crate::security::{
    authc::{AddUserReq, User},
    authz::SystemPrivileges,
};
use crate::AppState;

pub async fn add_user(
    state: web::Data<AppState>,
    user: User,
    web::Json(new_user): web::Json<AddUserReq>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_system(&user, SystemPrivileges::MANAGE_SECURITY)?;
    state.auth.add_user(new_user)?;
    Ok(HttpResponse::Ok().into())
}

pub async fn remove_user(
    state: web::Data<AppState>,
    user: User,
    web::Path(user_name): web::Path<String>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_system(&user, SystemPrivileges::MANAGE_SECURITY)?;
    state.auth.remove_user(&user_name)?;
    Ok(HttpResponse::Ok().into())
}

pub async fn list_users(state: web::Data<AppState>, user: User) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_system(&user, SystemPrivileges::MANAGE_SECURITY)?;
    let users = state.auth.list_users()?;
    Ok(HttpResponse::Ok().json(users))
}

pub async fn set_permissions(
    state: web::Data<AppState>,
    user: User,
) -> crate::Result<HttpResponse> {
    todo!()
}
