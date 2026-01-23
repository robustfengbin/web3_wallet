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
  // Privacy features configuration (for Zcash)
  privacyFeatures?: PrivacyFeaturesConfig;
  // All supported address prefixes
  addressPrefixes?: AddressPrefixConfig[];
}

export interface PrivacyFeaturesConfig {
  /** Supports Orchard shielded pool */
  supportsOrchard: boolean;
  /** Supports Sapling shielded pool */
  supportsSapling: boolean;
  /** Default address type for new addresses */
  defaultAddressType: 'transparent' | 'unified';
  /** Enable privacy transfer UI */
  enablePrivacyTransfer: boolean;
}

export interface AddressPrefixConfig {
  type: 'transparent' | 'sapling' | 'unified';
  prefix: string;
  description: string;
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
    privacyFeatures: {
      supportsOrchard: true,
      supportsSapling: true,
      defaultAddressType: 'unified',
      enablePrivacyTransfer: true,
    },
    addressPrefixes: [
      { type: 'transparent', prefix: 't1', description: 'Transparent (public)' },
      { type: 'transparent', prefix: 't3', description: 'Transparent P2SH' },
      { type: 'sapling', prefix: 'zs', description: 'Sapling (shielded)' },
      { type: 'unified', prefix: 'u1', description: 'Unified (recommended)' },
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

// Check if a chain supports privacy features
export function supportsPrivacy(chainId: string): boolean {
  const chain = CHAINS[chainId];
  return chain?.privacyFeatures?.supportsOrchard === true;
}

// Check if a chain supports Orchard
export function supportsOrchard(chainId: string): boolean {
  const chain = CHAINS[chainId];
  return chain?.privacyFeatures?.supportsOrchard === true;
}

// Get the default address type for a chain
export function getDefaultAddressType(chainId: string): 'transparent' | 'unified' {
  const chain = CHAINS[chainId];
  return chain?.privacyFeatures?.defaultAddressType || 'transparent';
}

// Check if privacy transfer UI should be enabled
export function isPrivacyTransferEnabled(chainId: string): boolean {
  const chain = CHAINS[chainId];
  return chain?.privacyFeatures?.enablePrivacyTransfer === true;
}

// Get address type from address string
export function getAddressTypeFromAddress(chainId: string, address: string): string {
  const chain = CHAINS[chainId];
  if (!chain?.addressPrefixes) {
    return 'unknown';
  }

  for (const prefix of chain.addressPrefixes) {
    if (address.startsWith(prefix.prefix)) {
      return prefix.type;
    }
  }

  return 'unknown';
}

// Validate address format for a chain
export function validateAddressFormat(chainId: string, address: string): boolean {
  const chain = CHAINS[chainId];
  if (!chain) return false;

  // Check standard prefix
  if (address.startsWith(chain.addressPrefix)) {
    return true;
  }

  // Check all address prefixes
  if (chain.addressPrefixes) {
    return chain.addressPrefixes.some(p => address.startsWith(p.prefix));
  }

  return false;
}
