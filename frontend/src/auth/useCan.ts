import { useCallback } from 'react'
import { can, type Area, type Role } from './permissions'
import { useAuth } from './useAuth'

/**
 * Returns `can(area, access)` for the signed-in user's role, for hiding what the
 * role cannot do. The server still enforces every rule.
 */
export function useCan() {
  const { user } = useAuth()
  const role = (user?.role ?? null) as Role | null
  return useCallback((area: Area, access: 'read' | 'write' = 'read') => can(role, area, access), [role])
}
