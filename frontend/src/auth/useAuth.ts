import React from 'react';
import type { components } from '../api/generated';

type MeResponse = components['schemas']['MeResponse'];

interface UpdateMePayload {
  display_name?: string;
  current_password?: string;
  new_password?: string;
}

interface AuthContextValue {
  user: MeResponse | null;
  isLoading: boolean;
  login: (email: string, password: string) => Promise<void>;
  register: (email: string, password: string, displayName: string, tenantName: string) => Promise<void>;
  logout: () => Promise<void>;
  updateMe: (payload: UpdateMePayload) => Promise<MeResponse>;
  deleteMe: () => Promise<void>;
}

export const AuthContext = React.createContext<AuthContextValue | null>(null);

export function useAuth(): AuthContextValue {
  const context = React.useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
