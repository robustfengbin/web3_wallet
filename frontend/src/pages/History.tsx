import React, { useState, useEffect } from 'react';
import { ExternalLink, RefreshCw, ChevronLeft, ChevronRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card, LoadingSpinner, StatusBadge } from '../components/Common';
import { transferService } from '../services/api';
import { TransferListResponse } from '../types';
import { getChain } from '../config/chains';

export function History() {
  const { t } = useTranslation();
  const [transfers, setTransfers] = useState<TransferListResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [page, setPage] = useState(0);
  const limit = 10;

  useEffect(() => {
    loadTransfers();
  }, [page]);

  const loadTransfers = async () => {
    setIsLoading(true);
    try {
      const data = await transferService.listTransfers(limit, page * limit);
      setTransfers(data);
    } catch (error) {
      console.error('Failed to load transfers:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleString();
  };

  // Format amount based on chain decimals
  const formatAmount = (amount: string, chainId: string) => {
    const num = parseFloat(amount);
    if (isNaN(num)) return amount;
    // Use reasonable precision based on chain
    const chain = getChain(chainId);
    const decimals = chain?.tokens[0]?.decimals || 18;
    // Show at most 8 decimal places, but trim trailing zeros
    const maxDecimals = Math.min(decimals, 8);
    return num.toFixed(maxDecimals).replace(/\.?0+$/, '');
  };

  // Get explorer URL for transaction based on chain
  const getExplorerUrl = (txHash: string, chainId: string) => {
    const chain = getChain(chainId);
    if (!chain?.explorerUrl) {
      return `https://etherscan.io/tx/${txHash}`;
    }
    // Different explorers use different URL patterns
    if (chainId === 'zcash') {
      return `${chain.explorerUrl}/transactions/${txHash}`;
    }
    return `${chain.explorerUrl}/tx/${txHash}`;
  };

  // Get chain display name
  const getChainName = (chainId: string) => {
    const chain = getChain(chainId);
    return chain?.name || chainId.toUpperCase();
  };

  const totalPages = transfers ? Math.ceil(transfers.total / limit) : 0;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900">{t('history.title')}</h1>
        <button
          onClick={loadTransfers}
          className="flex items-center px-4 py-2 text-gray-600 hover:text-gray-800"
        >
          <RefreshCw className="w-4 h-4 mr-2" />
          {t('common.refresh')}
        </button>
      </div>

      <Card>
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <LoadingSpinner size="lg" />
          </div>
        ) : !transfers?.transfers.length ? (
          <div className="text-center py-12 text-gray-500">
            <p>{t('history.noTransfers')}</p>
          </div>
        ) : (
          <>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="text-left text-sm text-gray-500 border-b">
                    <th className="pb-3 font-medium">{t('history.id')}</th>
                    <th className="pb-3 font-medium">{t('history.date')}</th>
                    <th className="pb-3 font-medium">{t('history.to')}</th>
                    <th className="pb-3 font-medium">{t('history.amount')}</th>
                    <th className="pb-3 font-medium">{t('history.chain')}</th>
                    <th className="pb-3 font-medium">{t('history.status')}</th>
                    <th className="pb-3 font-medium">{t('history.txHash')}</th>
                  </tr>
                </thead>
                <tbody>
                  {transfers.transfers.map((transfer) => (
                    <tr
                      key={transfer.id}
                      className="border-b last:border-0 hover:bg-gray-50"
                    >
                      <td className="py-4 text-sm">#{transfer.id}</td>
                      <td className="py-4 text-sm text-gray-500">
                        {formatDate(transfer.created_at)}
                      </td>
                      <td className="py-4 font-mono text-sm">
                        {transfer.to_address.slice(0, 8)}...
                        {transfer.to_address.slice(-6)}
                      </td>
                      <td className="py-4 font-medium">
                        {formatAmount(transfer.amount, transfer.chain)} {transfer.token}
                      </td>
                      <td className="py-4 text-sm">{getChainName(transfer.chain)}</td>
                      <td className="py-4">
                        <StatusBadge status={transfer.status} />
                      </td>
                      <td className="py-4">
                        {transfer.tx_hash ? (
                          <a
                            href={getExplorerUrl(transfer.tx_hash, transfer.chain)}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="flex items-center text-blue-600 hover:text-blue-800 text-sm"
                          >
                            {transfer.tx_hash.slice(0, 10)}...
                            <ExternalLink className="w-3 h-3 ml-1" />
                          </a>
                        ) : (
                          <span className="text-gray-400">-</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {/* Pagination */}
            {totalPages > 1 && (
              <div className="flex items-center justify-between mt-6 pt-4 border-t">
                <p className="text-sm text-gray-500">
                  {t('history.showing', {
                    start: page * limit + 1,
                    end: Math.min((page + 1) * limit, transfers.total),
                    total: transfers.total,
                  })}
                </p>
                <div className="flex items-center space-x-2">
                  <button
                    onClick={() => setPage((p) => Math.max(0, p - 1))}
                    disabled={page === 0}
                    className="p-2 rounded-lg hover:bg-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <ChevronLeft className="w-5 h-5" />
                  </button>
                  <span className="text-sm">
                    {t('history.page', { current: page + 1, total: totalPages })}
                  </span>
                  <button
                    onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
                    disabled={page >= totalPages - 1}
                    className="p-2 rounded-lg hover:bg-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <ChevronRight className="w-5 h-5" />
                  </button>
                </div>
              </div>
            )}
          </>
        )}
      </Card>
    </div>
  );
}
