import React from 'react';
import { useTranslation } from 'react-i18next';
import { Eye, Shield, Lock, TrendingUp } from 'lucide-react';
import type { CombinedZcashBalance } from '../../../../types/orchard';
import { formatZec } from '../../../../types/orchard';

interface BalanceBreakdownProps {
  balance: CombinedZcashBalance | null;
  loading?: boolean;
  error?: string | null;
}

export function BalanceBreakdown({
  balance,
  loading = false,
  error = null,
}: BalanceBreakdownProps) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <div className="animate-pulse space-y-4">
          <div className="h-4 bg-gray-200 rounded w-1/4"></div>
          <div className="h-8 bg-gray-200 rounded w-1/2"></div>
          <div className="grid grid-cols-2 gap-4">
            <div className="h-16 bg-gray-200 rounded"></div>
            <div className="h-16 bg-gray-200 rounded"></div>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-50 rounded-lg border border-red-200 p-6">
        <p className="text-red-600">{error}</p>
      </div>
    );
  }

  if (!balance) {
    return null;
  }

  const transparentZec = parseFloat(balance.transparent_balance) || 0;
  const shieldedZec = balance.shielded_balance
    ? balance.shielded_balance.total_zatoshis / 100_000_000
    : 0;
  const spendableZec = balance.shielded_balance
    ? balance.shielded_balance.spendable_zatoshis / 100_000_000
    : 0;
  const pendingZec = balance.shielded_balance
    ? balance.shielded_balance.pending_zatoshis / 100_000_000
    : 0;

  return (
    <div className="bg-white rounded-lg border border-gray-200 overflow-hidden">
      {/* Total Balance Header */}
      <div className="bg-gradient-to-r from-yellow-500 to-yellow-600 px-6 py-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-yellow-100 text-sm">
              {t('zcash.orchard.totalBalance', 'Total Balance')}
            </p>
            <p className="text-white text-3xl font-bold">
              {balance.total_zec.toFixed(8)} ZEC
            </p>
          </div>
          <TrendingUp className="w-8 h-8 text-yellow-200" />
        </div>
      </div>

      {/* Balance Breakdown */}
      <div className="p-6 space-y-4">
        {/* Transparent Balance */}
        <div className="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-red-100 rounded-lg">
              <Eye className="w-5 h-5 text-red-600" />
            </div>
            <div>
              <p className="font-medium text-gray-900">
                {t('zcash.orchard.transparentBalance', 'Transparent')}
              </p>
              <p className="text-xs text-gray-500">
                {t('zcash.orchard.transparentDesc', 'Publicly visible on blockchain')}
              </p>
            </div>
          </div>
          <p className="text-lg font-mono font-medium text-gray-900">
            {transparentZec.toFixed(8)} ZEC
          </p>
        </div>

        {/* Shielded Balance */}
        {balance.shielded_balance ? (
          <div className="space-y-2">
            <div className="flex items-center justify-between p-4 bg-green-50 rounded-lg">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-green-100 rounded-lg">
                  <Shield className="w-5 h-5 text-green-600" />
                </div>
                <div>
                  <p className="font-medium text-gray-900">
                    {t('zcash.orchard.shieldedBalance', 'Shielded (Orchard)')}
                  </p>
                  <p className="text-xs text-gray-500">
                    {t('zcash.orchard.shieldedDesc', 'Private and encrypted')}
                  </p>
                </div>
              </div>
              <p className="text-lg font-mono font-medium text-green-700">
                {shieldedZec.toFixed(8)} ZEC
              </p>
            </div>

            {/* Spendable vs Pending */}
            <div className="grid grid-cols-2 gap-2 ml-12">
              <div className="p-3 bg-green-50/50 rounded-lg border border-green-100">
                <div className="flex items-center gap-2">
                  <Lock className="w-4 h-4 text-green-600" />
                  <span className="text-sm text-gray-600">
                    {t('zcash.orchard.spendable', 'Spendable')}
                  </span>
                </div>
                <p className="font-mono text-green-700 mt-1">
                  {spendableZec.toFixed(8)} ZEC
                </p>
              </div>
              <div className="p-3 bg-yellow-50/50 rounded-lg border border-yellow-100">
                <div className="flex items-center gap-2">
                  <span className="w-4 h-4 flex items-center justify-center">
                    <span className="animate-pulse w-2 h-2 bg-yellow-500 rounded-full"></span>
                  </span>
                  <span className="text-sm text-gray-600">
                    {t('zcash.orchard.pending', 'Pending')}
                  </span>
                </div>
                <p className="font-mono text-yellow-700 mt-1">
                  {pendingZec.toFixed(8)} ZEC
                </p>
              </div>
            </div>

            {/* Note Count */}
            <div className="text-xs text-gray-500 ml-12">
              {t('zcash.orchard.noteCount', '{{count}} shielded notes', {
                count: balance.shielded_balance.note_count,
              })}
            </div>
          </div>
        ) : (
          <div className="p-4 bg-gray-50 rounded-lg border-2 border-dashed border-gray-200">
            <div className="flex items-center gap-3">
              <Shield className="w-5 h-5 text-gray-400" />
              <div>
                <p className="font-medium text-gray-600">
                  {t('zcash.orchard.notEnabled', 'Orchard Not Enabled')}
                </p>
                <p className="text-xs text-gray-500">
                  {t('zcash.orchard.enablePrompt', 'Enable Orchard to receive private transfers')}
                </p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
