import React from 'react';
import { NavLink } from 'react-router-dom';
import {
  LayoutDashboard,
  Wallet,
  ArrowLeftRight,
  History,
  Settings,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../hooks/useAuth';

export function Sidebar() {
  const { t } = useTranslation();
  const { user } = useAuth();

  const navItems = [
    { path: '/', label: t('sidebar.dashboard'), icon: LayoutDashboard },
    { path: '/wallets', label: t('sidebar.wallets'), icon: Wallet },
    { path: '/transfer', label: t('sidebar.transfer'), icon: ArrowLeftRight },
    { path: '/history', label: t('sidebar.history'), icon: History },
    { path: '/settings', label: t('sidebar.settings'), icon: Settings },
  ];

  return (
    <aside className="w-64 bg-gray-900 text-white min-h-screen">
      <div className="p-6">
        <h1 className="text-xl font-bold">{t('sidebar.title')}</h1>
        <p className="text-gray-400 text-sm mt-1">{t('sidebar.subtitle')}</p>
      </div>

      <nav className="mt-6">
        {navItems.map((item) => (
          <NavLink
            key={item.path}
            to={item.path}
            className={({ isActive }) =>
              `flex items-center px-6 py-3 text-sm transition-colors ${
                isActive
                  ? 'bg-blue-600 text-white'
                  : 'text-gray-300 hover:bg-gray-800 hover:text-white'
              }`
            }
          >
            <item.icon className="w-5 h-5 mr-3" />
            {item.label}
          </NavLink>
        ))}
      </nav>

      <div className="absolute bottom-0 left-0 w-64 p-6 border-t border-gray-700">
        <div className="text-sm">
          <p className="text-gray-400">{t('sidebar.loggedInAs')}</p>
          <p className="font-medium">{user?.username}</p>
          <p className="text-xs text-gray-500">
            {user?.role === 'admin' ? t('common.admin') : t('common.operator')}
          </p>
        </div>
      </div>
    </aside>
  );
}
