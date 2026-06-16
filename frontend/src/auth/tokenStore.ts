import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';

interface TokenState {
  accessToken: string | null;
  refreshToken: string | null;
  isAuthenticated: boolean;
  setTokens: (access: string, refresh: string) => void;
  clear: () => void;
}

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
    },
  ),
);

export const tokenStore = {
  getAccessToken: () => useTokenStore.getState().accessToken,
  getRefreshToken: () => useTokenStore.getState().refreshToken,
  setTokens: (access: string, refresh: string) =>
    useTokenStore.getState().setTokens(access, refresh),
  clear: () => useTokenStore.getState().clear(),
};

// Expose test helpers on window when running under Playwright
if (import.meta.env.VITE_TEST_MODE === 'true') {
  (window as unknown as Record<string, unknown>).__batchwise = {
    getToken: () => useTokenStore.getState().accessToken,
    setToken: (token: string) => {
      const refresh = useTokenStore.getState().refreshToken ?? ''
      useTokenStore.getState().setTokens(token, refresh)
    },
  }
}
