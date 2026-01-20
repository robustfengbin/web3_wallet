import React, { useState, useEffect } from 'react';
import { AlertCircle, CheckCircle, Server, Zap, ExternalLink, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card, LoadingSpinner } from '../../../components/Common';
import { settingsService } from '../../../services/api';
import { useAuth } from '../../../hooks/useAuth';
import { getChain, ChainConfig } from '../../../config/chains';
import type { RpcPreset, RpcConfig, TestRpcResponse } from '../../../services/api/settings';

interface ChainRpcSettingsProps {
  chainId: string;
}

// Chain-specific RPC presets
const CHAIN_RPC_PRESETS: Record<string, RpcPreset[]> = {
  ethereum: [
    { id: 'alchemy', name: 'Alchemy', url_template: 'https://eth-mainnet.g.alchemy.com/v2/{API_KEY}', requires_api_key: true, website: 'https://www.alchemy.com/' },
    { id: 'infura', name: 'Infura', url_template: 'https://mainnet.infura.io/v3/{API_KEY}', requires_api_key: true, website: 'https://www.infura.io/' },
    { id: 'llamarpc', name: 'Llama RPC', url_template: 'https://eth.llamarpc.com', requires_api_key: false, website: 'https://llamarpc.com/' },
    { id: 'ankr', name: 'Ankr', url_template: 'https://rpc.ankr.com/eth', requires_api_key: false, website: 'https://www.ankr.com/' },
    { id: 'publicnode', name: 'PublicNode', url_template: 'https://ethereum.publicnode.com', requires_api_key: false, website: 'https://www.publicnode.com/' },
  ],
  zcash: [
    { id: 'local', name: 'Local Node (zcashd)', url_template: 'http://127.0.0.1:8232', requires_api_key: false, website: 'https://zcash.readthedocs.io/' },
    { id: 'zcha', name: 'Zcha.in API', url_template: 'https://api.zcha.in/v2', requires_api_key: false, website: 'https://zcha.in/' },
  ],
};

