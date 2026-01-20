/**
 * Orchard Privacy Protocol API Service
 *
 * API endpoints for Zcash Orchard shielded transfers.
 */

import axios from './axios';
import type {
  CombinedZcashBalance,
  EnableOrchardRequest,
  EnableOrchardResponse,
  GenerateAddressRequest,
  OrchardTransactionInfo,
  OrchardTransferRequest,
  OrchardTransferResponse,
  ScanProgress,
  ShieldedBalance,
  UnifiedAddressInfo,
} from '../../types/orchard';

/**
 * Enable Orchard for a Zcash wallet
 *
 * This derives Orchard keys and generates a unified address.
 */
export async function enableOrchard(
  request: EnableOrchardRequest
): Promise<EnableOrchardResponse> {
  const response = await axios.post<EnableOrchardResponse>(
    `/wallets/${request.wallet_id}/orchard/enable`,
    { birthday_height: request.birthday_height }
  );
  return response.data;
}

/**
 * Get all unified addresses for a wallet
 */
export async function getUnifiedAddresses(
  walletId: number
): Promise<UnifiedAddressInfo[]> {
  const response = await axios.get<UnifiedAddressInfo[]>(
    `/wallets/${walletId}/orchard/addresses`
  );
  return response.data;
}

/**
 * Generate a new unified address
 */
export async function generateUnifiedAddress(
  walletId: number,
  request: GenerateAddressRequest
): Promise<UnifiedAddressInfo> {
  const response = await axios.post<UnifiedAddressInfo>(
    `/wallets/${walletId}/orchard/addresses`,
    request
  );
  return response.data;
}

/**
 * Get shielded (Orchard) balance for a wallet
 */
export async function getShieldedBalance(
  walletId: number
): Promise<ShieldedBalance> {
  const response = await axios.get<ShieldedBalance>(
    `/wallets/${walletId}/orchard/balance`
  );
  return response.data;
}

/**
 * Get combined balance (transparent + shielded)
 */
export async function getCombinedBalance(
  walletId: number
): Promise<CombinedZcashBalance> {
  const response = await axios.get<CombinedZcashBalance>(
    `/wallets/${walletId}/orchard/balance/combined`
  );
  return response.data;
}

/**
 * Initiate an Orchard (shielded) transfer
 */
export async function initiateOrchardTransfer(
  request: OrchardTransferRequest
): Promise<OrchardTransferResponse> {
  const response = await axios.post<OrchardTransferResponse>(
    '/transfers/orchard',
    request
  );
  return response.data;
}

/**
 * Execute a pending Orchard transfer
 */
export async function executeOrchardTransfer(
  transferId: number
): Promise<OrchardTransferResponse> {
  const response = await axios.post<OrchardTransferResponse>(
    `/transfers/orchard/${transferId}/execute`
  );
  return response.data;
}

/**
 * Get Orchard transaction history
 */
export async function getOrchardTransactions(
  walletId: number,
  limit: number = 50
): Promise<OrchardTransactionInfo[]> {
  const response = await axios.get<OrchardTransactionInfo[]>(
    `/wallets/${walletId}/orchard/transactions`,
    { params: { limit } }
  );
  return response.data;
}

/**
 * Get scan progress
 */
export async function getScanProgress(): Promise<ScanProgress> {
  const response = await axios.get<ScanProgress>('/zcash/scan/status');
  return response.data;
}

/**
 * Trigger a sync of the Orchard scanner
 */
export async function syncOrchard(): Promise<ScanProgress> {
  const response = await axios.post<ScanProgress>('/zcash/scan/sync');
  return response.data;
}

/**
 * Parse a unified address to get its components
 */
export async function parseUnifiedAddress(
  address: string
): Promise<UnifiedAddressInfo> {
  const response = await axios.get<UnifiedAddressInfo>(
    '/zcash/address/parse',
    { params: { address } }
  );
  return response.data;
}

/**
 * Validate a unified address
 */
export async function validateUnifiedAddress(
  address: string
): Promise<{ valid: boolean; error?: string }> {
  try {
    const response = await axios.get<{ valid: boolean }>(
      '/zcash/address/validate',
      { params: { address } }
    );
    return response.data;
  } catch (error: any) {
    return {
      valid: false,
      error: error.response?.data?.message || 'Invalid address',
    };
  }
}

/**
 * Estimate fee for an Orchard transfer
 */
export async function estimateOrchardFee(
  request: Omit<OrchardTransferRequest, 'wallet_id'>
): Promise<{
  fee_zatoshis: number;
  fee_zec: string;
  num_actions: number;
}> {
  const response = await axios.post<{
    fee_zatoshis: number;
    fee_zec: string;
    num_actions: number;
  }>('/transfers/orchard/estimate-fee', request);
  return response.data;
}

/**
 * Check if Orchard is enabled for a wallet
 */
export async function isOrchardEnabled(walletId: number): Promise<boolean> {
  try {
    await getShieldedBalance(walletId);
    return true;
  } catch {
    return false;
  }
}

// Export all functions as a namespace for convenient imports
const orchardApi = {
  enableOrchard,
  getUnifiedAddresses,
  generateUnifiedAddress,
  getShieldedBalance,
  getCombinedBalance,
  initiateOrchardTransfer,
  executeOrchardTransfer,
  getOrchardTransactions,
  getScanProgress,
  syncOrchard,
  parseUnifiedAddress,
  validateUnifiedAddress,
  estimateOrchardFee,
  isOrchardEnabled,
};

export default orchardApi;
