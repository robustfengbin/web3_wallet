import api from './axios';

export interface RpcPreset {
  id: string;
  name: string;
  url_template: string;
  requires_api_key: boolean;
  website: string;
}

export interface RpcConfig {
  primary_rpc: string;
  fallback_rpcs: string[];
}

export interface TestRpcResponse {
  success: boolean;
  latency_ms?: number;
  block_number?: number;
  error?: string;
}

export const settingsService = {
  getRpcPresets: (): Promise<RpcPreset[]> => {
    return api.get('/settings/rpc/presets');
  },

  getRpcConfig: (): Promise<RpcConfig> => {
    return api.get('/settings/rpc');
  },

  updateRpcConfig: (config: { primary_rpc: string; fallback_rpcs?: string[] }): Promise<{ message: string; restart_required: boolean }> => {
    return api.put('/settings/rpc', config);
  },

  testRpcEndpoint: (rpc_url: string): Promise<TestRpcResponse> => {
    return api.post('/settings/rpc/test', { rpc_url });
  },
};