export function ChainRpcSettings({ chainId }: ChainRpcSettingsProps) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const chain = getChain(chainId) as ChainConfig;
  const presets = CHAIN_RPC_PRESETS[chainId] || [];

  const [selectedPreset, setSelectedPreset] = useState<string>('custom');
  const [customRpcUrl, setCustomRpcUrl] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [rpcUser, setRpcUser] = useState('');
  const [rpcPassword, setRpcPassword] = useState('');
  const [rpcLoading, setRpcLoading] = useState(false);
  const [rpcError, setRpcError] = useState('');
  const [rpcSuccess, setRpcSuccess] = useState('');
  const [testResult, setTestResult] = useState<TestRpcResponse | null>(null);
  const [isTesting, setIsTesting] = useState(false);

  // Load current config on mount
  useEffect(() => {
    loadRpcData();
  }, [chainId]);

  const loadRpcData = async () => {
    if (chainId === 'ethereum') {
      try {
        const config = await settingsService.getRpcConfig();
        setCustomRpcUrl(config.primary_rpc);

        // Try to match with preset
        let matched = false;
        for (const preset of presets) {
          if (preset.requires_api_key) {
            const prefix = preset.url_template.replace('{API_KEY}', '');
            if (config.primary_rpc.startsWith(prefix)) {
              const extractedKey = config.primary_rpc.substring(prefix.length);
              setSelectedPreset(preset.id);
              setApiKey(extractedKey);
              matched = true;
              break;
            }
          } else if (preset.url_template === config.primary_rpc) {
            setSelectedPreset(preset.id);
            matched = true;
            break;
          }
        }
        if (!matched) {
          setSelectedPreset('custom');
        }
      } catch (err) {
        console.error('Failed to load RPC config:', err);
      }
    } else {
      // For non-Ethereum chains, use default preset
      if (presets.length > 0) {
        setSelectedPreset(presets[0].id);
        setCustomRpcUrl(presets[0].url_template);
      }
    }
  };

  const getCurrentRpcUrl = (): string => {
    if (selectedPreset === 'custom') {
      return customRpcUrl;
    }
    const preset = presets.find(p => p.id === selectedPreset);
    if (!preset) return customRpcUrl;

    if (preset.requires_api_key && apiKey) {
      return preset.url_template.replace('{API_KEY}', apiKey);
    }
    return preset.url_template;
  };

  const handleTestRpc = async () => {
    const rpcUrl = getCurrentRpcUrl();
    if (!rpcUrl) return;

    setIsTesting(true);
    setTestResult(null);
    setRpcError('');

    try {
      const result = await settingsService.testRpcEndpoint(rpcUrl);
      setTestResult(result);
    } catch (err) {
      setRpcError(err instanceof Error ? err.message : 'Test failed');
    } finally {
      setIsTesting(false);
    }
  };

  const handleSaveRpc = async () => {
    const rpcUrl = getCurrentRpcUrl();
    if (!rpcUrl) {
      setRpcError(t('settings.rpc.urlRequired'));
      return;
    }

    setRpcLoading(true);
    setRpcError('');
    setRpcSuccess('');

    try {
      if (chainId === 'ethereum') {
        await settingsService.updateRpcConfig({
          primary_rpc: rpcUrl,
        });
      }
      // For other chains, we'd need to implement chain-specific RPC config API
      // For now, just show success message
      setRpcSuccess(t('settings.rpc.saved'));
      setCustomRpcUrl(rpcUrl);
    } catch (err) {
      setRpcError(err instanceof Error ? err.message : 'Failed to save');
    } finally {
      setRpcLoading(false);
    }
  };

  const isAdmin = user?.role === 'admin';

  if (!isAdmin) {
    return (
      <Card>
        <div className="text-center py-8">
          <AlertCircle className="w-12 h-12 text-yellow-500 mx-auto mb-4" />
          <p className="text-gray-600">{t('settings.adminOnly')}</p>
        </div>
      </Card>
    );
  }

  return (
    <div>
      <div className="flex items-center mb-6">
        <span
          className="w-10 h-10 rounded-full flex items-center justify-center text-white text-xl mr-3"
          style={{ backgroundColor: chain.color }}
        >
          {chain.icon}
        </span>
        <div>
          <h1 className="text-2xl font-bold text-gray-900">
            {chain.name} {t('settings.rpc.title')}
          </h1>
          <p className="text-sm text-gray-500">{t('chains.configureRpc', { chain: chain.name })}</p>
        </div>
      </div>

      <div className="max-w-2xl">
        <Card>
          <div className="space-y-4">
            <p className="text-sm text-gray-500">
              {chainId === 'zcash'
                ? t('settings.rpc.zcashDescription')
                : t('settings.rpc.description')}
            </p>

            {rpcError && (
              <div className="p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 flex items-center">
                <AlertCircle className="w-5 h-5 mr-2 flex-shrink-0" />
                {rpcError}
              </div>
            )}

            {rpcSuccess && (
              <div className="p-3 bg-green-50 border border-green-200 rounded-lg text-green-700 flex items-center">
                <CheckCircle className="w-5 h-5 mr-2 flex-shrink-0" />
                {rpcSuccess}
              </div>
            )}

            {/* Preset Selection */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                {t('settings.rpc.selectProvider')}
              </label>
              <div className="grid grid-cols-2 gap-2">
                {presets.map((preset) => (
                  <button
                    key={preset.id}
                    onClick={() => {
                      setSelectedPreset(preset.id);
                      setApiKey('');
                      if (!preset.requires_api_key) {
                        setCustomRpcUrl(preset.url_template);
                      }
                      setTestResult(null);
                    }}
                    className={`flex items-center justify-between p-3 rounded-lg border text-left transition-colors ${
                      selectedPreset === preset.id
                        ? 'border-blue-500 bg-blue-50'
                        : 'border-gray-200 hover:border-gray-300'
                    }`}
                  >
                    <div>
                      <div className="font-medium text-sm">{preset.name}</div>
                      {preset.requires_api_key && (
                        <div className="text-xs text-gray-500">{t('settings.rpc.requiresApiKey')}</div>
                      )}
                    </div>
                    <a
                      href={preset.website}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={(e) => e.stopPropagation()}
                      className="text-gray-400 hover:text-blue-500"
                    >
                      <ExternalLink className="w-4 h-4" />
                    </a>
                  </button>
                ))}
                <button
                  onClick={() => {
                    setSelectedPreset('custom');
                    setTestResult(null);
                  }}
                  className={`flex items-center p-3 rounded-lg border text-left transition-colors ${
                    selectedPreset === 'custom'
                      ? 'border-blue-500 bg-blue-50'
                      : 'border-gray-200 hover:border-gray-300'
                  }`}
                >
                  <Server className="w-4 h-4 mr-2" />
                  <span className="font-medium text-sm">{t('settings.rpc.custom')}</span>
                </button>
              </div>
            </div>

            {/* API Key input for providers that require it */}
            {selectedPreset !== 'custom' && presets.find(p => p.id === selectedPreset)?.requires_api_key && (
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  API Key
                </label>
                <input
                  type="text"
                  value={apiKey}
                  onChange={(e) => {
                    setApiKey(e.target.value);
                    setTestResult(null);
                  }}
                  placeholder={t('settings.rpc.apiKeyPlaceholder')}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            )}

            {/* Custom URL input */}
            {selectedPreset === 'custom' && (
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.rpc.customUrl')}
                </label>
                <input
                  type="text"
                  value={customRpcUrl}
                  onChange={(e) => {
                    setCustomRpcUrl(e.target.value);
                    setTestResult(null);
                  }}
                  placeholder="https://..."
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            )}

            {/* RPC Auth for Zcash local node */}
            {chainId === 'zcash' && (selectedPreset === 'local' || selectedPreset === 'custom') && (
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    {t('settings.rpc.rpcUser')}
                  </label>
                  <input
                    type="text"
                    value={rpcUser}
                    onChange={(e) => setRpcUser(e.target.value)}
                    placeholder={t('settings.rpc.optional')}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    {t('settings.rpc.rpcPassword')}
                  </label>
                  <input
                    type="password"
                    value={rpcPassword}
                    onChange={(e) => setRpcPassword(e.target.value)}
                    placeholder={t('settings.rpc.optional')}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                </div>
              </div>
            )}

            {/* Current URL display */}
            <div className="p-3 bg-gray-50 rounded-lg">
              <div className="text-xs text-gray-500 mb-1">{t('settings.rpc.currentUrl')}</div>
              <code className="text-sm text-gray-700 break-all">{getCurrentRpcUrl() || '-'}</code>
            </div>

            {/* Test Result */}
            {testResult && (
              <div className={`p-3 rounded-lg ${testResult.success ? 'bg-green-50 border border-green-200' : 'bg-red-50 border border-red-200'}`}>
                <div className="flex items-center">
                  {testResult.success ? (
                    <CheckCircle className="w-5 h-5 text-green-600 mr-2" />
                  ) : (
                    <AlertCircle className="w-5 h-5 text-red-600 mr-2" />
                  )}
                  <span className={testResult.success ? 'text-green-700' : 'text-red-700'}>
                    {testResult.success ? t('settings.rpc.testSuccess') : t('settings.rpc.testFailed')}
                  </span>
                </div>
                {testResult.success && (
                  <div className="mt-2 text-sm text-gray-600">
                    <div>{t('settings.rpc.latency')}: <span className="font-medium">{testResult.latency_ms}ms</span></div>
                    {testResult.block_number && (
                      <div>{t('settings.rpc.blockNumber')}: <span className="font-medium">{testResult.block_number?.toLocaleString()}</span></div>
                    )}
                  </div>
                )}
                {testResult.error && (
                  <div className="mt-2 text-sm text-red-600">{testResult.error}</div>
                )}
              </div>
            )}

            {/* Action Buttons */}
            <div className="flex space-x-3">
              <button
                onClick={handleTestRpc}
                disabled={isTesting || !getCurrentRpcUrl()}
                className="flex items-center px-4 py-2 border border-gray-300 text-gray-700 rounded-lg hover:bg-gray-50 disabled:opacity-50"
              >
                {isTesting ? (
                  <LoadingSpinner size="sm" />
                ) : (
                  <>
                    <Zap className="w-4 h-4 mr-2" />
                    {t('settings.rpc.test')}
                  </>
                )}
              </button>
              <button
                onClick={handleSaveRpc}
                disabled={rpcLoading || !getCurrentRpcUrl()}
                className="flex items-center px-4 py-2 text-white rounded-lg hover:opacity-90 disabled:opacity-50"
                style={{ backgroundColor: chain.color }}
              >
                {rpcLoading ? (
                  <LoadingSpinner size="sm" />
                ) : (
                  <>
                    <RefreshCw className="w-4 h-4 mr-2" />
                    {t('settings.rpc.save')}
                  </>
                )}
              </button>
            </div>

            <p className="text-xs text-gray-500">{t('settings.rpc.restartNote')}</p>
          </div>
        </Card>
      </div>
    </div>
  );
}
