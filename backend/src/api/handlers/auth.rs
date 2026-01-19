use actix_web::{web, HttpResponse};
use std::sync::Arc;

use crate::api::middleware::AuthenticatedUser;
use crate::db::models::{ChangePasswordRequest, LoginRequest};
use crate::error::AppResult;
use crate::services::AuthService;

pub async fn login(
    auth_service: web::Data<Arc<AuthService>>,
    request: web::Json<LoginRequest>,
) -> AppResult<HttpResponse> {
    let response = auth_service.login(request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn logout() -> AppResult<HttpResponse> {
    // For stateless JWT, logout is handled client-side
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Logged out successfully"})))
}

pub async fn change_password(
    auth_service: web::Data<Arc<AuthService>>,
    user: AuthenticatedUser,
    request: web::Json<ChangePasswordRequest>,
) -> AppResult<HttpResponse> {
    auth_service
        .change_password(user.user_id, &request.old_password, &request.new_password)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Password changed successfully"})))
}

pub async fn me(
    auth_service: web::Data<Arc<AuthService>>,
    user: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let user_data = auth_service.get_user(user.user_id).await?;
    match user_data {
        Some(u) => Ok(HttpResponse::Ok().json(crate::db::models::UserResponse::from(u))),
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "User not found"}))),
    }
}
