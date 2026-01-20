import React, { useState, useRef, useEffect } from 'react';
import { ChevronDown, Check } from 'lucide-react';

// Token configuration with chain info and icons
export interface TokenConfig {
  symbol: string;
  name: string;
  chain: string;
  chainName: string;
  icon: string; // emoji or image URL
  chainIcon: string;
  color: string; // background color for icon
}

export const TOKENS: TokenConfig[] = [
  {
    symbol: 'ETH',
    name: 'Ethereum',
    chain: 'ethereum',
    chainName: 'Ethereum Mainnet',
    icon: '⟠',
    chainIcon: '⟠',
    color: '#627EEA',
  },
  {
    symbol: 'USDT',
    name: 'Tether USD',
    chain: 'ethereum',
    chainName: 'Ethereum Mainnet',
    icon: '₮',
    chainIcon: '⟠',
    color: '#26A17B',
  },
  {
    symbol: 'USDC',
    name: 'USD Coin',
    chain: 'ethereum',
    chainName: 'Ethereum Mainnet',
    icon: '$',
    chainIcon: '⟠',
    color: '#2775CA',
  },
  {
    symbol: 'DAI',
    name: 'Dai Stablecoin',
    chain: 'ethereum',
    chainName: 'Ethereum Mainnet',
    icon: '◈',
    chainIcon: '⟠',
    color: '#F5AC37',
  },
  {
    symbol: 'WETH',
    name: 'Wrapped Ether',
    chain: 'ethereum',
    chainName: 'Ethereum Mainnet',
    icon: '⟠',
    chainIcon: '⟠',
    color: '#EC4899',
  },
  {
    symbol: 'ZEC',
    name: 'Zcash',
    chain: 'zcash',
    chainName: 'Zcash Mainnet',
    icon: 'Ⓩ',
    chainIcon: 'Ⓩ',
    color: '#F4B728',
  },
];

interface TokenSelectorProps {
  selectedToken: string;
  selectedChain: string;
  onSelect: (token: string, chain: string) => void;
  balance?: string;
  disabled?: boolean;
}

export function TokenSelector({
  selectedToken,
  selectedChain,
  onSelect,
  balance,
  disabled = false,
}: TokenSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const selectedConfig = TOKENS.find(
    (t) => t.symbol === selectedToken && t.chain === selectedChain
  ) || TOKENS[0];

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSelect = (token: TokenConfig) => {
    onSelect(token.symbol, token.chain);
    setIsOpen(false);
  };

  return (
    <div className="relative" ref={dropdownRef}>
      {/* Selected Token Display */}
      <button
        type="button"
        onClick={() => !disabled && setIsOpen(!isOpen)}
        disabled={disabled}
        className={`w-full flex items-center justify-between px-3 py-2.5 border rounded-lg transition-colors ${
          disabled
            ? 'bg-gray-100 cursor-not-allowed'
            : 'bg-white hover:border-blue-400 cursor-pointer'
        } ${isOpen ? 'border-blue-500 ring-2 ring-blue-100' : 'border-gray-300'}`}
      >
        <div className="flex items-center space-x-3">
          {/* Token Icon */}
          <div
            className="w-8 h-8 rounded-full flex items-center justify-center text-white font-bold text-sm"
            style={{ backgroundColor: selectedConfig.color }}
          >
            {selectedConfig.icon}
          </div>

          {/* Token Info */}
          <div className="text-left">
            <div className="flex items-center space-x-2">
              <span className="font-semibold text-gray-900">{selectedConfig.symbol}</span>
              {balance && (
                <span className="text-xs text-gray-400">
                  ({parseFloat(balance).toFixed(4)})
                </span>
              )}
            </div>
            <div className="flex items-center text-xs text-gray-500">
              <span className="mr-1">{selectedConfig.chainIcon}</span>
              <span>{selectedConfig.chainName}</span>
            </div>
          </div>
        </div>

        <ChevronDown
          className={`w-5 h-5 text-gray-400 transition-transform ${isOpen ? 'rotate-180' : ''}`}
        />
      </button>

      {/* Dropdown - using Portal-like positioning */}
      {isOpen && (
        <>
          {/* Backdrop to close on click outside */}
          <div
            className="fixed inset-0 z-40"
            onClick={() => setIsOpen(false)}
          />
          <div className="absolute z-50 w-full mt-1 bg-white border border-gray-200 rounded-lg shadow-xl max-h-52 overflow-y-auto">
            {TOKENS.map((token) => {
              const isSelected = token.symbol === selectedToken && token.chain === selectedChain;
              return (
                <button
                  key={`${token.chain}-${token.symbol}`}
                  type="button"
                  onClick={() => handleSelect(token)}
                  className={`w-full flex items-center justify-between px-3 py-2 hover:bg-gray-50 transition-colors border-b border-gray-100 last:border-b-0 ${
                    isSelected ? 'bg-blue-50' : ''
                  }`}
                >
                  <div className="flex items-center space-x-3">
                    {/* Token Icon */}
                    <div
                      className="w-7 h-7 rounded-full flex items-center justify-center text-white font-bold text-xs"
                      style={{ backgroundColor: token.color }}
                    >
                      {token.icon}
                    </div>

                    {/* Token Info */}
                    <div className="text-left">
                      <div className="flex items-center space-x-2">
                        <span className="font-semibold text-gray-900 text-sm">{token.symbol}</span>
                        <span className="text-xs text-gray-400">{token.name}</span>
                      </div>
                      <div className="flex items-center text-xs text-gray-500">
                        <span className="mr-1">{token.chainIcon}</span>
                        <span>{token.chainName}</span>
                      </div>
                    </div>
                  </div>

                  {isSelected && <Check className="w-4 h-4 text-blue-600" />}
                </button>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}

// Helper function to get token config
export function getTokenConfig(symbol: string, chain: string): TokenConfig | undefined {
  return TOKENS.find((t) => t.symbol === symbol && t.chain === chain);
}
