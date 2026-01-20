import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Shield, Send, Eye, EyeOff } from 'lucide-react';
import { ChainTransfer } from '../components/ChainTransfer';
import { PrivacyTransfer } from './PrivacyTransfer';

type TransferMode = 'transparent' | 'privacy';

export function ZcashTransfer() {
  const { t } = useTranslation();
  const [mode, setMode] = useState<TransferMode>('transparent');

  return (
    <div className="space-y-6">
      {/* Transfer Mode Selector */}
      <div className="bg-white rounded-lg shadow p-1 flex">
        <button
          onClick={() => setMode('transparent')}
          className={`flex-1 flex items-center justify-center gap-2 py-3 px-4 rounded-md font-medium transition-colors ${
            mode === 'transparent'
              ? 'bg-yellow-500 text-white'
              : 'text-gray-600 hover:bg-gray-100'
          }`}
        >
          <Eye className="w-5 h-5" />
          {t('zcash.transfer.transparent', 'Transparent Transfer')}
        </button>
        <button
          onClick={() => setMode('privacy')}
          className={`flex-1 flex items-center justify-center gap-2 py-3 px-4 rounded-md font-medium transition-colors ${
            mode === 'privacy'
              ? 'bg-yellow-500 text-white'
              : 'text-gray-600 hover:bg-gray-100'
          }`}
        >
          <Shield className="w-5 h-5" />
          {t('zcash.transfer.privacy', 'Privacy Transfer')}
        </button>
      </div>

      {/* Mode Description */}
      <div className={`p-4 rounded-lg border ${
        mode === 'transparent'
          ? 'bg-blue-50 border-blue-200'
          : 'bg-green-50 border-green-200'
      }`}>
        <div className="flex items-start gap-3">
          {mode === 'transparent' ? (
            <>
              <Eye className="w-5 h-5 text-blue-600 flex-shrink-0 mt-0.5" />
              <div>
                <p className="text-sm font-medium text-blue-800">
                  {t('zcash.transfer.transparentTitle', 'Transparent (Public) Transfer')}
                </p>
                <p className="text-xs text-blue-700 mt-1">
                  {t(
                    'zcash.transfer.transparentDesc',
                    'Transaction details (sender, recipient, amount) are visible on the blockchain. Similar to Bitcoin transactions.'
                  )}
                </p>
              </div>
            </>
          ) : (
            <>
              <EyeOff className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
              <div>
                <p className="text-sm font-medium text-green-800">
                  {t('zcash.transfer.privacyTitle', 'Private (Shielded) Transfer')}
                </p>
                <p className="text-xs text-green-700 mt-1">
                  {t(
                    'zcash.transfer.privacyDesc',
                    'Transaction details are encrypted using zero-knowledge proofs. Sender, recipient, and amount remain completely private.'
                  )}
                </p>
              </div>
            </>
          )}
        </div>
      </div>

      {/* Transfer Component */}
      {mode === 'transparent' ? (
        <ChainTransfer chainId="zcash" />
      ) : (
        <PrivacyTransfer />
      )}
    </div>
  );
}
