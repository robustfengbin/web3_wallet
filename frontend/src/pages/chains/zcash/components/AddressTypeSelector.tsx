import React from 'react';
import { useTranslation } from 'react-i18next';
import { Shield, Eye, Lock } from 'lucide-react';
import type { AddressType, AddressTypeConfig } from '../../../../types/orchard';
import { ADDRESS_TYPE_CONFIGS } from '../../../../types/orchard';

interface AddressTypeSelectorProps {
  value: AddressType;
  onChange: (type: AddressType) => void;
  disabled?: boolean;
}

export function AddressTypeSelector({
  value,
  onChange,
  disabled = false,
}: AddressTypeSelectorProps) {
  const { t } = useTranslation();

  const getIcon = (type: AddressType) => {
    switch (type) {
      case 'unified':
        return <Shield className="w-5 h-5" />;
      case 'transparent':
        return <Eye className="w-5 h-5" />;
      case 'orchard_only':
        return <Lock className="w-5 h-5" />;
    }
  };

  const getPrivacyColor = (config: AddressTypeConfig) => {
    switch (config.privacyLevel) {
      case 'full':
        return 'text-green-600 bg-green-100';
      case 'partial':
        return 'text-yellow-600 bg-yellow-100';
      case 'none':
        return 'text-red-600 bg-red-100';
    }
  };

  return (
    <div className="space-y-2">
      <label className="block text-sm font-medium text-gray-700">
        {t('zcash.orchard.addressType', 'Address Type')}
      </label>
      <div className="grid grid-cols-1 gap-3">
        {ADDRESS_TYPE_CONFIGS.map((config) => (
          <button
            key={config.type}
            type="button"
            disabled={disabled}
            onClick={() => onChange(config.type)}
            className={`
              relative flex items-start p-4 rounded-lg border-2 transition-all
              ${value === config.type
                ? 'border-yellow-500 bg-yellow-50'
                : 'border-gray-200 hover:border-gray-300 bg-white'
              }
              ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
            `}
          >
            <div className="flex items-center h-5">
              <input
                type="radio"
                checked={value === config.type}
                onChange={() => onChange(config.type)}
                disabled={disabled}
                className="h-4 w-4 text-yellow-600 border-gray-300 focus:ring-yellow-500"
              />
            </div>
            <div className="ml-3 flex-1">
              <div className="flex items-center gap-2">
                <span className={`p-1 rounded ${getPrivacyColor(config)}`}>
                  {getIcon(config.type)}
                </span>
                <span className="font-medium text-gray-900">{config.label}</span>
                <span className="text-xs text-gray-500">({config.prefix}...)</span>
              </div>
              <p className="mt-1 text-sm text-gray-500">{config.description}</p>
            </div>
            {config.type === 'unified' && (
              <span className="absolute top-2 right-2 text-xs bg-yellow-500 text-white px-2 py-0.5 rounded">
                {t('common.recommended', 'Recommended')}
              </span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
