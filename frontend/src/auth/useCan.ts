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

/**
 * Whether the signed-in user is an Owner or Manager, who alone sign off
 * compliance records such as label approvals.
 */
export function useIsManager(): boolean {
  const { user } = useAuth()
  return user?.role === 'owner' || user?.role === 'manager'
}

/** Whether the signed-in user's role may make changes in `area`. */
export function useCanWrite(area: Area): boolean {
  const { user } = useAuth()
  return can((user?.role ?? null) as Role | null, area, 'write')
}
