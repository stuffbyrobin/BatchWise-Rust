import React, { useCallback, useEffect, useState } from 'react';
import { apiClient, _initTokenStore } from '../api/client';
import { tokenStore, useTokenStore } from './tokenStore';
import { AuthContext } from './useAuth';
import type { components } from '../api/generated';

type MeResponse = components['schemas']['MeResponse'];

interface AuthProviderProps {
  children: React.ReactNode;
}

// Wire the API client to the token store before any component renders.
_initTokenStore(
  tokenStore.getAccessToken,
  tokenStore.getRefreshToken,
  tokenStore.setTokens,
  tokenStore.clear,
);

/**
 * React provider that exposes user state and auth methods via AuthContext.
 * Loads the current user on mount; logout always clears the local session.
 */
export function AuthProvider({ children }: AuthProviderProps) {
  const [user, setUser] = useState<MeResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Load the current user. After a reload only the refresh token is present;
  // the /me call's 401 makes the API client refresh the access token first.
  useEffect(() => {
    if (tokenStore.getRefreshToken() || tokenStore.getAccessToken()) {
      apiClient
        .get<MeResponse>('/api/v1/auth/me')
        .then((data) => setUser(data))
        .catch(() => setUser(null))
        .finally(() => setIsLoading(false));
    } else {
      setIsLoading(false);
    }
  }, []);

  // When the API client clears the tokens (failed refresh), drop the user so
  // ProtectedRoute redirects to /login.
  useEffect(
    () =>
      useTokenStore.subscribe((state) => {
        if (!state.accessToken && !state.refreshToken) setUser(null);
      }),
    [],
  );

  const login = useCallback(
    async (email: string, password: string) => {
      const response = await apiClient.post<{
        access_token: string;
        refresh_token: string;
        expires_in: number;
      }>('/api/v1/auth/login', { email, password });
      tokenStore.setTokens(response.access_token, response.refresh_token);
      const userData = await apiClient.get<MeResponse>('/api/v1/auth/me');
      setUser(userData);
    },
    [],
  );

  const register = useCallback(
    async (email: string, password: string, displayName: string, tenantName: string) => {
      const response = await apiClient.post<{
        access_token: string;
        refresh_token: string;
        expires_in: number;
      }>('/api/v1/auth/register', {
        email,
        password,
        display_name: displayName,
        tenant_name: tenantName,
      });
      tokenStore.setTokens(response.access_token, response.refresh_token);
      const userData = await apiClient.get<MeResponse>('/api/v1/auth/me');
      setUser(userData);
    },
    [],
  );

  // Always clears the local session, even when the server call fails (offline,
  // token already expired): logging out must never leave the user signed in.
  const logout = useCallback(async () => {
    const refreshToken = tokenStore.getRefreshToken();
    try {
      if (refreshToken) {
        await apiClient.post('/api/v1/auth/logout', { refresh_token: refreshToken });
      }
    } finally {
      tokenStore.clear();
      setUser(null);
    }
  }, []);

  const updateMe = useCallback(async (payload: { display_name?: string; current_password?: string; new_password?: string }) => {
    const updated = await apiClient.patch<MeResponse>('/api/v1/auth/me', payload);
    setUser(updated);
    return updated;
  }, []);

  const deleteMe = useCallback(async () => {
    await apiClient.delete('/api/v1/auth/me');
    tokenStore.clear();
    setUser(null);
  }, []);

  const value = React.useMemo(
    () => ({ user, isLoading, login, register, logout, updateMe, deleteMe }),
    [user, isLoading, login, register, logout, updateMe, deleteMe],
  );

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  );
}
