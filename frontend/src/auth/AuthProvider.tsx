import React, { useCallback, useEffect, useState } from 'react';
import { apiClient, _initTokenStore } from '../api/client';
import { tokenStore } from './tokenStore';
import { AuthContext } from './useAuth';
import type { components } from '../api/generated';

type MeResponse = components['schemas']['MeResponse'];

interface AuthProviderProps {
  children: React.ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [user, setUser] = useState<MeResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Initialize token store on mount
  useEffect(() => {
    _initTokenStore(
      tokenStore.getAccessToken,
      tokenStore.getRefreshToken,
      tokenStore.setTokens,
      tokenStore.clear,
    );

    // Check if we have a token and fetch user
    const accessToken = tokenStore.getAccessToken();
    if (accessToken) {
      apiClient
        .get<MeResponse>('/api/v1/auth/me')
        .then((data) => setUser(data))
        .catch(() => setUser(null))
        .finally(() => setIsLoading(false));
    } else {
      setIsLoading(false);
    }
  }, []);

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

  const logout = useCallback(async () => {
    await apiClient.post('/api/v1/auth/logout');
    tokenStore.clear();
    setUser(null);
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
