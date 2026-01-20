import React from 'react';
import { useTranslation } from 'react-i18next';
import { Lock, Info } from 'lucide-react';

interface MemoInputProps {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  maxLength?: number;
}

export function MemoInput({
  value,
  onChange,
  disabled = false,
  maxLength = 512,
}: MemoInputProps) {
  const { t } = useTranslation();
  const charCount = value.length;
  const isNearLimit = charCount > maxLength * 0.8;

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <label className="flex items-center gap-2 text-sm font-medium text-gray-700">
          <Lock className="w-4 h-4 text-green-600" />
          {t('zcash.orchard.encryptedMemo', 'Encrypted Memo')}
          <span className="text-gray-400 font-normal">
            ({t('common.optional', 'Optional')})
          </span>
        </label>
        <span
          className={`text-xs ${
            isNearLimit ? 'text-yellow-600' : 'text-gray-400'
          }`}
        >
          {charCount}/{maxLength}
        </span>
      </div>

      <textarea
        value={value}
        onChange={(e) => onChange(e.target.value.slice(0, maxLength))}
        disabled={disabled}
        rows={3}
        placeholder={t(
          'zcash.orchard.memoPlaceholder',
          'Enter an encrypted memo (only visible to recipient)...'
        )}
        className={`
          w-full px-4 py-3 rounded-lg border transition-colors resize-none
          ${disabled
            ? 'bg-gray-100 border-gray-200 text-gray-500 cursor-not-allowed'
            : 'bg-white border-gray-300 focus:border-yellow-500 focus:ring-2 focus:ring-yellow-500/20'
          }
        `}
      />

      <div className="flex items-start gap-2 text-xs text-gray-500">
        <Info className="w-4 h-4 text-gray-400 flex-shrink-0 mt-0.5" />
        <p>
          {t(
            'zcash.orchard.memoInfo',
            'This memo is encrypted end-to-end. Only the recipient with the viewing key can read it. The memo is stored on-chain but completely private.'
          )}
        </p>
      </div>
    </div>
  );
}
