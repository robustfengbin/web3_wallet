import React from 'react';
import { useTranslation } from 'react-i18next';

interface StatusBadgeProps {
  status: 'pending' | 'submitted' | 'confirmed' | 'failed';
}

export function StatusBadge({ status }: StatusBadgeProps) {
  const { t } = useTranslation();

  const statusConfig = {
    pending: {
      bg: 'bg-yellow-100',
      text: 'text-yellow-800',
    },
    submitted: {
      bg: 'bg-blue-100',
      text: 'text-blue-800',
    },
    confirmed: {
      bg: 'bg-green-100',
      text: 'text-green-800',
    },
    failed: {
      bg: 'bg-red-100',
      text: 'text-red-800',
    },
  };

  const config = statusConfig[status] || statusConfig.pending;

  return (
    <span
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${config.bg} ${config.text}`}
    >
      {t(`status.${status}`)}
    </span>
  );
}
