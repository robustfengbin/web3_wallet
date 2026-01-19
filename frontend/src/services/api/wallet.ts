import api from './axios';
import {
  Wallet,
  BalanceResponse,
  CreateWalletRequest,
  ImportWalletRequest,
  ExportPrivateKeyResponse,
} from '../../types';

export const walletService = {
  async listWallets(chain?: string): Promise<Wallet[]> {
    const params = chain ? { chain } : {};
    return api.get('/wallets', { params });
  },

  async getWallet(id: number): Promise<Wallet> {
    return api.get(`/wallets/${id}`);
  },

  async createWallet(data: CreateWalletRequest): Promise<Wallet> {
    return api.post('/wallets', data);
  },

  async importWallet(data: ImportWalletRequest): Promise<Wallet> {
    return api.post('/wallets/import', data);
  },

  async getBalance(address: string, chain?: string): Promise<BalanceResponse> {
    const params: Record<string, string> = { address };
    if (chain) params.chain = chain;
    return api.get('/wallets/balance', { params });
  },

  async setActiveWallet(id: number): Promise<void> {
    return api.put(`/wallets/${id}/activate`);
  },

  async exportPrivateKey(id: number, password: string): Promise<ExportPrivateKeyResponse> {
    return api.post(`/wallets/${id}/export-key`, { password });
  },

  async deleteWallet(id: number): Promise<void> {
    return api.delete(`/wallets/${id}`);
  },
};
