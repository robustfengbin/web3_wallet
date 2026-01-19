#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{AppError, AppResult};

use super::traits::ChainClient;

/// Registry for managing multiple blockchain clients
pub struct ChainRegistry {
    chains: HashMap<String, Arc<dyn ChainClient>>,
}

impl ChainRegistry {
    pub fn new() -> Self {
        Self {
            chains: HashMap::new(),
        }
    }

    /// Register a new chain client
    pub fn register(&mut self, client: Arc<dyn ChainClient>) {
        let chain_id = client.chain_id().to_string();
        tracing::info!("Registering chain: {} ({})", client.chain_name(), chain_id);
        self.chains.insert(chain_id, client);
    }

    /// Get a chain client by ID
    pub fn get(&self, chain_id: &str) -> AppResult<Arc<dyn ChainClient>> {
        self.chains
            .get(chain_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Chain '{}' not supported", chain_id)))
    }

    /// List all registered chains
    pub fn list_chains(&self) -> Vec<ChainInfo> {
        self.chains
            .values()
            .map(|c| ChainInfo {
                id: c.chain_id().to_string(),
                name: c.chain_name().to_string(),
                native_token: c.native_token_symbol().to_string(),
            })
            .collect()
    }

    /// Check if a chain is registered
    pub fn has_chain(&self, chain_id: &str) -> bool {
        self.chains.contains_key(chain_id)
    }
}

impl Default for ChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChainInfo {
    pub id: String,
    pub name: String,
    pub native_token: String,
}
