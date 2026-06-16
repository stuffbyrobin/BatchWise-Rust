import React from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../../auth/useAuth'
import { useTenant, useUpdateTenant } from './hooks/useTenant'

export function AccountPage() {
  const { user, updateMe, deleteMe, logout } = useAuth()
  const navigate = useNavigate()
  const { data: tenant } = useTenant()
  const updateTenant = useUpdateTenant()

  // Display name
  const [displayName, setDisplayName] = React.useState(user?.display_name ?? '')
  const [profileSaving, setProfileSaving] = React.useState(false)
  const [profileMsg, setProfileMsg] = React.useState<{ ok: boolean; text: string } | null>(null)

  const handleSaveProfile = async (e: React.FormEvent) => {
    e.preventDefault()
    setProfileSaving(true)
    setProfileMsg(null)
    try {
      await updateMe({ display_name: displayName })
      setProfileMsg({ ok: true, text: 'Name updated.' })
    } catch (err) {
      setProfileMsg({ ok: false, text: err instanceof Error ? err.message : 'Save failed.' })
    } finally {
      setProfileSaving(false)
    }
  }

  // Change password
  const [currentPw, setCurrentPw] = React.useState('')
  const [newPw, setNewPw] = React.useState('')
  const [confirmPw, setConfirmPw] = React.useState('')
  const [pwSaving, setPwSaving] = React.useState(false)
  const [pwMsg, setPwMsg] = React.useState<{ ok: boolean; text: string } | null>(null)

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault()
    if (newPw !== confirmPw) {
      setPwMsg({ ok: false, text: 'New passwords do not match.' })
      return
    }
    if (newPw.length < 8) {
      setPwMsg({ ok: false, text: 'Password must be at least 8 characters.' })
      return
    }
    setPwSaving(true)
    setPwMsg(null)
    try {
      await updateMe({ current_password: currentPw, new_password: newPw })
      setCurrentPw('')
      setNewPw('')
      setConfirmPw('')
      setPwMsg({ ok: true, text: 'Password changed. You will be logged out now.' })
      setTimeout(() => logout().then(() => navigate('/login')), 1800)
    } catch (err) {
      setPwMsg({ ok: false, text: err instanceof Error ? err.message : 'Password change failed.' })
    } finally {
      setPwSaving(false)
    }
  }

  // Brewery settings
  const [nextBatchNum, setNextBatchNum] = React.useState<string>('')
  const [ibuMethod, setIbuMethod] = React.useState<'tinseth' | 'rager'>('tinseth')
  const [brewerySaving, setBrewerySaving] = React.useState(false)
  const [breweryMsg, setBreweryMsg] = React.useState<{ ok: boolean; text: string } | null>(null)

  React.useEffect(() => {
    if (tenant?.next_batch_number != null) setNextBatchNum(String(tenant.next_batch_number))
    if (tenant?.ibu_method) setIbuMethod(tenant.ibu_method as 'tinseth' | 'rager')
  }, [tenant?.next_batch_number, tenant?.ibu_method])

  // Brewery address
  const [address, setAddress] = React.useState<string>('')
  const [addressSaving, setAddressSaving] = React.useState(false)
  const [addressMsg, setAddressMsg] = React.useState<{ ok: boolean; text: string } | null>(null)

  React.useEffect(() => {
    if (tenant?.address != null) setAddress(tenant.address)
  }, [tenant?.address])

  const handleSaveAddress = async (e: React.FormEvent) => {
    e.preventDefault()
    setAddressSaving(true)
    setAddressMsg(null)
    try {
      await updateTenant.mutateAsync({ address })
      setAddressMsg({ ok: true, text: 'Saved.' })
    } catch (err) {
      setAddressMsg({ ok: false, text: err instanceof Error ? err.message : 'Save failed.' })
    } finally {
      setAddressSaving(false)
    }
  }

  // Beer Duty SPR production
  const [sbrProduction, setSbrProduction] = React.useState<string>('')
  const [sbrSaving, setSbrSaving] = React.useState(false)
  const [sbrMsg, setSbrMsg] = React.useState<{ ok: boolean; text: string } | null>(null)

  React.useEffect(() => {
    if (tenant?.sbr_annual_production_hl_pa != null) {
      setSbrProduction(String(tenant.sbr_annual_production_hl_pa))
    }
  }, [tenant?.sbr_annual_production_hl_pa])

  const handleSaveSbr = async (e: React.FormEvent) => {
    e.preventDefault()
    setSbrSaving(true)
    setSbrMsg(null)
    const val = sbrProduction === '' ? 0 : parseFloat(sbrProduction)
    try {
      await updateTenant.mutateAsync({ sbr_annual_production_hl_pa: val })
      setSbrMsg({ ok: true, text: 'Saved.' })
    } catch (err) {
      setSbrMsg({ ok: false, text: err instanceof Error ? err.message : 'Save failed.' })
    } finally {
      setSbrSaving(false)
    }
  }

  // Order numbers
  const [nextOrderNum, setNextOrderNum] = React.useState<string>('')
  const [orderNumSaving, setOrderNumSaving] = React.useState(false)
  const [orderNumMsg, setOrderNumMsg] = React.useState<{ ok: boolean; text: string } | null>(null)

  React.useEffect(() => {
    setNextOrderNum(tenant?.next_order_number != null ? String(tenant.next_order_number) : '')
  }, [tenant?.next_order_number])

  const handleSaveOrderNum = async (e: React.FormEvent) => {
    e.preventDefault()
    setOrderNumSaving(true)
    setOrderNumMsg(null)
    const val = nextOrderNum === '' ? undefined : parseInt(nextOrderNum, 10)
    try {
      await updateTenant.mutateAsync({ next_order_number: val })
      setOrderNumMsg({ ok: true, text: 'Saved.' })
    } catch (err) {
      setOrderNumMsg({ ok: false, text: err instanceof Error ? err.message : 'Save failed.' })
    } finally {
      setOrderNumSaving(false)
    }
  }

  const handleSaveBrewery = async (e: React.FormEvent) => {
    e.preventDefault()
    setBrewerySaving(true)
    setBreweryMsg(null)
    const val = nextBatchNum === '' ? null : parseInt(nextBatchNum, 10)
    try {
      await updateTenant.mutateAsync({ next_batch_number: val ?? undefined, ibu_method: ibuMethod })
      setBreweryMsg({ ok: true, text: 'Saved.' })
    } catch (err) {
      setBreweryMsg({ ok: false, text: err instanceof Error ? err.message : 'Save failed.' })
    } finally {
      setBrewerySaving(false)
    }
  }

  // Delete account
  const [deleteConfirmed, setDeleteConfirmed] = React.useState(false)
  const [deleting, setDeleting] = React.useState(false)
  const [deleteMsg, setDeleteMsg] = React.useState<string | null>(null)

  const handleDeleteAccount = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!deleteConfirmed) return
    setDeleting(true)
    setDeleteMsg(null)
    try {
      await deleteMe()
      navigate('/login')
    } catch (err) {
      setDeleteMsg(err instanceof Error ? err.message : 'Delete failed.')
      setDeleting(false)
    }
  }

  const inputCls = 'w-full rounded border px-3 py-2 text-sm bg-[var(--color-surface)] text-[var(--color-fg)] border-[var(--color-border)] focus:outline-none focus:border-[var(--color-accent)]'
  const labelCls = 'block text-xs font-medium text-[var(--color-muted)] mb-1'
  const sectionCls = 'rounded-lg border p-5 mb-5'

  return (
    <div className="max-w-xl mx-auto px-4 py-8">
      <h1 className="text-xl font-bold mb-6" style={{ color: 'var(--color-fg)' }}>Account Settings</h1>

      {/* Profile */}
      <div className={sectionCls} style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
        <h2 className="text-base font-semibold mb-4" style={{ color: 'var(--color-fg)' }}>Profile</h2>
        <div className="mb-3">
          <label className={labelCls}>Email</label>
          <p className="text-sm" style={{ color: 'var(--color-fg)' }}>{user?.email}</p>
        </div>
        <form onSubmit={handleSaveProfile}>
          <div className="mb-4">
            <label className={labelCls} htmlFor="displayName">Display Name</label>
            <input
              id="displayName"
              className={inputCls}
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              required
              minLength={1}
              maxLength={100}
            />
          </div>
          {profileMsg && (
            <p className={`text-xs mb-3 ${profileMsg.ok ? 'text-green-600' : 'text-[var(--color-danger)]'}`}>
              {profileMsg.text}
            </p>
          )}
          <button
            type="submit"
            disabled={profileSaving || displayName === user?.display_name}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {profileSaving ? 'Saving…' : 'Save Name'}
          </button>
        </form>
      </div>

      {/* Change password */}
      <div className={sectionCls} style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
        <h2 className="text-base font-semibold mb-4" style={{ color: 'var(--color-fg)' }}>Change Password</h2>
        <form onSubmit={handleChangePassword}>
          <div className="mb-3">
            <label className={labelCls} htmlFor="currentPw">Current Password</label>
            <input id="currentPw" type="password" className={inputCls} value={currentPw} onChange={(e) => setCurrentPw(e.target.value)} required autoComplete="current-password" />
          </div>
          <div className="mb-3">
            <label className={labelCls} htmlFor="newPw">New Password</label>
            <input id="newPw" type="password" className={inputCls} value={newPw} onChange={(e) => setNewPw(e.target.value)} required minLength={8} autoComplete="new-password" />
          </div>
          <div className="mb-4">
            <label className={labelCls} htmlFor="confirmPw">Confirm New Password</label>
            <input id="confirmPw" type="password" className={inputCls} value={confirmPw} onChange={(e) => setConfirmPw(e.target.value)} required autoComplete="new-password" />
          </div>
          {pwMsg && (
            <p className={`text-xs mb-3 ${pwMsg.ok ? 'text-green-600' : 'text-[var(--color-danger)]'}`}>
              {pwMsg.text}
            </p>
          )}
          <button
            type="submit"
            disabled={pwSaving || !currentPw || !newPw || !confirmPw}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {pwSaving ? 'Changing…' : 'Change Password'}
          </button>
        </form>
      </div>

      {/* Brewery settings */}
      <div className={sectionCls} style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
        <h2 className="text-base font-semibold mb-1" style={{ color: 'var(--color-fg)' }}>Brewery</h2>
        <p className="text-xs mb-4" style={{ color: 'var(--color-muted)' }}>
          Set the next batch number to use when creating a new batch. It auto-increments after each batch is created.
        </p>
        <form onSubmit={handleSaveBrewery}>
          <div className="mb-4">
            <label className={labelCls} htmlFor="nextBatchNum">Next Batch Number</label>
            <input
              id="nextBatchNum"
              type="number"
              min={1}
              step={1}
              placeholder="e.g. 42"
              className={inputCls}
              value={nextBatchNum}
              onChange={(e) => setNextBatchNum(e.target.value)}
            />
          </div>
          <div className="mb-4">
            <label className={labelCls} htmlFor="ibuMethod">IBU Calculation Method</label>
            <select
              id="ibuMethod"
              className={inputCls}
              value={ibuMethod}
              onChange={(e) => setIbuMethod(e.target.value as 'tinseth' | 'rager')}
            >
              <option value="tinseth">Tinseth (recommended for craft/homebrewing)</option>
              <option value="rager">Rager</option>
            </select>
          </div>
          {breweryMsg && (
            <p className={`text-xs mb-3 ${breweryMsg.ok ? 'text-green-600' : 'text-[var(--color-danger)]'}`}>
              {breweryMsg.text}
            </p>
          )}
          <button
            type="submit"
            disabled={brewerySaving}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {brewerySaving ? 'Saving…' : 'Save'}
          </button>
        </form>
      </div>

      {/* Order Numbers */}
      <div className={sectionCls} style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
        <h2 className="text-base font-semibold mb-1" style={{ color: 'var(--color-fg)' }}>Order Numbers</h2>
        <p className="text-xs mb-4" style={{ color: 'var(--color-muted)' }}>
          Set this to match your existing numbering so BatchWise orders continue the sequence.
        </p>
        <form onSubmit={handleSaveOrderNum}>
          <div className="mb-4">
            <label className={labelCls} htmlFor="nextOrderNum">Starting Order Number</label>
            <input
              id="nextOrderNum"
              type="number"
              min={1}
              step={1}
              placeholder={`e.g. ${tenant?.next_order_number ?? 1}`}
              className={inputCls}
              value={nextOrderNum}
              onChange={(e) => setNextOrderNum(e.target.value)}
            />
            <p className="text-xs mt-1" style={{ color: 'var(--color-muted)' }}>
              Current next order will be ORD-{nextOrderNum || (tenant?.next_order_number ?? 1)}.
            </p>
          </div>
          {orderNumMsg && (
            <p className={`text-xs mb-3 ${orderNumMsg.ok ? 'text-green-600' : 'text-[var(--color-danger)]'}`}>
              {orderNumMsg.text}
            </p>
          )}
          <button
            type="submit"
            disabled={orderNumSaving}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {orderNumSaving ? 'Saving…' : 'Save'}
          </button>
        </form>
      </div>

      {/* Brewery Address */}
      <div className={sectionCls} style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
        <h2 className="text-base font-semibold mb-1" style={{ color: 'var(--color-fg)' }}>Brewery Address</h2>
        <p className="text-xs mb-4" style={{ color: 'var(--color-muted)' }}>
          Used as the responsible party address on UK label records.
        </p>
        <form onSubmit={handleSaveAddress}>
          <div className="mb-4">
            <label className={labelCls} htmlFor="address">Address</label>
            <textarea
              id="address"
              rows={3}
              className={inputCls}
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder="e.g. 1 Brewery Lane, London, EC1A 1BB"
            />
          </div>
          {addressMsg && (
            <p className={`text-xs mb-3 ${addressMsg.ok ? 'text-green-600' : 'text-[var(--color-danger)]'}`}>
              {addressMsg.text}
            </p>
          )}
          <button
            type="submit"
            disabled={addressSaving}
            className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {addressSaving ? 'Saving…' : 'Save'}
          </button>
        </form>
      </div>

      {/* Beer Duty SPR — always shown per spec; field is relevant even before duty flag is enabled */}
      <div className={sectionCls} style={{ borderColor: 'var(--color-border)', background: 'var(--color-surface)' }}>
          <h2 className="text-base font-semibold mb-1" style={{ color: 'var(--color-fg)' }}>Beer Duty — Small Producer Relief</h2>
          <p className="text-xs mb-4" style={{ color: 'var(--color-muted)' }}>
            Enter your brewery's annual production in hectolitres of pure alcohol (hLPA) to enable
            Small Producer Relief calculations on duty returns. Leave at 0 if SPR does not apply.
          </p>
          <form onSubmit={handleSaveSbr}>
            <div className="mb-4">
              <label className={labelCls} htmlFor="sbrProduction">
                Annual Production (hLPA)
              </label>
              <input
                id="sbrProduction"
                type="number"
                min={0}
                step={0.1}
                placeholder="e.g. 1000"
                className={inputCls}
                value={sbrProduction}
                onChange={(e) => setSbrProduction(e.target.value)}
              />
              <p className="text-xs mt-1" style={{ color: 'var(--color-muted)' }}>
                ≤2100 hLPA = 50% relief · 2100–4500 hLPA = sliding scale · &gt;4500 = no relief
              </p>
            </div>
            {sbrMsg && (
              <p className={`text-xs mb-3 ${sbrMsg.ok ? 'text-green-600' : 'text-[var(--color-danger)]'}`}>
                {sbrMsg.text}
              </p>
            )}
            <button
              type="submit"
              disabled={sbrSaving}
              className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50"
            >
              {sbrSaving ? 'Saving…' : 'Save'}
            </button>
          </form>
        </div>

      {/* Danger zone */}
      <div className={sectionCls} style={{ borderColor: 'var(--color-danger)', background: 'var(--color-surface)' }}>
        <h2 className="text-base font-semibold mb-1" style={{ color: 'var(--color-danger)' }}>Danger Zone</h2>
        <p className="text-xs mb-4" style={{ color: 'var(--color-muted)' }}>
          Deactivating your account is permanent. You will lose access immediately. Your data is preserved but cannot be recovered without contacting support.
        </p>
        <form onSubmit={handleDeleteAccount}>
          <div className="flex items-center gap-2 mb-4">
            <input
              id="deleteCheck"
              type="checkbox"
              checked={deleteConfirmed}
              onChange={(e) => setDeleteConfirmed(e.target.checked)}
              className="w-4 h-4"
            />
            <label htmlFor="deleteCheck" className="text-xs" style={{ color: 'var(--color-fg)' }}>
              I understand this will deactivate my account
            </label>
          </div>
          {deleteMsg && (
            <p className="text-xs mb-3 text-[var(--color-danger)]">{deleteMsg}</p>
          )}
          <button
            type="submit"
            disabled={deleting || !deleteConfirmed}
            className="px-4 py-2 rounded text-sm text-white hover:opacity-90 disabled:opacity-50"
            style={{ background: 'var(--color-danger)' }}
          >
            {deleting ? 'Deactivating…' : 'Deactivate Account'}
          </button>
        </form>
      </div>
    </div>
  )
}
