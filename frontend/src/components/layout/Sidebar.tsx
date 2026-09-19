import { NavLink } from 'react-router-dom'
import { useAuth } from '../../auth/useAuth'
import { useCan } from '../../auth/useCan'
import type { Area } from '../../auth/permissions'
import { BrandMark } from '../BrandMark'

const BASE_NAV: { to: string; label: string; end: boolean; area: Area }[] = [
  { to: '/app', label: 'Dashboard', end: true, area: 'dashboard' },
  { to: '/inventory', label: 'Inventory', end: false, area: 'production' },
  { to: '/recipes', label: 'Recipes', end: false, area: 'production' },
  { to: '/batches', label: 'Batches', end: false, area: 'production' },
  { to: '/fermenters', label: 'Fermenters', end: true, area: 'production' },
  { to: '/fermenters/schedule', label: 'Schedule', end: false, area: 'production' },
  { to: '/calendar', label: 'Calendar', end: false, area: 'production' },
  { to: '/yeast-kinetics', label: 'Yeast Kinetics', end: false, area: 'production' },
]

const LIBRARY_NAV = [
  { to: '/library/styles', label: 'Beer Styles' },
  { to: '/library/equipment-profiles', label: 'Equipment' },
  { to: '/library/mash-profiles', label: 'Mash Profiles' },
  { to: '/library/yeasts', label: 'Yeasts' },
  { to: '/library/fermentables', label: 'Fermentables' },
]

const WATER_NAV = [
  { to: '/water/profiles', label: 'Profiles' },
  { to: '/water/calculator', label: 'Calculator' },
  { to: '/water/adjustments', label: 'Adjustments' },
]

const COMMERCIAL_NAV: { flag: string; to: string; label: string; area: Area }[] = [
  { flag: 'yeast_banking', to: '/yeast-bank', label: 'Yeast Bank', area: 'production' },
  { flag: 'tracking', to: '/container-assets', label: 'Container Assets', area: 'distribution' },
  { flag: 'reporting', to: '/cost-rates', label: 'Cost Rates', area: 'costs' },
  { flag: 'reporting', to: '/batch-costs', label: 'Batch Costs', area: 'costs' },
  { flag: 'reporting', to: '/cost-reports', label: 'Cost Reports', area: 'costs' },
  { flag: 'duty', to: '/duty', label: 'Beer Duty', area: 'duty' },
  { flag: 'labels', to: '/labels', label: 'Label Records', area: 'production' },
  { flag: 'label_design', to: '/label-design', label: 'Label Design', area: 'production' },
  { flag: 'packaging', to: '/packaging-runs', label: 'Packaging Runs', area: 'production' },
  { flag: 'packaging', to: '/distribution-movements', label: 'Distribution', area: 'distribution' },
  { flag: 'traceability', to: '/traceability', label: 'Traceability', area: 'production' },
  { flag: 'procurement', to: '/suppliers', label: 'Suppliers', area: 'production' },
  { flag: 'procurement', to: '/purchase-orders', label: 'Purchase Orders', area: 'production' },
  { flag: 'equipment_maintenance', to: '/equipment', label: 'Equipment', area: 'production' },
  { flag: 'equipment_maintenance', to: '/maintenance-due', label: 'Maintenance Due', area: 'production' },
]

/**
 * Left navigation sidebar with feature-flagged sections. Requires useAuth.
 */
export function Sidebar() {
  const { user } = useAuth()
  const can = useCan()
  const flags = user?.feature_flags ?? {}
  // Only show what the user's role may open.
  const commercial = COMMERCIAL_NAV.filter((item) => flags[item.flag] === true && can(item.area))

  const linkClass = ({ isActive }: { isActive: boolean }) =>
    `block px-3 py-1.5 rounded text-sm transition-colors ${
      isActive
        ? 'bg-[var(--color-accent)] text-white'
        : 'text-[var(--color-fg)] hover:bg-[var(--color-border)]'
    }`

  const hasWater = flags['water'] === true
  const hasCommercial = commercial.length > 0

  return (
    <nav
      className="flex flex-col gap-0.5 p-2 w-[210px] shrink-0 border-r h-full overflow-y-auto"
      style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
    >
      <NavLink to="/app" end className="px-2 py-2 mb-2 block no-underline">
        <BrandMark size={26} />
      </NavLink>

      {BASE_NAV.filter((item) => can(item.area)).map((item) => (
        <NavLink key={item.to} to={item.to} end={item.end} className={linkClass}>
          {item.label}
        </NavLink>
      ))}

      <div className="px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wider font-dm-mono text-[var(--color-muted)]">
        Library
      </div>
      {LIBRARY_NAV.map((item) => (
        <NavLink key={item.to} to={item.to} className={linkClass}>
          {item.label}
        </NavLink>
      ))}

      {hasWater && (
        <>
          <div className="px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wider font-dm-mono text-[var(--color-muted)]">
            Water
          </div>
          {WATER_NAV.map((item) => (
            <NavLink key={item.to} to={item.to} className={linkClass}>
              {item.label}
            </NavLink>
          ))}
        </>
      )}

      {hasCommercial && (
        <>
          <div className="px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wider font-dm-mono text-[var(--color-muted)]">
            Commercial
          </div>
          {commercial.map((item) => (
            <NavLink key={item.to} to={item.to} className={linkClass}>
              {item.label}
            </NavLink>
          ))}
        </>
      )}

      <div className="flex-1" />
      <div className="px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wider font-dm-mono text-[var(--color-muted)]">
        Settings
      </div>
      {can('audit') && (
        <NavLink to="/compliance-audit" className={linkClass}>
          Audit Log
        </NavLink>
      )}
      {can('members') && (
        <NavLink to="/members" className={linkClass}>
          Members
        </NavLink>
      )}
      <NavLink to="/account" className={linkClass}>
        Account
      </NavLink>
    </nav>
  )
}
