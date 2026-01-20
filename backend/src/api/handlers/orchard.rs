//! Orchard Privacy Protocol API Handlers
//!
//! Handles API requests for Zcash Orchard shielded transfers.

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::middleware::AuthenticatedUser;
use crate::error::{AppError, AppResult};
use crate::services::WalletService;

/// Request to enable Orchard for a wallet
#[derive(Debug, Deserialize)]
pub struct EnableOrchardRequest {
    pub birthday_height: u64,
}

/// Response after enabling Orchard
#[derive(Debug, Serialize)]
pub struct EnableOrchardResponse {
    pub unified_address: UnifiedAddressInfo,
    pub viewing_key: String,
}

/// Unified address information
#[derive(Debug, Serialize)]
pub struct UnifiedAddressInfo {
    pub address: String,
    pub has_orchard: bool,
    pub has_sapling: bool,
    pub has_transparent: bool,
    pub transparent_address: Option<String>,
    pub address_index: u32,
    pub account_index: u32,
}

/// Shielded balance response
#[derive(Debug, Serialize)]
pub struct ShieldedBalanceResponse {
    pub total_zatoshis: u64,
    pub spendable_zatoshis: u64,
    pub pending_zatoshis: u64,
    pub note_count: u32,
    pub pool: String,
}

/// Combined balance response
#[derive(Debug, Serialize)]
pub struct CombinedBalanceResponse {
    pub wallet_id: i32,
    pub address: String,
    pub transparent_balance: String,
    pub shielded_balance: Option<ShieldedBalanceResponse>,
    pub total_zec: f64,
}

/// Scan progress response
#[derive(Debug, Serialize)]
pub struct ScanProgressResponse {
    pub chain: String,
    pub scan_type: String,
    pub last_scanned_height: u64,
    pub chain_tip_height: u64,
    pub progress_percent: f64,
    pub estimated_seconds_remaining: Option<u64>,
    pub is_scanning: bool,
    pub notes_found: u64,
}

/// Orchard transfer request
#[derive(Debug, Deserialize)]
pub struct OrchardTransferRequest {
    pub wallet_id: i32,
    pub to_address: String,
    pub amount: String,
    pub amount_zatoshis: Option<u64>,
    pub memo: Option<String>,
    pub target_pool: Option<String>,
}

/// Orchard transfer response
#[derive(Debug, Serialize)]
pub struct OrchardTransferResponse {
    pub transfer_id: i64,
    pub status: String,
    pub tx_hash: Option<String>,
}

/// Enable Orchard for a Zcash wallet
pub async fn enable_orchard(
    wallet_service: web::Data<Arc<WalletService>>,
    user: AuthenticatedUser,
    path: web::Path<i32>,
    request: web::Json<EnableOrchardRequest>,
) -> AppResult<HttpResponse> {
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can enable Orchard".to_string()));
    }

    let wallet_id = path.into_inner();

    let (unified_address, viewing_key) = wallet_service
        .enable_orchard(wallet_id, request.birthday_height)
        .await?;

    let response = EnableOrchardResponse {
        unified_address: UnifiedAddressInfo {
            address: unified_address.address,
            has_orchard: unified_address.has_orchard,
            has_sapling: unified_address.has_sapling,
            has_transparent: unified_address.has_transparent,
            transparent_address: unified_address.transparent_address,
            address_index: unified_address.address_index,
            account_index: unified_address.account_index,
        },
        viewing_key,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Get shielded balance for a wallet
pub async fn get_shielded_balance(
    wallet_service: web::Data<Arc<WalletService>>,
    path: web::Path<i32>,
) -> AppResult<HttpResponse> {
    let wallet_id = path.into_inner();
    let balance = wallet_service.get_shielded_balance(wallet_id).await?;

    let response = ShieldedBalanceResponse {
        total_zatoshis: balance.total_zatoshis,
        spendable_zatoshis: balance.spendable_zatoshis,
        pending_zatoshis: balance.pending_zatoshis,
        note_count: balance.note_count,
        pool: format!("{:?}", balance.pool).to_lowercase(),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Get combined balance (transparent + shielded)
pub async fn get_combined_balance(
    wallet_service: web::Data<Arc<WalletService>>,
    path: web::Path<i32>,
) -> AppResult<HttpResponse> {
    let wallet_id = path.into_inner();
    let balance = wallet_service.get_combined_zcash_balance(wallet_id).await?;

    let response = CombinedBalanceResponse {
        wallet_id: balance.wallet_id,
        address: balance.address,
        transparent_balance: balance.transparent_balance,
        shielded_balance: balance.shielded_balance.map(|b| ShieldedBalanceResponse {
            total_zatoshis: b.total_zatoshis,
            spendable_zatoshis: b.spendable_zatoshis,
            pending_zatoshis: b.pending_zatoshis,
            note_count: b.note_count,
            pool: format!("{:?}", b.pool).to_lowercase(),
        }),
        total_zec: balance.total_zec,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Get scan progress
pub async fn get_scan_progress(
    wallet_service: web::Data<Arc<WalletService>>,
) -> AppResult<HttpResponse> {
    let progress = wallet_service.get_scan_progress().await?;

    let response = ScanProgressResponse {
        chain: progress.chain,
        scan_type: progress.scan_type,
        last_scanned_height: progress.last_scanned_height,
        chain_tip_height: progress.chain_tip_height,
        progress_percent: progress.progress_percent,
        estimated_seconds_remaining: progress.estimated_seconds_remaining,
        is_scanning: progress.is_scanning,
        notes_found: progress.notes_found,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Trigger Orchard sync
pub async fn sync_orchard(
    wallet_service: web::Data<Arc<WalletService>>,
    user: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can trigger sync".to_string()));
    }

    let progress = wallet_service.sync_orchard().await?;

    let response = ScanProgressResponse {
        chain: progress.chain,
        scan_type: progress.scan_type,
        last_scanned_height: progress.last_scanned_height,
        chain_tip_height: progress.chain_tip_height,
        progress_percent: progress.progress_percent,
        estimated_seconds_remaining: progress.estimated_seconds_remaining,
        is_scanning: progress.is_scanning,
        notes_found: progress.notes_found,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Initiate an Orchard transfer
pub async fn initiate_orchard_transfer(
    _wallet_service: web::Data<Arc<WalletService>>,
    user: AuthenticatedUser,
    request: web::Json<OrchardTransferRequest>,
) -> AppResult<HttpResponse> {
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can initiate transfers".to_string()));
    }

    // TODO: Implement actual transfer initiation
    // For now, return a mock response
    let response = OrchardTransferResponse {
        transfer_id: 1,
        status: "pending".to_string(),
        tx_hash: None,
    };

    tracing::info!(
        "Orchard transfer initiated: wallet={}, to={}, amount={}",
        request.wallet_id,
        request.to_address,
        request.amount
    );

    Ok(HttpResponse::Ok().json(response))
}

/// Execute a pending Orchard transfer
pub async fn execute_orchard_transfer(
    _wallet_service: web::Data<Arc<WalletService>>,
    user: AuthenticatedUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can execute transfers".to_string()));
    }

    let transfer_id = path.into_inner();

    // TODO: Implement actual transfer execution
    // For now, return a mock response
    let response = OrchardTransferResponse {
        transfer_id,
        status: "submitted".to_string(),
        tx_hash: Some("mock_tx_hash_placeholder".to_string()),
    };

    tracing::info!("Orchard transfer executed: id={}", transfer_id);

    Ok(HttpResponse::Ok().json(response))
}
