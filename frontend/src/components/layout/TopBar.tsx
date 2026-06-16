import { useNavigate } from 'react-router-dom'
import { useAuth } from '../../auth/useAuth'

export function TopBar() {
  const { user, logout } = useAuth()
  const navigate = useNavigate()

  const handleLogout = async () => {
    await logout()
    navigate('/login')
  }

  return (
    <header
      className="flex items-center justify-between px-6 py-3 border-b"
      style={{ background: 'var(--color-surface)', borderColor: 'var(--color-border)' }}
    >
      <span className="font-semibold text-[var(--color-fg)]">{user?.tenant_name ?? ''}</span>
      <div className="flex items-center gap-4">
        <span className="text-sm text-[var(--color-muted)]">{user?.email ?? ''}</span>
        <button
          onClick={handleLogout}
          className="px-3 py-1 text-sm rounded border border-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-accent)] hover:text-white transition-colors"
        >
          Logout
        </button>
      </div>
    </header>
  )
}
