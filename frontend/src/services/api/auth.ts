import api from './axios';
import { LoginResponse, User } from '../../types';

export const authService = {
  async login(username: string, password: string): Promise<LoginResponse> {
    return api.post('/auth/login', { username, password });
  },

  async logout(): Promise<void> {
    return api.post('/auth/logout');
  },

  async changePassword(oldPassword: string, newPassword: string): Promise<void> {
    return api.put('/auth/password', {
      old_password: oldPassword,
      new_password: newPassword,
    });
  },

  async getMe(): Promise<User> {
    return api.get('/auth/me');
  },
};
