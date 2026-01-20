// Chain configuration - add new chains here
export interface ChainConfig {
  id: string;
  name: string;
  symbol: string;
  icon: string;
  color: string;
  addressPrefix: string;
  explorerUrl?: string;
  // Supported tokens on this chain (empty for non-EVM chains)
  tokens: TokenConfig[];
}

export interface TokenConfig {
  symbol: string;
  name: string;
  icon: string;
  color: string;
  decimals: number;
  contractAddress?: string;
}

// Chain definitions
export const CHAINS: Record<string, ChainConfig> = {
  ethereum: {
    id: 'ethereum',
    name: 'Ethereum',
    symbol: 'ETH',
    icon: '⟠',
    color: '#627EEA',
    addressPrefix: '0x',
    explorerUrl: 'https://etherscan.io',
    tokens: [
      { symbol: 'ETH', name: 'Ethereum', icon: '⟠', color: '#627EEA', decimals: 18 },
      { symbol: 'USDT', name: 'Tether USD', icon: '₮', color: '#26A17B', decimals: 6, contractAddress: '0xdAC17F958D2ee523a2206206994597C13D831ec7' },
      { symbol: 'USDC', name: 'USD Coin', icon: '$', color: '#2775CA', decimals: 6, contractAddress: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48' },
      { symbol: 'DAI', name: 'Dai Stablecoin', icon: '◈', color: '#F5AC37', decimals: 18, contractAddress: '0x6B175474E89094C44Da98b954EescdeCB5' },
      { symbol: 'WETH', name: 'Wrapped Ether', icon: '⟠', color: '#EC4899', decimals: 18, contractAddress: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2' },
    ],
  },
  zcash: {
    id: 'zcash',
    name: 'Zcash',
    symbol: 'ZEC',
    icon: 'Ⓩ',
    color: '#F4B728',
    addressPrefix: 't1',
    explorerUrl: 'https://mainnet.zcashexplorer.app',
    tokens: [
      { symbol: 'ZEC', name: 'Zcash', icon: 'Ⓩ', color: '#F4B728', decimals: 8 },
    ],
  },
  // Future chains can be added here:
  // solana: { ... },
  // bsc: { ... },
};

// Get chain by ID
export function getChain(chainId: string): ChainConfig | undefined {
  return CHAINS[chainId];
}

// Get all chain IDs
export function getChainIds(): string[] {
  return Object.keys(CHAINS);
}

// Get native token for a chain
export function getNativeToken(chainId: string): TokenConfig | undefined {
  const chain = CHAINS[chainId];
  return chain?.tokens[0];
}

// Get all tokens for a chain
export function getChainTokens(chainId: string): TokenConfig[] {
  return CHAINS[chainId]?.tokens || [];
}

// Check if a token is native
export function isNativeToken(chainId: string, symbol: string): boolean {
  const chain = CHAINS[chainId];
  return chain?.symbol === symbol;
}
