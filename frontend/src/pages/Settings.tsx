import React, { useState, useEffect } from 'react';
import { Key, AlertCircle, CheckCircle, Globe, Server, Zap, ExternalLink, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card, LoadingSpinner } from '../components/Common';
import { authService, settingsService } from '../services/api';
import { useAuth } from '../hooks/useAuth';
import type { RpcPreset, RpcConfig, TestRpcResponse } from '../services/api/settings';

const languages = [
  { code: 'zh', name: '中文' },
  { code: 'en', name: 'English' },
];

export function Settings() {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const [oldPassword, setOldPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  // RPC Configuration state
  const [rpcPresets, setRpcPresets] = useState<RpcPreset[]>([]);
  const [rpcConfig, setRpcConfig] = useState<RpcConfig | null>(null);
  const [selectedPreset, setSelectedPreset] = useState<string>('custom');
  const [customRpcUrl, setCustomRpcUrl] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [rpcLoading, setRpcLoading] = useState(false);
  const [rpcError, setRpcError] = useState('');
  const [rpcSuccess, setRpcSuccess] = useState('');
  const [testResult, setTestResult] = useState<TestRpcResponse | null>(null);
  const [isTesting, setIsTesting] = useState(false);

  // Load RPC configuration on mount
  useEffect(() => {
    loadRpcData();
  }, []);

  const loadRpcData = async () => {
    try {
      const [presets, config] = await Promise.all([
        settingsService.getRpcPresets(),
        settingsService.getRpcConfig(),
      ]);
      setRpcPresets(presets);
      setRpcConfig(config);
      setCustomRpcUrl(config.primary_rpc);

      // Try to match current config with a preset
      let matched = false;
      for (const preset of presets) {
        if (preset.requires_api_key) {
          // For presets requiring API key, check if URL starts with the template prefix
          const prefix = preset.url_template.replace('{API_KEY}', '');
          if (config.primary_rpc.startsWith(prefix)) {
            // Extract API key from the URL
            const extractedKey = config.primary_rpc.substring(prefix.length);
            setSelectedPreset(preset.id);
            setApiKey(extractedKey);
            matched = true;
            break;
          }
        } else {
          // For presets without API key, exact match
          if (preset.url_template === config.primary_rpc) {
            setSelectedPreset(preset.id);
            matched = true;
            break;
          }
        }
      }

      if (!matched) {
        setSelectedPreset('custom');
      }
    } catch (err) {
      console.error('Failed to load RPC config:', err);
    }
  };

  const getCurrentRpcUrl = (): string => {
    if (selectedPreset === 'custom') {
      return customRpcUrl;
    }
    const preset = rpcPresets.find(p => p.id === selectedPreset);
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
      await settingsService.updateRpcConfig({
        primary_rpc: rpcUrl,
        fallback_rpcs: rpcConfig?.fallback_rpcs,
      });
      setRpcSuccess(t('settings.rpc.saved'));
      setCustomRpcUrl(rpcUrl);
    } catch (err) {
      setRpcError(err instanceof Error ? err.message : 'Failed to save');
    } finally {
      setRpcLoading(false);
    }
  };

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setSuccess('');

    if (newPassword !== confirmPassword) {
      setError(t('settings.passwordMismatch'));
      return;
    }

    if (newPassword.length < 6) {
      setError(t('settings.passwordTooShort'));
      return;
    }

    setIsLoading(true);
    try {
      await authService.changePassword(oldPassword, newPassword);
      setSuccess(t('settings.passwordChanged'));
      setOldPassword('');
      setNewPassword('');
      setConfirmPassword('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to change password');
    } finally {
      setIsLoading(false);
    }
  };

  const handleLanguageChange = (langCode: string) => {
    i18n.changeLanguage(langCode);
  };

  return (
    <div>
      <h1 className="text-2xl font-bold text-gray-900 mb-6">{t('settings.title')}</h1>

      <div className="max-w-2xl space-y-6">
        {/* Language Settings */}
        <Card title={t('settings.languageSettings')}>
          <div className="space-y-3">
            <p className="text-sm text-gray-500">{t('settings.selectLanguage')}</p>
            <div className="flex space-x-3">
              {languages.map((lang) => (
                <button
                  key={lang.code}
                  onClick={() => handleLanguageChange(lang.code)}
                  className={`flex items-center px-4 py-2 rounded-lg border transition-colors ${
                    i18n.language === lang.code
                      ? 'border-blue-500 bg-blue-50 text-blue-700'
                      : 'border-gray-300 hover:border-gray-400'
                  }`}
                >
                  <Globe className="w-4 h-4 mr-2" />
                  {lang.name}
                </button>
              ))}
            </div>
          </div>
        </Card>

        {/* RPC Configuration - Admin only */}
        {user?.role === 'admin' && (
          <Card title={t('settings.rpc.title')}>
            <div className="space-y-4">
              <p className="text-sm text-gray-500">{t('settings.rpc.description')}</p>

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
                  {rpcPresets.map((preset) => (
                    <button
                      key={preset.id}
                      onClick={() => {
                        setSelectedPreset(preset.id);
                        setApiKey(''); // 清空API Key，不同节点类型使用不同的Key
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
              {selectedPreset !== 'custom' && rpcPresets.find(p => p.id === selectedPreset)?.requires_api_key && (
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
                      <div>{t('settings.rpc.blockNumber')}: <span className="font-medium">{testResult.block_number?.toLocaleString()}</span></div>
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
                  className="flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
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
        )}

        {/* User Info */}
        <Card title={t('settings.accountInfo')}>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-gray-500">{t('settings.username')}</span>
              <span className="font-medium">{user?.username}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-gray-500">{t('settings.role')}</span>
              <span className="px-2 py-1 bg-gray-100 text-gray-700 rounded">
                {user?.role === 'admin' ? t('common.admin') : t('common.operator')}
              </span>
            </div>
          </div>
        </Card>

        {/* Change Password */}
        <Card title={t('settings.changePassword')}>
          {error && (
            <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 flex items-center">
              <AlertCircle className="w-5 h-5 mr-2" />
              {error}
            </div>
          )}

          {success && (
            <div className="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg text-green-700 flex items-center">
              <CheckCircle className="w-5 h-5 mr-2" />
              {success}
            </div>
          )}

          <form onSubmit={handleChangePassword} className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.currentPassword')}
              </label>
              <input
                type="password"
                value={oldPassword}
                onChange={(e) => setOldPassword(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.newPassword')}
              </label>
              <input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.confirmPassword')}
              </label>
              <input
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                required
              />
            </div>

            <button
              type="submit"
              disabled={isLoading}
              className="flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
            >
              {isLoading ? (
                <LoadingSpinner size="sm" />
              ) : (
                <>
                  <Key className="w-4 h-4 mr-2" />
                  {t('settings.changePassword')}
                </>
              )}
            </button>
          </form>
        </Card>
      </div>
    </div>
  );
}
