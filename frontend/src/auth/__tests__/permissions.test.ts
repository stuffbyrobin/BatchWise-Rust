import { describe, expect, it } from 'vitest'
import { areaForPath, can, granted, isViewOnlyPage, pageNeedsWrite, ROLES, type Access, type Area } from '../permissions'

describe('permissions', () => {
  // Same rows as grants_match_the_agreed_table in src/platform/authz.rs.
  // Columns: owner, manager, brewer, sales, viewer.
  const table: [Area, Access[]][] = [
    ['account', ['write', 'write', 'write', 'write', 'write']],
    ['tenant', ['write', 'read', 'read', 'read', 'read']],
    ['dashboard', ['read', 'read', 'read', 'read', 'read']],
    ['production', ['write', 'write', 'write', 'read', 'read']],
    ['distribution', ['write', 'write', 'write', 'write', 'read']],
    ['sales', ['write', 'write', 'read', 'write', 'read']],
    ['costs', ['write', 'write', 'read', 'none', 'read']],
    ['duty', ['write', 'write', 'none', 'none', 'read']],
    ['audit', ['read', 'read', 'none', 'none', 'read']],
    ['members', ['write', 'write', 'none', 'none', 'none']],
  ]

  it.each(table)('matches the backend table for %s', (area, row) => {
    ROLES.forEach((role, i) => expect(granted(role, area)).toBe(row[i]))
  })

  it('treats write as including read and refuses without a role', () => {
    expect(can('brewer', 'production', 'read')).toBe(true)
    expect(can('brewer', 'production', 'write')).toBe(true)
    expect(can('viewer', 'production', 'write')).toBe(false)
    expect(can('sales', 'costs')).toBe(false)
    expect(can(null, 'dashboard')).toBe(false)
  })

  it('maps app routes to areas', () => {
    expect(areaForPath('/cost-reports')).toBe('costs')
    expect(areaForPath('/container-assets/c1/qr')).toBe('distribution')
    expect(areaForPath('/duty')).toBe('duty')
    expect(areaForPath('/batches/b1')).toBe('production')
    expect(pageNeedsWrite('/recipes/new')).toBe(true)
    expect(pageNeedsWrite('/recipes/r1')).toBe(false)
    expect(isViewOnlyPage('/container-assets/c1/qr')).toBe(true)
    expect(isViewOnlyPage('/batches')).toBe(false)
  })
})
