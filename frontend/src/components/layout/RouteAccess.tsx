import type { ReactNode } from 'react'
import { useLocation } from 'react-router-dom'
import { areaForPath, can, isViewOnlyPage, pageNeedsWrite, ROLE_LABELS, type Role } from '../../auth/permissions'
import { useAuth } from '../../auth/useAuth'

/**
 * Applies the role's access to the current page: pages the role cannot open
 * (or create and import pages it cannot use) show a notice instead, and pages
 * it can only read carry a read-only note. The server enforces the same rules.
 */
export function RouteAccess({ children }: { children: ReactNode }) {
  const { pathname } = useLocation()
  const { user } = useAuth()
  const role = (user?.role ?? null) as Role | null
  if (!role) return <>{children}</>

  const area = areaForPath(pathname)
  if (!can(role, area, pageNeedsWrite(pathname) ? 'write' : 'read')) {
    return (
      <div className="max-w-xl">
        <h1 className="text-lg font-semibold mb-2">No access</h1>
        <p className="text-sm text-[var(--color-muted)]">
          Your role ({ROLE_LABELS[role]}) cannot open this page. Ask an Owner or Manager if you need access.
        </p>
      </div>
    )
  }

  const readOnly = !can(role, area, 'write') && !isViewOnlyPage(pathname)
  return (
    <>
      {readOnly && (
        <p
          role="note"
          className="mb-4 rounded border border-[var(--color-border)] bg-[var(--color-surface)] px-3 py-2 text-sm text-[var(--color-muted)]"
        >
          You can view this page, but your role ({ROLE_LABELS[role]}) cannot make changes here.
        </p>
      )}
      {children}
    </>
  )
}
