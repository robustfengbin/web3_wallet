import React from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw, CheckCircle, Clock, Database } from 'lucide-react';
import type { ScanProgress as ScanProgressType } from '../../../../types/orchard';

interface ScanProgressProps {
  progress: ScanProgressType | null;
  onSync?: () => void;
  loading?: boolean;
}

export function ScanProgress({
  progress,
  onSync,
  loading = false,
}: ScanProgressProps) {
  const { t } = useTranslation();

  if (!progress) {
    return null;
  }

  const formatTime = (seconds: number): string => {
    if (seconds < 60) {
      return `${seconds}s`;
    }
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    if (minutes < 60) {
      return `${minutes}m ${remainingSeconds}s`;
    }
    const hours = Math.floor(minutes / 60);
    const remainingMinutes = minutes % 60;
    return `${hours}h ${remainingMinutes}m`;
  };

  const isSynced = progress.progress_percent >= 100;
  const blocksRemaining = progress.chain_tip_height - progress.last_scanned_height;

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Database className="w-5 h-5 text-gray-500" />
          <h3 className="font-medium text-gray-900">
            {t('zcash.orchard.scanProgress', 'Blockchain Sync')}
          </h3>
        </div>
        {onSync && (
          <button
            onClick={onSync}
            disabled={loading || progress.is_scanning}
            className={`
              flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium
              ${loading || progress.is_scanning
                ? 'bg-gray-100 text-gray-400 cursor-not-allowed'
                : 'bg-yellow-100 text-yellow-700 hover:bg-yellow-200'
              }
            `}
          >
            <RefreshCw className={`w-4 h-4 ${(loading || progress.is_scanning) ? 'animate-spin' : ''}`} />
            {progress.is_scanning
              ? t('zcash.orchard.syncing', 'Syncing...')
              : t('zcash.orchard.sync', 'Sync')}
          </button>
        )}
      </div>

      {/* Progress Bar */}
      <div className="relative h-2 bg-gray-200 rounded-full overflow-hidden mb-2">
        <div
          className={`absolute inset-y-0 left-0 rounded-full transition-all duration-500 ${
            isSynced ? 'bg-green-500' : 'bg-yellow-500'
          }`}
          style={{ width: `${Math.min(progress.progress_percent, 100)}%` }}
        />
      </div>

      {/* Status Info */}
      <div className="flex items-center justify-between text-sm">
        <div className="flex items-center gap-2">
          {isSynced ? (
            <>
              <CheckCircle className="w-4 h-4 text-green-500" />
              <span className="text-green-600">
                {t('zcash.orchard.synced', 'Fully synced')}
              </span>
            </>
          ) : (
            <>
              <Clock className="w-4 h-4 text-yellow-500" />
              <span className="text-yellow-600">
                {progress.progress_percent.toFixed(1)}%
              </span>
            </>
          )}
        </div>
        <span className="text-gray-500">
          {t('zcash.orchard.blockHeight', 'Block {{height}}', {
            height: progress.last_scanned_height.toLocaleString(),
          })}
        </span>
      </div>

      {/* Additional Info */}
      {!isSynced && (
        <div className="mt-3 pt-3 border-t border-gray-100 grid grid-cols-2 gap-4 text-xs text-gray-500">
          <div>
            <span className="block text-gray-400">
              {t('zcash.orchard.blocksRemaining', 'Blocks remaining')}
            </span>
            <span className="font-medium text-gray-600">
              {blocksRemaining.toLocaleString()}
            </span>
          </div>
          {progress.estimated_seconds_remaining !== null && (
            <div>
              <span className="block text-gray-400">
                {t('zcash.orchard.estimatedTime', 'Est. time')}
              </span>
              <span className="font-medium text-gray-600">
                {formatTime(progress.estimated_seconds_remaining)}
              </span>
            </div>
          )}
          <div>
            <span className="block text-gray-400">
              {t('zcash.orchard.notesFound', 'Notes found')}
            </span>
            <span className="font-medium text-gray-600">
              {progress.notes_found}
            </span>
          </div>
          <div>
            <span className="block text-gray-400">
              {t('zcash.orchard.chainTip', 'Chain tip')}
            </span>
            <span className="font-medium text-gray-600">
              {progress.chain_tip_height.toLocaleString()}
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
