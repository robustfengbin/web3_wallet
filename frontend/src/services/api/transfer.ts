import api from './axios';
import { Transfer, TransferListResponse, TransferRequest, ChainInfo } from '../../types';

export interface GasEstimateRequest {
  chain: string;
  to_address: string;
  token: string;
  amount: string;
}

export interface GasEstimateResponse {
  gas_limit: number;
  gas_price_gwei: string;
  estimated_fee_eth: string;
  estimated_fee_usd: string | null;
  // EIP-1559 specific fields
  base_fee_gwei: string | null;
  priority_fee_gwei: string | null;
  max_fee_gwei: string | null;
}

export const transferService = {
  async listTransfers(
    limit: number = 20,
    offset: number = 0,
    walletId?: number
  ): Promise<TransferListResponse> {
    const params: Record<string, number> = { limit, offset };
    if (walletId) params.wallet_id = walletId;
    return api.get('/transfers', { params });
  },

  async getTransfer(id: number): Promise<Transfer> {
    return api.get(`/transfers/${id}`);
  },

  async initiateTransfer(data: TransferRequest): Promise<Transfer> {
    return api.post('/transfers', data);
  },

  async executeTransfer(id: number): Promise<Transfer> {
    return api.post(`/transfers/${id}/execute`);
  },

  async listChains(): Promise<ChainInfo[]> {
    return api.get('/chains');
  },

  async estimateGas(data: GasEstimateRequest): Promise<GasEstimateResponse> {
    return api.post('/transfers/estimate-gas', data);
  },
};
