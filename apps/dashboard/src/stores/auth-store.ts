// ============================================================
// Auth Store — JWT token, user info, login/logout
// ============================================================

import { create } from 'zustand';
import { authApi } from '@/lib/api-client';
import type { AuthUser } from '@/lib/types';

const DEMO_USER: AuthUser = {
  id: 'demo-user',
  email: 'demo@masday.dev',
  name: 'Demo User',
  role: 'admin',
};

const isDevMode = process.env.NODE_ENV === 'development';

interface AuthState {
  token: string | null;
  user: AuthUser | null;
  isLoading: boolean;
  error: string | null;
  isAuthenticated: boolean;
  isDemo: boolean;
  login: (email: string, name: string) => Promise<void>;
  logout: () => void;
  checkAuth: () => Promise<void>;
  clearError: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  token: typeof window !== 'undefined' ? localStorage.getItem('auth_token') : null,
  user: null,
  isLoading: false,
  error: null,
  isAuthenticated: false,
  isDemo: false,

  login: async (email: string, name: string) => {
    set({ isLoading: true, error: null });
    try {
      const result = await authApi.login(email, name);
      localStorage.setItem('auth_token', result.token);
      set({
        token: result.token,
        user: result.user,
        isAuthenticated: true,
        isDemo: false,
        isLoading: false,
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Login failed';
      set({ error: message, isLoading: false });
      throw err;
    }
  },

  logout: () => {
    localStorage.removeItem('auth_token');
    set({ token: null, user: null, isAuthenticated: false, isDemo: false });
  },

  checkAuth: async () => {
    const token = typeof window !== 'undefined' ? localStorage.getItem('auth_token') : null;
    if (!token) {
      set({ isAuthenticated: false, user: null, isDemo: false });
      return;
    }
    try {
      const result = await authApi.getMe();
      set({ user: result.user, isAuthenticated: true, isDemo: false, token });
    } catch {
      if (isDevMode) {
        set({ user: DEMO_USER, isAuthenticated: true, isDemo: true, token });
      } else {
        localStorage.removeItem('auth_token');
        set({ token: null, user: null, isAuthenticated: false, isDemo: false });
      }
    }
  },

  clearError: () => set({ error: null }),
}));
