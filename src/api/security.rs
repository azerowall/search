use actix_web::{web, HttpResponse};

use crate::security::{
    authc::{AddUserReq, User},
    authz::{Permissions, SystemPrivileges},
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
    state.access_control.remove_user(&user_name);
    Ok(HttpResponse::Ok().into())
}

pub async fn list_users(state: web::Data<AppState>, user: User) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_system(&user, SystemPrivileges::MANAGE_SECURITY)?;
    let users = state.auth.list_users()?;
    Ok(HttpResponse::Ok().json(users))
}

pub async fn assign_permissions(
    state: web::Data<AppState>,
    user: User,
    target_user: web::Path<String>,
    perms: web::Json<Permissions>,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_system(&user, SystemPrivileges::MANAGE_SECURITY)?;
    state
        .access_control
        .assign_permissions(target_user.into_inner(), perms.into_inner())?;
    Ok(HttpResponse::Ok().into())
}

pub async fn list_users_permissions(
    state: web::Data<AppState>,
    user: User,
) -> crate::Result<HttpResponse> {
    state
        .access_control
        .check_system(&user, SystemPrivileges::MANAGE_SECURITY)?;
    let list = state.access_control.list_users_permissions();
    Ok(HttpResponse::Ok().json(list))
}
