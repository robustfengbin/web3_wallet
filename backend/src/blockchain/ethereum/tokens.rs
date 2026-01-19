#![allow(dead_code)]

use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub symbol: String,
    pub name: String,
    pub contract_address: String,
    pub decimals: u8,
}

/// Supported ERC20 tokens on Ethereum mainnet
pub static SUPPORTED_TOKENS: Lazy<HashMap<String, TokenInfo>> = Lazy::new(|| {
    let mut tokens = HashMap::new();

    tokens.insert(
        "USDT".to_string(),
        TokenInfo {
            symbol: "USDT".to_string(),
            name: "Tether USD".to_string(),
            contract_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
            decimals: 6,
        },
    );

    tokens.insert(
        "USDC".to_string(),
        TokenInfo {
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            contract_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            decimals: 6,
        },
    );

    tokens.insert(
        "DAI".to_string(),
        TokenInfo {
            symbol: "DAI".to_string(),
            name: "Dai Stablecoin".to_string(),
            contract_address: "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(),
            decimals: 18,
        },
    );

    tokens.insert(
        "WETH".to_string(),
        TokenInfo {
            symbol: "WETH".to_string(),
            name: "Wrapped Ether".to_string(),
            contract_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            decimals: 18,
        },
    );

    tokens
});

/// Get token info by symbol
pub fn get_token_info(symbol: &str) -> Option<&TokenInfo> {
    SUPPORTED_TOKENS.get(&symbol.to_uppercase())
}
