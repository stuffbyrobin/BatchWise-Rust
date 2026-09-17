import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';

interface TokenState {
  accessToken: string | null;
  refreshToken: string | null;
  isAuthenticated: boolean;
  setTokens: (access: string, refresh: string) => void;
  clear: () => void;
}

/**
 * Zustand store for auth tokens. Only the refresh token is persisted to sessionStorage;
 * the access token is memory-only.
 */
export const useTokenStore = create<TokenState>()(
  persist(
    (set) => ({
      accessToken: null,
      refreshToken: null,
      isAuthenticated: false,
      setTokens: (access, refresh) =>
        set({ accessToken: access, refreshToken: refresh, isAuthenticated: true }),
      clear: () =>
        set({ accessToken: null, refreshToken: null, isAuthenticated: false }),
    }),
    {
      name: 'batchwise-auth',
      storage: createJSONStorage(() => sessionStorage),
      // Only the refresh token survives a reload; the access token lives in
      // memory and is re-issued from the refresh token when needed.
      partialize: (state) => ({ refreshToken: state.refreshToken }),
    },
  ),
);

/**
 * Imperative getter/setter interface for the token store, used by the API client.
 */
export const tokenStore = {
  getAccessToken: () => useTokenStore.getState().accessToken,
  getRefreshToken: () => useTokenStore.getState().refreshToken,
  setTokens: (access: string, refresh: string) =>
    useTokenStore.getState().setTokens(access, refresh),
  clear: () => useTokenStore.getState().clear(),
};

// Expose test helpers on window for Playwright (dev server only, never in a build)
if (import.meta.env.DEV && import.meta.env.VITE_TEST_MODE === 'true') {
  (window as unknown as Record<string, unknown>).__batchwise = {
    getToken: () => useTokenStore.getState().accessToken,
    setToken: (token: string) => {
      const refresh = useTokenStore.getState().refreshToken ?? ''
      useTokenStore.getState().setTokens(token, refresh)
    },
  }
}
