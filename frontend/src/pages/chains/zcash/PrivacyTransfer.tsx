import React, { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Shield, Send, AlertTriangle, CheckCircle, ArrowRight, Info, Lock } from 'lucide-react';
import { Card } from '../../../components/Common/Card';
import { LoadingSpinner } from '../../../components/Common/LoadingSpinner';
import { BalanceBreakdown } from './components/BalanceBreakdown';
import { ScanProgress } from './components/ScanProgress';
import { MemoInput } from './components/MemoInput';
import orchardApi from '../../../services/api/orchard';
import { walletService } from '../../../services/api/wallet';
import type {
  CombinedZcashBalance,
  ScanProgress as ScanProgressType,
  OrchardTransferRequest,
} from '../../../types/orchard';
import { isUnifiedAddress, zecToZatoshis } from '../../../types/orchard';
import type { Wallet } from '../../../types';

export function PrivacyTransfer() {
  const { t } = useTranslation();

  // Wallet state
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [selectedWalletId, setSelectedWalletId] = useState<number | null>(null);
  const [balance, setBalance] = useState<CombinedZcashBalance | null>(null);
  const [scanProgress, setScanProgress] = useState<ScanProgressType | null>(null);
  const [isOrchardEnabled, setIsOrchardEnabled] = useState(false);

  // Form state
  const [toAddress, setToAddress] = useState('');
  const [amount, setAmount] = useState('');
  const [memo, setMemo] = useState('');

  // UI state
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [txHash, setTxHash] = useState<string | null>(null);

  // Load wallets on mount
  useEffect(() => {
    async function loadWallets() {
      try {
        const allWallets = await walletService.listWallets();
        const zcashWallets = allWallets.filter((w) => w.chain === 'zcash');
        setWallets(zcashWallets);

        // Select active wallet or first wallet
        const active = zcashWallets.find((w) => w.is_active) || zcashWallets[0];
        if (active) {
          setSelectedWalletId(active.id);
        }
      } catch (err: any) {
        setError(err.message || 'Failed to load wallets');
      } finally {
        setLoading(false);
      }
    }
    loadWallets();
  }, []);

  // Load balance when wallet changes
  useEffect(() => {
    async function loadBalance() {
      if (!selectedWalletId) return;

      try {
        // Check if Orchard is enabled
        const enabled = await orchardApi.isOrchardEnabled(selectedWalletId);
        setIsOrchardEnabled(enabled);

        if (enabled) {
          // Get combined balance
          const combinedBalance = await orchardApi.getCombinedBalance(selectedWalletId);
          setBalance(combinedBalance);

          // Get scan progress
          const progress = await orchardApi.getScanProgress();
          setScanProgress(progress);
        } else {
          // Get just transparent balance
          const wallet = wallets.find((w) => w.id === selectedWalletId);
          if (wallet) {
            const balanceResponse = await walletService.getBalance(wallet.address, 'zcash');
            setBalance({
              wallet_id: selectedWalletId,
              address: wallet.address,
              transparent_balance: balanceResponse.native_balance,
              shielded_balance: null,
              total_zec: parseFloat(balanceResponse.native_balance) || 0,
            });
          }
        }
      } catch (err: any) {
        console.error('Failed to load balance:', err);
      }
    }

    loadBalance();
  }, [selectedWalletId, wallets]);

  // Handle sync
  const handleSync = useCallback(async () => {
    try {
      const progress = await orchardApi.syncOrchard();
      setScanProgress(progress);
    } catch (err: any) {
      setError(err.message || 'Sync failed');
    }
  }, []);

  // Handle enable Orchard
  const handleEnableOrchard = async () => {
    if (!selectedWalletId) return;

    setSubmitting(true);
    setError(null);

    try {
      // Get current block height (birthday)
      const response = await orchardApi.enableOrchard({
        wallet_id: selectedWalletId,
        birthday_height: 2000000, // Should be current block height
      });

      setIsOrchardEnabled(true);
      setSuccess(t('zcash.orchard.enableSuccess', 'Orchard enabled successfully!'));

      // Reload balance
      const combinedBalance = await orchardApi.getCombinedBalance(selectedWalletId);
      setBalance(combinedBalance);
    } catch (err: any) {
      setError(err.message || 'Failed to enable Orchard');
    } finally {
      setSubmitting(false);
    }
  };

  // Validate form
  const validateForm = (): string | null => {
    if (!toAddress) {
      return t('zcash.orchard.errors.addressRequired', 'Recipient address is required');
    }

    if (!isUnifiedAddress(toAddress)) {
      return t(
        'zcash.orchard.errors.invalidAddress',
        'Please enter a valid unified address (u1...)'
      );
    }

    if (!amount || parseFloat(amount) <= 0) {
      return t('zcash.orchard.errors.amountRequired', 'Please enter a valid amount');
    }

    const amountZec = parseFloat(amount);
    // Can use both transparent and shielded balance for privacy transfers
    // Transparent -> Shielded (shielding) is supported
    const transparentZec = parseFloat(balance?.transparent_balance || '0');
    const shieldedZec = (balance?.shielded_balance?.spendable_zatoshis || 0) / 100_000_000;
    const totalAvailable = transparentZec + shieldedZec;

    if (amountZec > totalAvailable) {
      return t('zcash.orchard.errors.insufficientBalance', 'Insufficient balance');
    }

    return null;
  };

  // Handle submit
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const validationError = validateForm();
    if (validationError) {
      setError(validationError);
      return;
    }

    if (!selectedWalletId) return;

    setSubmitting(true);
    setError(null);
    setSuccess(null);
    setTxHash(null);

    try {
      const request: OrchardTransferRequest = {
        wallet_id: selectedWalletId,
        to_address: toAddress,
        amount: amount,
        amount_zatoshis: zecToZatoshis(parseFloat(amount)),
        memo: memo || undefined,
      };

      // Initiate transfer
      const initiateResponse = await orchardApi.initiateOrchardTransfer(request);

      // Execute transfer
      const executeResponse = await orchardApi.executeOrchardTransfer(
        initiateResponse.transfer_id
      );

      setTxHash(executeResponse.tx_hash);
      setSuccess(t('zcash.orchard.transferSuccess', 'Shielded transfer submitted successfully!'));

      // Clear form
      setToAddress('');
      setAmount('');
      setMemo('');

      // Reload balance
      const combinedBalance = await orchardApi.getCombinedBalance(selectedWalletId);
      setBalance(combinedBalance);
    } catch (err: any) {
      setError(err.response?.data?.message || err.message || 'Transfer failed');
    } finally {
      setSubmitting(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <LoadingSpinner />
      </div>
    );
  }

  if (wallets.length === 0) {
    return (
      <Card>
        <div className="text-center py-8">
          <Shield className="w-12 h-12 text-gray-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-gray-900 mb-2">
            {t('zcash.orchard.noWallets', 'No Zcash Wallets')}
          </h3>
          <p className="text-gray-500">
            {t('zcash.orchard.createWalletPrompt', 'Create a Zcash wallet to use privacy transfers.')}
          </p>
        </div>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-3">
        <div className="p-3 bg-yellow-100 rounded-lg">
          <Shield className="w-6 h-6 text-yellow-600" />
        </div>
        <div>
          <h1 className="text-2xl font-bold text-gray-900">
            {t('zcash.orchard.title', 'Zcash Privacy Transfer')}
          </h1>
          <p className="text-gray-500">
            {t('zcash.orchard.subtitle', 'Send ZEC with full end-to-end privacy using Orchard')}
          </p>
        </div>
      </div>

      {/* Wallet Selector */}
      <Card>
        <label className="block text-sm font-medium text-gray-700 mb-2">
          {t('zcash.orchard.selectWallet', 'Select Wallet')}
        </label>
        <select
          value={selectedWalletId || ''}
          onChange={(e) => setSelectedWalletId(Number(e.target.value))}
          className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-yellow-500 focus:border-yellow-500"
        >
          {wallets.map((wallet) => (
            <option key={wallet.id} value={wallet.id}>
              {wallet.name} - {wallet.address.slice(0, 10)}...
            </option>
          ))}
        </select>
      </Card>

      {/* Enable Orchard Section - Show prominently at top when not enabled */}
      {!isOrchardEnabled && (
        <div className="bg-white rounded-lg shadow-md border-l-4 border-blue-500 p-6">
          <div className="flex items-start gap-4">
            <div className="p-3 bg-blue-100 rounded-full">
              <Lock className="w-6 h-6 text-blue-600" />
            </div>
            <div className="flex-1">
              <h3 className="text-lg font-semibold text-gray-900">
                {t('zcash.orchard.enableOrchard', 'Enable Privacy Features')}
              </h3>
              <p className="text-gray-600 mt-1">
                {t(
                  'zcash.orchard.enableDescription',
                  'Enable Orchard to receive and send private transactions. This will generate a unified address that supports both transparent and shielded transfers.'
                )}
              </p>
              <button
                onClick={handleEnableOrchard}
                disabled={submitting}
                className="mt-4 px-6 py-2.5 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed inline-flex items-center gap-2 transition-colors"
              >
                {submitting ? (
                  <>
                    <LoadingSpinner size="sm" />
                    {t('common.enabling', 'Enabling...')}
                  </>
                ) : (
                  <>
                    <Shield className="w-5 h-5" />
                    {t('zcash.orchard.enableButton', 'Enable Orchard')}
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Balance Breakdown */}
      <BalanceBreakdown balance={balance} />

      {/* Scan Progress */}
      {isOrchardEnabled && (
        <ScanProgress progress={scanProgress} onSync={handleSync} />
      )}

      {/* Transfer Form */}
      {isOrchardEnabled && (
        <Card>
          <h3 className="text-lg font-medium text-gray-900 mb-4 flex items-center gap-2">
            <Send className="w-5 h-5 text-yellow-600" />
            {t('zcash.orchard.sendPrivate', 'Send Private Transfer')}
          </h3>

          <form onSubmit={handleSubmit} className="space-y-4">
            {/* Recipient Address */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                {t('zcash.orchard.recipient', 'Recipient Address')}
              </label>
              <input
                type="text"
                value={toAddress}
                onChange={(e) => setToAddress(e.target.value)}
                placeholder="u1..."
                disabled={submitting}
                className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-yellow-500 focus:border-yellow-500 disabled:bg-gray-100 disabled:cursor-not-allowed font-mono text-sm"
              />
              <p className="mt-1 text-xs text-gray-500">
                {t('zcash.orchard.addressHint', 'Enter a unified address (u1...) for full privacy')}
              </p>
            </div>

            {/* Amount */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                {t('zcash.orchard.amount', 'Amount (ZEC)')}
              </label>
              <div className="relative">
                <input
                  type="number"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  placeholder="0.00000000"
                  step="0.00000001"
                  min="0"
                  disabled={submitting}
                  className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-yellow-500 focus:border-yellow-500 disabled:bg-gray-100 disabled:cursor-not-allowed font-mono"
                />
                <button
                  type="button"
                  onClick={() => {
                    // Use total balance (transparent + shielded)
                    const transparentZec = parseFloat(balance?.transparent_balance || '0');
                    const shieldedZec = (balance?.shielded_balance?.spendable_zatoshis || 0) / 100_000_000;
                    const maxZec = transparentZec + shieldedZec - 0.0001; // Leave some for fee
                    if (maxZec > 0) {
                      setAmount(maxZec.toFixed(8));
                    }
                  }}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-sm text-yellow-600 hover:text-yellow-700"
                >
                  {t('common.max', 'Max')}
                </button>
              </div>
              {balance && (
                <p className="mt-1 text-xs text-gray-500">
                  {t('zcash.orchard.availableTotal', 'Available: {{amount}} ZEC (transparent + shielded)', {
                    amount: (
                      parseFloat(balance.transparent_balance || '0') +
                      (balance.shielded_balance?.spendable_zatoshis || 0) / 100_000_000
                    ).toFixed(8),
                  })}
                </p>
              )}
            </div>

            {/* Memo */}
            <MemoInput value={memo} onChange={setMemo} disabled={submitting} />

            {/* Privacy Notice */}
            <div className="flex items-start gap-3 p-4 bg-green-50 rounded-lg border border-green-200">
              <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
              <div>
                <p className="text-sm font-medium text-green-800">
                  {t('zcash.orchard.privacyNotice', 'End-to-End Privacy Protected')}
                </p>
                <p className="text-xs text-green-700 mt-1">
                  {t(
                    'zcash.orchard.privacyDescription',
                    'This transaction uses the Orchard protocol with Halo 2 proofs. The sender, recipient, amount, and memo are all encrypted and hidden from public view.'
                  )}
                </p>
              </div>
            </div>

            {/* Error Message */}
            {error && (
              <div className="flex items-start gap-3 p-4 bg-red-50 rounded-lg border border-red-200">
                <AlertTriangle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
                <p className="text-sm text-red-700">{error}</p>
              </div>
            )}

            {/* Success Message */}
            {success && (
              <div className="flex items-start gap-3 p-4 bg-green-50 rounded-lg border border-green-200">
                <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
                <div>
                  <p className="text-sm font-medium text-green-800">{success}</p>
                  {txHash && (
                    <p className="text-xs text-green-700 mt-1 font-mono">
                      TX: {txHash}
                    </p>
                  )}
                </div>
              </div>
            )}

            {/* Submit Button */}
            <button
              type="submit"
              disabled={submitting || !toAddress || !amount}
              className="w-full py-3 bg-yellow-500 text-white rounded-lg font-medium hover:bg-yellow-600 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {submitting ? (
                <>
                  <LoadingSpinner />
                  {t('zcash.orchard.sending', 'Sending...')}
                </>
              ) : (
                <>
                  <Shield className="w-5 h-5" />
                  {t('zcash.orchard.sendPrivateButton', 'Send Private Transfer')}
                  <ArrowRight className="w-5 h-5" />
                </>
              )}
            </button>
          </form>
        </Card>
      )}
    </div>
  );
}
