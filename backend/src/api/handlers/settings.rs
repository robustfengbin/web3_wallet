use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::middleware::AuthenticatedUser;
use crate::blockchain::ethereum::EthereumClient;
use crate::db::repositories::SettingsRepository;
use crate::error::{AppError, AppResult};

// Database keys for RPC settings
const RPC_PRIMARY_KEY: &str = "rpc_primary";
const RPC_FALLBACKS_KEY: &str = "rpc_fallbacks";

/// Preset RPC providers
#[derive(Debug, Clone, Serialize)]
pub struct RpcPreset {
    pub id: String,
    pub name: String,
    pub url_template: String,
    pub requires_api_key: bool,
    pub website: String,
}

/// Current RPC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    pub primary_rpc: String,
    pub fallback_rpcs: Vec<String>,
}

/// Load RPC configuration from database (for startup)
pub async fn load_rpc_config_from_db(
    settings_repo: &SettingsRepository,
    default_rpc: &str,
    default_fallbacks: &[String],
) -> RpcConfig {
    let primary_rpc = settings_repo
        .get(RPC_PRIMARY_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| default_rpc.to_string());

    let fallback_rpcs = settings_repo
        .get(RPC_FALLBACKS_KEY)
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_else(|| default_fallbacks.to_vec());

    tracing::info!("Loaded RPC config from database: {}", primary_rpc);

    RpcConfig {
        primary_rpc,
        fallback_rpcs,
    }
}

/// Get list of preset RPC providers
pub async fn get_rpc_presets() -> AppResult<HttpResponse> {
    let presets = vec![
        RpcPreset {
            id: "alchemy".to_string(),
            name: "Alchemy".to_string(),
            url_template: "https://eth-mainnet.g.alchemy.com/v2/{API_KEY}".to_string(),
            requires_api_key: true,
            website: "https://alchemy.com".to_string(),
        },
        RpcPreset {
            id: "infura".to_string(),
            name: "Infura".to_string(),
            url_template: "https://mainnet.infura.io/v3/{API_KEY}".to_string(),
            requires_api_key: true,
            website: "https://infura.io".to_string(),
        },
        RpcPreset {
            id: "quicknode".to_string(),
            name: "QuickNode".to_string(),
            url_template: "https://your-endpoint.quiknode.pro/{API_KEY}".to_string(),
            requires_api_key: true,
            website: "https://quicknode.com".to_string(),
        },
        RpcPreset {
            id: "ankr".to_string(),
            name: "Ankr (Free)".to_string(),
            url_template: "https://rpc.ankr.com/eth".to_string(),
            requires_api_key: false,
            website: "https://ankr.com".to_string(),
        },
        RpcPreset {
            id: "publicnode".to_string(),
            name: "PublicNode (Free)".to_string(),
            url_template: "https://ethereum.publicnode.com".to_string(),
            requires_api_key: false,
            website: "https://publicnode.com".to_string(),
        },
        RpcPreset {
            id: "llamarpc".to_string(),
            name: "LlamaRPC (Free)".to_string(),
            url_template: "https://eth.llamarpc.com".to_string(),
            requires_api_key: false,
            website: "https://llamarpc.com".to_string(),
        },
        RpcPreset {
            id: "cloudflare".to_string(),
            name: "Cloudflare (Free)".to_string(),
            url_template: "https://cloudflare-eth.com".to_string(),
            requires_api_key: false,
            website: "https://cloudflare.com".to_string(),
        },
        RpcPreset {
            id: "onerpc".to_string(),
            name: "1RPC (Free)".to_string(),
            url_template: "https://1rpc.io/eth".to_string(),
            requires_api_key: false,
            website: "https://1rpc.io".to_string(),
        },
    ];

    Ok(HttpResponse::Ok().json(presets))
}

/// Get current RPC configuration
pub async fn get_rpc_config(
    settings_repo: web::Data<Arc<SettingsRepository>>,
    eth_client: web::Data<Arc<EthereumClient>>,
) -> AppResult<HttpResponse> {
    // Get the actual current RPC from EthereumClient (runtime value)
    let current_rpc = eth_client.get_current_rpc().await;

    // Get fallbacks from database
    let fallback_rpcs = settings_repo
        .get(RPC_FALLBACKS_KEY)
        .await?
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default();

    Ok(HttpResponse::Ok().json(RpcConfig {
        primary_rpc: current_rpc,
        fallback_rpcs,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateRpcConfigRequest {
    pub primary_rpc: String,
    pub fallback_rpcs: Option<Vec<String>>,
}

/// Update RPC configuration
pub async fn update_rpc_config(
    settings_repo: web::Data<Arc<SettingsRepository>>,
    eth_client: web::Data<Arc<EthereumClient>>,
    user: AuthenticatedUser,
    request: web::Json<UpdateRpcConfigRequest>,
) -> AppResult<HttpResponse> {
    // Only admin can update RPC config
    if user.role != "admin" {
        return Err(AppError::Forbidden("Only admin can update RPC configuration".to_string()));
    }

    // Validate URL format
    if !request.primary_rpc.starts_with("http://") && !request.primary_rpc.starts_with("https://") {
        return Err(AppError::ValidationError("Invalid RPC URL format".to_string()));
    }

    // Update EthereumClient dynamically (no restart needed)
    eth_client
        .update_rpc(request.primary_rpc.clone(), request.fallback_rpcs.clone())
        .await?;

    // Save to database for persistence across restarts
    settings_repo.set(RPC_PRIMARY_KEY, &request.primary_rpc).await?;

    if let Some(fallbacks) = &request.fallback_rpcs {
        let fallbacks_json = serde_json::to_string(fallbacks)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize fallbacks: {}", e)))?;
        settings_repo.set(RPC_FALLBACKS_KEY, &fallbacks_json).await?;
    }

    tracing::info!("RPC configuration saved to database and applied: {}", request.primary_rpc);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "RPC configuration updated and applied immediately.",
        "restart_required": false
    })))
}

#[derive(Debug, Deserialize)]
pub struct TestRpcRequest {
    pub rpc_url: String,
}

#[derive(Debug, Serialize)]
pub struct TestRpcResponse {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub block_number: Option<u64>,
    pub error: Option<String>,
}

/// Test an RPC endpoint
pub async fn test_rpc_endpoint(
    request: web::Json<TestRpcRequest>,
) -> AppResult<HttpResponse> {
    use ethers::prelude::*;
    use std::time::Instant;

    let start = Instant::now();

    let result = async {
        let provider = Provider::<Http>::try_from(&request.rpc_url)
            .map_err(|e| format!("Failed to create provider: {}", e))?;

        let block_number = provider
            .get_block_number()
            .await
            .map_err(|e| format!("Failed to get block number: {}", e))?;

        Ok::<u64, String>(block_number.as_u64())
    }
    .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    let response = match result {
        Ok(block_number) => TestRpcResponse {
            success: true,
            latency_ms: Some(latency_ms),
            block_number: Some(block_number),
            error: None,
        },
        Err(e) => TestRpcResponse {
            success: false,
            latency_ms: Some(latency_ms),
            block_number: None,
            error: Some(e),
        },
    };

    Ok(HttpResponse::Ok().json(response))
}
