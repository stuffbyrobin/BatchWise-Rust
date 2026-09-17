/**
 * The permission table from the backend (`src/platform/authz.rs`). The server
 * enforces it; the UI uses this copy only to hide what a role cannot do, so
 * keep the two in sync.
 */

export type Role = 'owner' | 'manager' | 'brewer' | 'sales' | 'viewer'

export type Area =
  | 'account'
  | 'tenant'
  | 'dashboard'
  | 'production'
  | 'distribution'
  | 'sales'
  | 'costs'
  | 'duty'
  | 'audit'
  | 'members'

export type Access = 'none' | 'read' | 'write'

export const ROLES: Role[] = ['owner', 'manager', 'brewer', 'sales', 'viewer']

export const ROLE_LABELS: Record<Role, string> = {
  owner: 'Owner',
  manager: 'Manager',
  brewer: 'Brewer',
  sales: 'Sales',
  viewer: 'Viewer',
}

const TABLE: Record<Area, Record<Role, Access>> = {
  account: { owner: 'write', manager: 'write', brewer: 'write', sales: 'write', viewer: 'write' },
  tenant: { owner: 'write', manager: 'read', brewer: 'read', sales: 'read', viewer: 'read' },
  dashboard: { owner: 'read', manager: 'read', brewer: 'read', sales: 'read', viewer: 'read' },
  production: { owner: 'write', manager: 'write', brewer: 'write', sales: 'read', viewer: 'read' },
  distribution: { owner: 'write', manager: 'write', brewer: 'write', sales: 'write', viewer: 'read' },
  sales: { owner: 'write', manager: 'write', brewer: 'read', sales: 'write', viewer: 'read' },
  costs: { owner: 'write', manager: 'write', brewer: 'read', sales: 'none', viewer: 'read' },
  duty: { owner: 'write', manager: 'write', brewer: 'none', sales: 'none', viewer: 'read' },
  audit: { owner: 'read', manager: 'read', brewer: 'none', sales: 'none', viewer: 'read' },
  members: { owner: 'write', manager: 'write', brewer: 'none', sales: 'none', viewer: 'none' },
}

const RANK: Record<Access, number> = { none: 0, read: 1, write: 2 }

/** The most `role` may do in `area`. */
export function granted(role: Role, area: Area): Access {
  return TABLE[area][role]
}

/** Whether `role` has at least `access` in `area`; false without a role. */
export function can(role: Role | null | undefined, area: Area, access: 'read' | 'write' = 'read'): boolean {
  return role != null && RANK[TABLE[area][role]] >= RANK[access]
}

/** Brewer, Sales and Viewer: the members a Manager may manage. */
export const STAFF_ROLES: Role[] = ['brewer', 'sales', 'viewer']
