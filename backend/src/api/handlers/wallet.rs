use actix_web::{web, HttpResponse};
use std::sync::Arc;

use crate::api::middleware::AuthenticatedUser;
use crate::db::models::{CreateWalletRequest, ExportPrivateKeyRequest, ImportWalletRequest};
use crate::error::{AppError, AppResult};
use crate::services::{AuthService, WalletService};

pub async fn list_wallets(
    wallet_service: web::Data<Arc<WalletService>>,
    query: web::Query<ChainQuery>,
) -> AppResult<HttpResponse> {
    let wallets = if let Some(chain) = &query.chain {
        wallet_service.list_wallets_by_chain(chain).await?
    } else {
        wallet_service.list_wallets().await?
    };

    Ok(HttpResponse::Ok().json(wallets))
}

pub async fn create_wallet(
    wallet_service: web::Data<Arc<WalletService>>,
    user: AuthenticatedUser,
    request: web::Json<CreateWalletRequest>,
) -> AppResult<HttpResponse> {
    // Only admin can create wallets
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can create wallets".to_string()));
    }

    let wallet = wallet_service
        .create_wallet(&request.name, &request.chain)
        .await?;

    Ok(HttpResponse::Created().json(wallet))
}

pub async fn import_wallet(
    wallet_service: web::Data<Arc<WalletService>>,
    user: AuthenticatedUser,
    request: web::Json<ImportWalletRequest>,
) -> AppResult<HttpResponse> {
    // Only admin can import wallets
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can import wallets".to_string()));
    }

    let wallet = wallet_service
        .import_wallet(&request.name, &request.private_key, &request.chain)
        .await?;

    Ok(HttpResponse::Created().json(wallet))
}

pub async fn get_wallet(
    wallet_service: web::Data<Arc<WalletService>>,
    path: web::Path<i32>,
) -> AppResult<HttpResponse> {
    let wallet = wallet_service.get_wallet(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(wallet))
}

pub async fn get_balance(
    wallet_service: web::Data<Arc<WalletService>>,
    query: web::Query<BalanceQuery>,
) -> AppResult<HttpResponse> {
    let chain = query.chain.as_deref().unwrap_or("ethereum");
    let balance = wallet_service.get_balance(&query.address, chain).await?;
    Ok(HttpResponse::Ok().json(balance))
}

pub async fn set_active_wallet(
    wallet_service: web::Data<Arc<WalletService>>,
    user: AuthenticatedUser,
    path: web::Path<i32>,
) -> AppResult<HttpResponse> {
    // Only admin can set active wallet
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can set active wallet".to_string()));
    }

    wallet_service.set_active_wallet(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Wallet set as active"})))
}

pub async fn export_private_key(
    wallet_service: web::Data<Arc<WalletService>>,
    auth_service: web::Data<Arc<AuthService>>,
    user: AuthenticatedUser,
    path: web::Path<i32>,
    request: web::Json<ExportPrivateKeyRequest>,
) -> AppResult<HttpResponse> {
    // Only admin can export private keys
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can export private keys".to_string()));
    }

    // Verify password
    let valid = auth_service
        .verify_user_password(user.user_id, &request.password)
        .await?;

    if !valid {
        return Err(AppError::InvalidCredentials);
    }

    let private_key = wallet_service.export_private_key(path.into_inner()).await?;

    Ok(HttpResponse::Ok().json(crate::db::models::ExportPrivateKeyResponse {
        private_key,
        warning: "Keep this private key secure. Anyone with access to it can control your funds.".to_string(),
    }))
}

pub async fn delete_wallet(
    wallet_service: web::Data<Arc<WalletService>>,
    user: AuthenticatedUser,
    path: web::Path<i32>,
) -> AppResult<HttpResponse> {
    // Only admin can delete wallets
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can delete wallets".to_string()));
    }

    wallet_service.delete_wallet(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Wallet deleted"})))
}

#[derive(Debug, serde::Deserialize)]
pub struct ChainQuery {
    pub chain: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BalanceQuery {
    pub address: String,
    pub chain: Option<String>,
}
