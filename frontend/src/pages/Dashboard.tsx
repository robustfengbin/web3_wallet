import React, { useState, useEffect } from 'react';
import { Wallet, ArrowLeftRight, CheckCircle, Clock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card, LoadingSpinner } from '../components/Common';
import { walletService, transferService } from '../services/api';
import { Wallet as WalletType, TransferListResponse } from '../types';

export function Dashboard() {
  const { t } = useTranslation();
  const [wallets, setWallets] = useState<WalletType[]>([]);
  const [transfers, setTransfers] = useState<TransferListResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      const [walletsData, transfersData] = await Promise.all([
        walletService.listWallets(),
        transferService.listTransfers(5, 0),
      ]);
      setWallets(walletsData);
      setTransfers(transfersData);
    } catch (error) {
      console.error('Failed to load dashboard data:', error);
    } finally {
      setIsLoading(false);
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <LoadingSpinner size="lg" />
      </div>
    );
  }

  const confirmedTransfers = transfers?.transfers.filter(
    (t) => t.status === 'confirmed'
  ).length || 0;

  const pendingTransfers = transfers?.transfers.filter(
    (t) => t.status === 'pending' || t.status === 'submitted'
  ).length || 0;

  return (
    <div>
      <h1 className="text-2xl font-bold text-gray-900 mb-6">{t('dashboard.title')}</h1>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
        <Card>
          <div className="flex items-center">
            <div className="p-3 bg-blue-100 rounded-full">
              <Wallet className="w-6 h-6 text-blue-600" />
            </div>
            <div className="ml-4">
              <p className="text-sm text-gray-500">{t('dashboard.totalWallets')}</p>
              <p className="text-2xl font-bold text-gray-900">{wallets.length}</p>
            </div>
          </div>
        </Card>

        <Card>
          <div className="flex items-center">
            <div className="p-3 bg-green-100 rounded-full">
              <ArrowLeftRight className="w-6 h-6 text-green-600" />
            </div>
            <div className="ml-4">
              <p className="text-sm text-gray-500">{t('dashboard.totalTransfers')}</p>
              <p className="text-2xl font-bold text-gray-900">
                {transfers?.total || 0}
              </p>
            </div>
          </div>
        </Card>

        <Card>
          <div className="flex items-center">
            <div className="p-3 bg-emerald-100 rounded-full">
              <CheckCircle className="w-6 h-6 text-emerald-600" />
            </div>
            <div className="ml-4">
              <p className="text-sm text-gray-500">{t('dashboard.confirmed')}</p>
              <p className="text-2xl font-bold text-gray-900">{confirmedTransfers}</p>
            </div>
          </div>
        </Card>

        <Card>
          <div className="flex items-center">
            <div className="p-3 bg-yellow-100 rounded-full">
              <Clock className="w-6 h-6 text-yellow-600" />
            </div>
            <div className="ml-4">
              <p className="text-sm text-gray-500">{t('dashboard.pending')}</p>
              <p className="text-2xl font-bold text-gray-900">{pendingTransfers}</p>
            </div>
          </div>
        </Card>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card title={t('dashboard.activeWallets')}>
          {wallets.length === 0 ? (
            <p className="text-gray-500">{t('dashboard.noWallets')}</p>
          ) : (
            <div className="space-y-3">
              {wallets.slice(0, 5).map((wallet) => (
                <div
                  key={wallet.id}
                  className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
                >
                  <div>
                    <p className="font-medium text-gray-900">{wallet.name}</p>
                    <p className="text-sm text-gray-500 font-mono">
                      {wallet.address.slice(0, 10)}...{wallet.address.slice(-8)}
                    </p>
                  </div>
                  <div className="text-right">
                    <span className="text-xs text-gray-500 uppercase">
                      {wallet.chain}
                    </span>
                    {wallet.is_active && (
                      <span className="ml-2 px-2 py-0.5 bg-green-100 text-green-800 text-xs rounded-full">
                        {t('common.active')}
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>

        <Card title={t('dashboard.recentTransfers')}>
          {!transfers?.transfers.length ? (
            <p className="text-gray-500">{t('dashboard.noTransfers')}</p>
          ) : (
            <div className="space-y-3">
              {transfers.transfers.map((transfer) => (
                <div
                  key={transfer.id}
                  className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
                >
                  <div>
                    <p className="text-sm text-gray-900">
                      {transfer.amount} {transfer.token}
                    </p>
                    <p className="text-xs text-gray-500 font-mono">
                      To: {transfer.to_address.slice(0, 10)}...
                    </p>
                  </div>
                  <div
                    className={`px-2 py-1 rounded text-xs font-medium ${
                      transfer.status === 'confirmed'
                        ? 'bg-green-100 text-green-800'
                        : transfer.status === 'failed'
                        ? 'bg-red-100 text-red-800'
                        : 'bg-yellow-100 text-yellow-800'
                    }`}
                  >
                    {t(`status.${transfer.status}`)}
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}
