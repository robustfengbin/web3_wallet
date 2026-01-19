use actix_web::{web, HttpResponse};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::middleware::AuthenticatedUser;
use crate::blockchain::ChainRegistry;
use crate::blockchain::traits::TransferParams;
use crate::db::models::TransferRequest;
use crate::error::{AppError, AppResult};
use crate::services::{TransferService, WalletService};

pub async fn initiate_transfer(
    transfer_service: web::Data<Arc<TransferService>>,
    user: AuthenticatedUser,
    request: web::Json<TransferRequest>,
) -> AppResult<HttpResponse> {
    // Only admin can initiate transfers
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can initiate transfers".to_string()));
    }

    let transfer = transfer_service
        .initiate_transfer(request.into_inner(), user.user_id)
        .await?;

    Ok(HttpResponse::Created().json(transfer))
}

pub async fn execute_transfer(
    transfer_service: web::Data<Arc<TransferService>>,
    user: AuthenticatedUser,
    path: web::Path<i32>,
) -> AppResult<HttpResponse> {
    // Only admin can execute transfers
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can execute transfers".to_string()));
    }

    let transfer = transfer_service.execute_transfer(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(transfer))
}

pub async fn get_transfer(
    transfer_service: web::Data<Arc<TransferService>>,
    path: web::Path<i32>,
) -> AppResult<HttpResponse> {
    let transfer = transfer_service.get_transfer(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(transfer))
}

pub async fn list_transfers(
    transfer_service: web::Data<Arc<TransferService>>,
    query: web::Query<TransferListQuery>,
) -> AppResult<HttpResponse> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let (transfers, total) = if let Some(wallet_id) = query.wallet_id {
        transfer_service
            .list_wallet_transfers(wallet_id, limit, offset)
            .await?
    } else {
        transfer_service.list_transfers(limit, offset).await?
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "transfers": transfers,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}

pub async fn list_chains(
    chain_registry: web::Data<Arc<ChainRegistry>>,
) -> AppResult<HttpResponse> {
    let chains = chain_registry.list_chains();
    Ok(HttpResponse::Ok().json(chains))
}

#[derive(Debug, serde::Deserialize)]
pub struct TransferListQuery {
    pub wallet_id: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct EstimateGasRequest {
    pub chain: String,
    pub to_address: String,
    pub token: String,
    pub amount: String,
}

#[derive(Debug, Serialize)]
pub struct EstimateGasResponse {
    pub gas_limit: u64,
    pub gas_price_gwei: Decimal,
    pub estimated_fee_eth: Decimal,
    pub estimated_fee_usd: Option<Decimal>,
    // EIP-1559 specific fields
    pub base_fee_gwei: Option<Decimal>,
    pub priority_fee_gwei: Option<Decimal>,
    pub max_fee_gwei: Option<Decimal>,
}

/// Estimate gas for a transfer
pub async fn estimate_gas(
    chain_registry: web::Data<Arc<ChainRegistry>>,
    wallet_service: web::Data<Arc<WalletService>>,
    _user: AuthenticatedUser,
    request: web::Json<EstimateGasRequest>,
) -> AppResult<HttpResponse> {
    // Get active wallet for the chain
    let wallet = wallet_service
        .get_active_wallet(&request.chain)
        .await?;

    // Get chain client
    let client = chain_registry.get(&request.chain)?;

    // Parse amount
    let amount: Decimal = request.amount.parse()
        .map_err(|_| AppError::ValidationError("Invalid amount".to_string()))?;

    // Create transfer params for estimation
    let params = TransferParams {
        from_address: wallet.address.clone(),
        to_address: request.to_address.clone(),
        token: request.token.clone(),
        amount,
        private_key: String::new(), // Not needed for estimation
        gas_price_gwei: None,
        gas_limit: None,
    };

    // Estimate gas
    let estimate = client.estimate_gas(&params).await?;

    Ok(HttpResponse::Ok().json(EstimateGasResponse {
        gas_limit: estimate.gas_limit,
        gas_price_gwei: estimate.gas_price_gwei,
        estimated_fee_eth: estimate.estimated_fee_eth,
        estimated_fee_usd: None, // TODO: Add price conversion
        base_fee_gwei: estimate.base_fee_gwei,
        priority_fee_gwei: estimate.priority_fee_gwei,
        max_fee_gwei: estimate.max_fee_gwei,
    }))
}
