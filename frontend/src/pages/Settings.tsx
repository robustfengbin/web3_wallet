import React, { useState } from 'react';
import { Key, AlertCircle, CheckCircle, Globe } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card, LoadingSpinner } from '../components/Common';
import { authService } from '../services/api';
import { useAuth } from '../hooks/useAuth';

const languages = [
  { code: 'zh', name: '中文' },
  { code: 'en', name: 'English' },
];

export function Settings() {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const [oldPassword, setOldPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setSuccess('');

    if (newPassword !== confirmPassword) {
      setError(t('settings.passwordMismatch'));
      return;
    }

    if (newPassword.length < 6) {
      setError(t('settings.passwordTooShort'));
      return;
    }

    setIsLoading(true);
    try {
      await authService.changePassword(oldPassword, newPassword);
      setSuccess(t('settings.passwordChanged'));
      setOldPassword('');
      setNewPassword('');
      setConfirmPassword('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to change password');
    } finally {
      setIsLoading(false);
    }
  };

  const handleLanguageChange = (langCode: string) => {
    i18n.changeLanguage(langCode);
  };

  return (
    <div>
      <h1 className="text-2xl font-bold text-gray-900 mb-6">{t('settings.title')}</h1>

      <div className="max-w-2xl space-y-6">
        {/* Language Settings */}
        <Card title={t('settings.languageSettings')}>
          <div className="space-y-3">
            <p className="text-sm text-gray-500">{t('settings.selectLanguage')}</p>
            <div className="flex space-x-3">
              {languages.map((lang) => (
                <button
                  key={lang.code}
                  onClick={() => handleLanguageChange(lang.code)}
                  className={`flex items-center px-4 py-2 rounded-lg border transition-colors ${
                    i18n.language === lang.code
                      ? 'border-blue-500 bg-blue-50 text-blue-700'
                      : 'border-gray-300 hover:border-gray-400'
                  }`}
                >
                  <Globe className="w-4 h-4 mr-2" />
                  {lang.name}
                </button>
              ))}
            </div>
          </div>
        </Card>

        {/* User Info */}
        <Card title={t('settings.accountInfo')}>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-gray-500">{t('settings.username')}</span>
              <span className="font-medium">{user?.username}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-gray-500">{t('settings.role')}</span>
              <span className="px-2 py-1 bg-gray-100 text-gray-700 rounded">
                {user?.role === 'admin' ? t('common.admin') : t('common.operator')}
              </span>
            </div>
          </div>
        </Card>

        {/* Change Password */}
        <Card title={t('settings.changePassword')}>
          {error && (
            <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 flex items-center">
              <AlertCircle className="w-5 h-5 mr-2" />
              {error}
            </div>
          )}

          {success && (
            <div className="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg text-green-700 flex items-center">
              <CheckCircle className="w-5 h-5 mr-2" />
              {success}
            </div>
          )}

          <form onSubmit={handleChangePassword} className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.currentPassword')}
              </label>
              <input
                type="password"
                value={oldPassword}
                onChange={(e) => setOldPassword(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.newPassword')}
              </label>
              <input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.confirmPassword')}
              </label>
              <input
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                required
              />
            </div>

            <button
              type="submit"
              disabled={isLoading}
              className="flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
            >
              {isLoading ? (
                <LoadingSpinner size="sm" />
              ) : (
                <>
                  <Key className="w-4 h-4 mr-2" />
                  {t('settings.changePassword')}
                </>
              )}
            </button>
          </form>
        </Card>
      </div>
    </div>
  );
}
