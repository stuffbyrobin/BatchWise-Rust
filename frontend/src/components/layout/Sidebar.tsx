import { NavLink } from 'react-router-dom'
import { useAuth } from '../../auth/useAuth'

const BASE_NAV = [
  { to: '/app', label: 'Dashboard', end: true },
  { to: '/inventory', label: 'Inventory', end: false },
  { to: '/recipes', label: 'Recipes', end: false },
  { to: '/batches', label: 'Batches', end: false },
  { to: '/fermenters', label: 'Fermenters', end: true },
  { to: '/fermenters/schedule', label: 'Schedule', end: false },
  { to: '/calendar', label: 'Calendar', end: false },
  { to: '/yeast-kinetics', label: 'Yeast Kinetics', end: false },
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

const COMMERCIAL_NAV: { flag: string; to: string; label: string }[] = [
  { flag: 'yeast_banking', to: '/yeast-bank', label: 'Yeast Bank' },
  { flag: 'tracking', to: '/container-assets', label: 'Container Assets' },
  { flag: 'reporting', to: '/cost-rates', label: 'Cost Rates' },
  { flag: 'reporting', to: '/batch-costs', label: 'Batch Costs' },
  { flag: 'reporting', to: '/cost-reports', label: 'Cost Reports' },
  { flag: 'duty', to: '/duty', label: 'Beer Duty' },
  { flag: 'labels', to: '/labels', label: 'Label Records' },
  { flag: 'label_design', to: '/label-design', label: 'Label Design' },
  { flag: 'packaging', to: '/packaging-runs', label: 'Packaging Runs' },
  { flag: 'packaging', to: '/distribution-movements', label: 'Distribution' },
  { flag: 'traceability', to: '/traceability', label: 'Traceability' },
  { flag: 'procurement', to: '/suppliers', label: 'Suppliers' },
  { flag: 'procurement', to: '/purchase-orders', label: 'Purchase Orders' },
  { flag: 'equipment_maintenance', to: '/equipment', label: 'Equipment' },
  { flag: 'equipment_maintenance', to: '/maintenance-due', label: 'Maintenance Due' },
]

export function Sidebar() {
  const { user } = useAuth()
  const flags = user?.feature_flags ?? {}

  const linkClass = ({ isActive }: { isActive: boolean }) =>
    `block px-3 py-1.5 rounded text-sm transition-colors ${
      isActive
        ? 'bg-[var(--color-accent)] text-white'
        : 'text-[var(--color-fg)] hover:bg-[var(--color-border)]'
    }`

  const hasWater = flags['water'] === true
  const hasCommercial = COMMERCIAL_NAV.some((item) => flags[item.flag] === true)

  return (
    <nav
      className="flex flex-col gap-0.5 p-2 w-[210px] shrink-0 border-r h-full overflow-y-auto"
      style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
    >
      <div className="px-3 py-2 mb-1 font-bold text-[var(--color-accent)] tracking-wide text-base">
        Batchwise
      </div>

      {BASE_NAV.map((item) => (
        <NavLink key={item.to} to={item.to} end={item.end} className={linkClass}>
          {item.label}
        </NavLink>
      ))}

      <div className="px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wider text-[var(--color-muted)]">
        Library
      </div>
      {LIBRARY_NAV.map((item) => (
        <NavLink key={item.to} to={item.to} className={linkClass}>
          {item.label}
        </NavLink>
      ))}

      {hasWater && (
        <>
          <div className="px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wider text-[var(--color-muted)]">
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
          <div className="px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wider text-[var(--color-muted)]">
            Commercial
          </div>
          {COMMERCIAL_NAV.filter((item) => flags[item.flag] === true).map((item) => (
            <NavLink key={item.to} to={item.to} className={linkClass}>
              {item.label}
            </NavLink>
          ))}
        </>
      )}

      <div className="flex-1" />
      <div className="px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wider text-[var(--color-muted)]">
        Settings
      </div>
      <NavLink to="/compliance-audit" className={linkClass}>
        Audit Log
      </NavLink>
      <NavLink to="/account" className={linkClass}>
        Account
      </NavLink>
    </nav>
  )
}
