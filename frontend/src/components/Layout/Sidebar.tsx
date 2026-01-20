import React from 'react';
import { NavLink, useLocation } from 'react-router-dom';
import {
  LayoutDashboard,
  Wallet,
  ArrowLeftRight,
  History,
  Settings,
  Server,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../hooks/useAuth';

// Chain configuration
const CHAIN_LIST = [
  { id: 'ethereum', name: 'Ethereum', icon: '⟠', color: '#627EEA' },
  { id: 'zcash', name: 'Zcash', icon: 'Ⓩ', color: '#F4B728' },
];

interface ChainSectionProps {
  chain: { id: string; name: string; icon: string; color: string };
  isExpanded: boolean;
}

function ChainSection({ chain, isExpanded }: ChainSectionProps) {
  const { t } = useTranslation();
  const basePath = `/${chain.id}`;

  return (
    <div>
      <NavLink
        to={`${basePath}/wallets`}
        className={({ isActive }) =>
          `flex items-center justify-between px-6 py-3 text-sm transition-colors ${
            isActive || isExpanded
              ? 'bg-gray-800 text-white'
              : 'text-gray-300 hover:bg-gray-800 hover:text-white'
          }`
        }
      >
        <div className="flex items-center">
          <span
            className="w-6 h-6 rounded-full flex items-center justify-center text-white text-xs mr-3"
            style={{ backgroundColor: chain.color }}
          >
            {chain.icon}
          </span>
          {chain.name}
        </div>
        {isExpanded ? (
          <ChevronDown className="w-4 h-4" />
        ) : (
          <ChevronRight className="w-4 h-4" />
        )}
      </NavLink>

      {isExpanded && (
        <div className="bg-gray-950 w-full">
          <NavLink
            to={`${basePath}/wallets`}
            className={({ isActive }) =>
              `flex items-center pl-12 pr-6 py-2 text-sm transition-colors ${
                isActive
                  ? 'text-white bg-blue-600'
                  : 'text-gray-400 hover:bg-gray-800 hover:text-white'
              }`
            }
          >
            <Wallet className="w-4 h-4 mr-3" />
            {t('sidebar.wallets')}
          </NavLink>
          <NavLink
            to={`${basePath}/transfer`}
            className={({ isActive }) =>
              `flex items-center pl-12 pr-6 py-2 text-sm transition-colors ${
                isActive
                  ? 'text-white bg-blue-600'
                  : 'text-gray-400 hover:bg-gray-800 hover:text-white'
              }`
            }
          >
            <ArrowLeftRight className="w-4 h-4 mr-3" />
            {t('sidebar.transfer')}
          </NavLink>
          <NavLink
            to={`${basePath}/rpc`}
            className={({ isActive }) =>
              `flex items-center pl-12 pr-6 py-2 text-sm transition-colors ${
                isActive
                  ? 'text-white bg-blue-600'
                  : 'text-gray-400 hover:bg-gray-800 hover:text-white'
              }`
            }
          >
            <Server className="w-4 h-4 mr-3" />
            {t('sidebar.rpcSettings')}
          </NavLink>
        </div>
      )}
    </div>
  );
}

export function Sidebar() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const location = useLocation();

  return (
    <aside className="w-64 bg-gray-900 text-white min-h-screen overflow-hidden">
      <div className="p-6">
        <h1 className="text-xl font-bold">{t('sidebar.title')}</h1>
        <p className="text-gray-400 text-sm mt-1">{t('sidebar.subtitle')}</p>
      </div>

      <nav className="mt-2">
        {/* Dashboard */}
        <NavLink
          to="/"
          end
          className={({ isActive }) =>
            `flex items-center px-6 py-3 text-sm transition-colors ${
              isActive
                ? 'bg-blue-600 text-white'
                : 'text-gray-300 hover:bg-gray-800 hover:text-white'
            }`
          }
        >
          <LayoutDashboard className="w-5 h-5 mr-3" />
          {t('sidebar.dashboard')}
        </NavLink>

        {/* Chain Sections */}
        <div className="mt-4">
          <div className="px-6 py-2 text-xs font-semibold text-gray-500 uppercase tracking-wider">
            {t('sidebar.chains')}
          </div>

          {CHAIN_LIST.map((chain) => (
            <ChainSection
              key={chain.id}
              chain={chain}
              isExpanded={location.pathname.startsWith(`/${chain.id}`)}
            />
          ))}
        </div>

        {/* History & Settings */}
        <div className="mt-4">
          <div className="px-6 py-2 text-xs font-semibold text-gray-500 uppercase tracking-wider">
            {t('sidebar.general')}
          </div>
          <NavLink
            to="/history"
            className={({ isActive }) =>
              `flex items-center px-6 py-3 text-sm transition-colors ${
                isActive
                  ? 'bg-blue-600 text-white'
                  : 'text-gray-300 hover:bg-gray-800 hover:text-white'
              }`
            }
          >
            <History className="w-5 h-5 mr-3" />
            {t('sidebar.history')}
          </NavLink>
          <NavLink
            to="/settings"
            className={({ isActive }) =>
              `flex items-center px-6 py-3 text-sm transition-colors ${
                isActive
                  ? 'bg-blue-600 text-white'
                  : 'text-gray-300 hover:bg-gray-800 hover:text-white'
              }`
            }
          >
            <Settings className="w-5 h-5 mr-3" />
            {t('sidebar.settings')}
          </NavLink>
        </div>
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
