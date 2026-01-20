import React, { useState, useEffect, useRef } from 'react';
import {
  Plus,
  Upload,
  Trash2,
  Key,
  CheckCircle,
  RefreshCw,
  Copy,
  Shield,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card, LoadingSpinner, Modal } from '../../../components/Common';
import { walletService } from '../../../services/api';
import orchardApi from '../../../services/api/orchard';
import { Wallet, BalanceResponse } from '../../../types';
import type { UnifiedAddressInfo } from '../../../types/orchard';
import { useAuth } from '../../../hooks/useAuth';
import { getChain, ChainConfig } from '../../../config/chains';

interface ChainWalletsProps {
  chainId: string;
}

export function ChainWallets({ chainId }: ChainWalletsProps) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const chain = getChain(chainId) as ChainConfig;

  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [balances, setBalances] = useState<Record<string, BalanceResponse>>({});
  const [unifiedAddresses, setUnifiedAddresses] = useState<Record<number, UnifiedAddressInfo>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  // Modal states
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showImportModal, setShowImportModal] = useState(false);
  const [showExportModal, setShowExportModal] = useState(false);
  const [showPrivacyAddressModal, setShowPrivacyAddressModal] = useState(false);
  const [selectedWalletId, setSelectedWalletId] = useState<number | null>(null);
  const [generatingPrivacyAddress, setGeneratingPrivacyAddress] = useState<number | null>(null);

  // Form states
  const [walletName, setWalletName] = useState('');
  const [privateKey, setPrivateKey] = useState('');
  const [password, setPassword] = useState('');
  const [exportedKey, setExportedKey] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const isLoadingRef = useRef(false);

  useEffect(() => {
    if (isLoadingRef.current) return;
    isLoadingRef.current = true;
    loadWallets().finally(() => {
      isLoadingRef.current = false;
    });
  }, [chainId]);

  const loadWallets = async () => {
    setIsLoading(true);
    try {
      const data = await walletService.listWallets(chainId);
      setWallets(data);

      // Load balances for all wallets
      const balancePromises = data.map((w) =>
        walletService.getBalance(w.address, w.chain).catch(() => null)
      );
      const results = await Promise.all(balancePromises);
      const newBalances: Record<string, BalanceResponse> = {};
      results.forEach((balance, index) => {
        if (balance) {
          newBalances[data[index].address] = balance;
        }
      });
      setBalances(newBalances);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load wallets');
    } finally {
      setIsLoading(false);
    }
  };

  const handleCreate = async () => {
    if (!walletName.trim()) return;
    setIsSubmitting(true);
    try {
      await walletService.createWallet({ name: walletName, chain: chainId });
      setSuccess(t('wallets.createSuccess'));
      setShowCreateModal(false);
      setWalletName('');
      loadWallets();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create wallet');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleImport = async () => {
    if (!walletName.trim() || !privateKey.trim()) return;
    setIsSubmitting(true);
    try {
      await walletService.importWallet({
        name: walletName,
        private_key: privateKey,
        chain: chainId,
      });
      setSuccess(t('wallets.importSuccess'));
      setShowImportModal(false);
      setWalletName('');
      setPrivateKey('');
      loadWallets();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to import wallet');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleExportKey = async () => {
    if (!selectedWalletId || !password.trim()) return;
    setIsSubmitting(true);
    try {
      const result = await walletService.exportPrivateKey(selectedWalletId, password);
      setExportedKey(result.private_key);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to export key');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleSetActive = async (id: number) => {
    try {
      await walletService.setActiveWallet(id);
      setSuccess(t('wallets.activeSuccess'));
      loadWallets();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to set active wallet');
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm(t('wallets.confirmDelete'))) return;
    try {
      await walletService.deleteWallet(id);
      setSuccess(t('wallets.deleteSuccess'));
      loadWallets();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete wallet');
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setSuccess(t('wallets.copiedToClipboard'));
  };

  // Generate privacy address for Zcash wallet
  const handleGeneratePrivacyAddress = async (walletId: number) => {
    setGeneratingPrivacyAddress(walletId);
    setError('');
    try {
      const response = await orchardApi.enableOrchard({
        wallet_id: walletId,
        birthday_height: 2000000, // TODO: Get current block height
      });
      setUnifiedAddresses((prev) => ({
        ...prev,
        [walletId]: response.unified_address,
      }));
      setSuccess(t('zcash.orchard.enableSuccess', 'Privacy address generated successfully!'));
    } catch (err: any) {
      setError(err.response?.data?.message || err.message || 'Failed to generate privacy address');
    } finally {
      setGeneratingPrivacyAddress(null);
    }
  };

  const isAdmin = user?.role === 'admin';

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <LoadingSpinner size="lg" />
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center">
          <span
            className="w-10 h-10 rounded-full flex items-center justify-center text-white text-xl mr-3"
            style={{ backgroundColor: chain.color }}
          >
            {chain.icon}
          </span>
          <div>
            <h1 className="text-2xl font-bold text-gray-900">
              {chain.name} {t('wallets.title')}
            </h1>
            <p className="text-sm text-gray-500">{t('chains.manageWallets', { chain: chain.name })}</p>
          </div>
        </div>
        {isAdmin && (
          <div className="flex space-x-3">
            <button
              onClick={() => setShowCreateModal(true)}
              className="flex items-center px-4 py-2 text-white rounded-lg hover:opacity-90"
              style={{ backgroundColor: chain.color }}
            >
              <Plus className="w-4 h-4 mr-2" />
              {t('wallets.createWallet')}
            </button>
            <button
              onClick={() => setShowImportModal(true)}
              className="flex items-center px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700"
            >
              <Upload className="w-4 h-4 mr-2" />
              {t('wallets.importWallet')}
            </button>
          </div>
        )}
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-700">
          {error}
          <button onClick={() => setError('')} className="ml-2 underline">
            {t('common.dismiss')}
          </button>
        </div>
      )}

      {success && (
        <div className="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg text-green-700">
          {success}
          <button onClick={() => setSuccess('')} className="ml-2 underline">
            {t('common.dismiss')}
          </button>
        </div>
      )}

      <div className="grid gap-4">
        {wallets.map((wallet) => (
          <Card key={wallet.id}>
            <div className="flex items-start justify-between">
              <div className="flex-1">
                <div className="flex items-center">
                  <h3 className="text-lg font-semibold text-gray-900">
                    {wallet.name}
                  </h3>
                  {wallet.is_active && (
                    <span className="ml-2 px-2 py-0.5 bg-green-100 text-green-800 text-xs rounded-full">
                      {t('common.active')}
                    </span>
                  )}
                </div>

                {/* Transparent Address */}
                <div className="mt-2 flex items-center">
                  <code className="text-sm text-gray-600 bg-gray-100 px-2 py-1 rounded">
                    {wallet.address}
                  </code>
                  <button
                    onClick={() => copyToClipboard(wallet.address)}
                    className="ml-2 text-gray-400 hover:text-gray-600"
                    title={t('common.copy')}
                  >
                    <Copy className="w-4 h-4" />
                  </button>
                </div>

                {/* Zcash Privacy Address Section */}
                {chainId === 'zcash' && (
                  <div className="mt-3">
                    {unifiedAddresses[wallet.id] ? (
                      // Show unified address if already generated
                      <div className="p-3 bg-green-50 border border-green-200 rounded-lg">
                        <div className="flex items-center gap-2 mb-2">
                          <Shield className="w-4 h-4 text-green-600" />
                          <span className="text-sm font-medium text-green-800">
                            {t('zcash.orchard.privacyAddress', 'Privacy Address (Unified)')}
                          </span>
                        </div>
                        <div className="flex items-center">
                          <code className="text-xs text-green-700 bg-green-100 px-2 py-1 rounded break-all">
                            {unifiedAddresses[wallet.id].address.slice(0, 30)}...
                          </code>
                          <button
                            onClick={() => copyToClipboard(unifiedAddresses[wallet.id].address)}
                            className="ml-2 text-green-600 hover:text-green-800"
                            title={t('common.copy')}
                          >
                            <Copy className="w-4 h-4" />
                          </button>
                        </div>
                      </div>
                    ) : (
                      // Show generate button if not generated
                      <button
                        onClick={() => handleGeneratePrivacyAddress(wallet.id)}
                        disabled={generatingPrivacyAddress === wallet.id}
                        className="flex items-center gap-2 px-3 py-2 text-sm bg-yellow-100 text-yellow-800 rounded-lg hover:bg-yellow-200 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                      >
                        {generatingPrivacyAddress === wallet.id ? (
                          <>
                            <LoadingSpinner size="sm" />
                            {t('zcash.orchard.generating', 'Generating...')}
                          </>
                        ) : (
                          <>
                            <Shield className="w-4 h-4" />
                            {t('zcash.orchard.generatePrivacyAddress', 'Generate Privacy Address')}
                          </>
                        )}
                      </button>
                    )}
                  </div>
                )}

                {balances[wallet.address] && (
                  <div className="mt-3">
                    <p className="text-sm text-gray-500">{t('wallets.balance')}</p>
                    <p className="text-lg font-semibold">
                      {parseFloat(balances[wallet.address].native_balance).toFixed(6)} {chain.symbol}
                    </p>
                    {balances[wallet.address].tokens.length > 0 && (
                      <div className="mt-1 flex flex-wrap gap-2">
                        {balances[wallet.address].tokens.map((token) => (
                          <span
                            key={token.symbol}
                            className="text-sm text-gray-600 bg-gray-100 px-2 py-0.5 rounded"
                          >
                            {parseFloat(token.balance).toFixed(2)} {token.symbol}
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>

              {isAdmin && (
                <div className="flex space-x-2">
                  <button
                    onClick={() => loadWallets()}
                    className="p-2 text-gray-400 hover:text-blue-600"
                    title={t('wallets.refreshBalance')}
                  >
                    <RefreshCw className="w-4 h-4" />
                  </button>
                  {!wallet.is_active && (
                    <button
                      onClick={() => handleSetActive(wallet.id)}
                      className="p-2 text-gray-400 hover:text-green-600"
                      title={t('wallets.setAsActive')}
                    >
                      <CheckCircle className="w-4 h-4" />
                    </button>
                  )}
                  <button
                    onClick={() => {
                      setSelectedWalletId(wallet.id);
                      setExportedKey('');
                      setPassword('');
                      setShowExportModal(true);
                    }}
                    className="p-2 text-gray-400 hover:text-yellow-600"
                    title={t('wallets.exportPrivateKey')}
                  >
                    <Key className="w-4 h-4" />
                  </button>
                  <button
                    onClick={() => handleDelete(wallet.id)}
                    className="p-2 text-gray-400 hover:text-red-600"
                    title={t('wallets.deleteWallet')}
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              )}
            </div>
          </Card>
        ))}

        {wallets.length === 0 && (
          <Card>
            <p className="text-center text-gray-500 py-8">
              {t('wallets.noWallets')}
            </p>
          </Card>
        )}
      </div>

      {/* Create Wallet Modal */}
      <Modal
        isOpen={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        title={`${t('wallets.createWallet')} - ${chain.name}`}
      >
        <div className="space-y-4">
          <div className="flex items-center p-3 bg-gray-50 rounded-lg">
            <span
              className="w-8 h-8 rounded-full flex items-center justify-center text-white mr-3"
              style={{ backgroundColor: chain.color }}
            >
              {chain.icon}
            </span>
            <div>
              <p className="font-medium">{chain.name}</p>
              <p className="text-sm text-gray-500">{t('chains.addressFormat')}: {chain.addressPrefix}...</p>
            </div>
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('wallets.walletName')}
            </label>
            <input
              type="text"
              value={walletName}
              onChange={(e) => setWalletName(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder={t('wallets.walletNamePlaceholder')}
            />
          </div>
          <div className="flex justify-end space-x-3">
            <button
              onClick={() => setShowCreateModal(false)}
              className="px-4 py-2 text-gray-600 hover:text-gray-800"
            >
              {t('common.cancel')}
            </button>
            <button
              onClick={handleCreate}
              disabled={isSubmitting || !walletName.trim()}
              className="px-4 py-2 text-white rounded-lg hover:opacity-90 disabled:opacity-50"
              style={{ backgroundColor: chain.color }}
            >
              {isSubmitting ? <LoadingSpinner size="sm" /> : t('wallets.create')}
            </button>
          </div>
        </div>
      </Modal>

      {/* Import Wallet Modal */}
      <Modal
        isOpen={showImportModal}
        onClose={() => setShowImportModal(false)}
        title={`${t('wallets.importWallet')} - ${chain.name}`}
      >
        <div className="space-y-4">
          <div className="flex items-center p-3 bg-gray-50 rounded-lg">
            <span
              className="w-8 h-8 rounded-full flex items-center justify-center text-white mr-3"
              style={{ backgroundColor: chain.color }}
            >
              {chain.icon}
            </span>
            <div>
              <p className="font-medium">{chain.name}</p>
              <p className="text-sm text-gray-500">{t('chains.addressFormat')}: {chain.addressPrefix}...</p>
            </div>
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('wallets.walletName')}
            </label>
            <input
              type="text"
              value={walletName}
              onChange={(e) => setWalletName(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder={t('wallets.walletNamePlaceholder')}
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('wallets.privateKey')}
            </label>
            <input
              type="password"
              value={privateKey}
              onChange={(e) => setPrivateKey(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
              placeholder={chainId === 'ethereum' ? '0x...' : t('wallets.privateKeyPlaceholder')}
            />
          </div>
          <div className="flex justify-end space-x-3">
            <button
              onClick={() => setShowImportModal(false)}
              className="px-4 py-2 text-gray-600 hover:text-gray-800"
            >
              {t('common.cancel')}
            </button>
            <button
              onClick={handleImport}
              disabled={isSubmitting || !walletName.trim() || !privateKey.trim()}
              className="px-4 py-2 text-white rounded-lg hover:opacity-90 disabled:opacity-50"
              style={{ backgroundColor: chain.color }}
            >
              {isSubmitting ? <LoadingSpinner size="sm" /> : t('wallets.import')}
            </button>
          </div>
        </div>
      </Modal>

      {/* Export Private Key Modal */}
      <Modal
        isOpen={showExportModal}
        onClose={() => {
          setShowExportModal(false);
          setExportedKey('');
          setPassword('');
        }}
        title={t('wallets.exportKeyTitle')}
      >
        <div className="space-y-4">
          {!exportedKey ? (
            <>
              <div className="p-3 bg-yellow-50 border border-yellow-200 rounded-lg text-yellow-800 text-sm">
                {t('wallets.exportKeyWarning')}
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('wallets.exportKeyConfirm')}
                </label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder={t('login.password')}
                />
              </div>
              <div className="flex justify-end space-x-3">
                <button
                  onClick={() => setShowExportModal(false)}
                  className="px-4 py-2 text-gray-600 hover:text-gray-800"
                >
                  {t('common.cancel')}
                </button>
                <button
                  onClick={handleExportKey}
                  disabled={isSubmitting || !password.trim()}
                  className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
                >
                  {isSubmitting ? <LoadingSpinner size="sm" /> : t('wallets.export')}
                </button>
              </div>
            </>
          ) : (
            <>
              <div className="p-3 bg-red-50 border border-red-200 rounded-lg text-red-800 text-sm">
                {t('wallets.keepSecure')}
              </div>
              <div className="p-3 bg-gray-100 rounded-lg">
                <code className="text-sm break-all">{exportedKey}</code>
              </div>
              <div className="flex justify-end space-x-3">
                <button
                  onClick={() => copyToClipboard(exportedKey)}
                  className="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700"
                >
                  {t('common.copy')}
                </button>
                <button
                  onClick={() => {
                    setShowExportModal(false);
                    setExportedKey('');
                    setPassword('');
                  }}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                  {t('common.done')}
                </button>
              </div>
            </>
          )}
        </div>
      </Modal>
    </div>
  );
}
