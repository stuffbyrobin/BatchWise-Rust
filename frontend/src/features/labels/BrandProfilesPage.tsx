import React from 'react'
import { Link } from 'react-router-dom'
import {
  useBrandProfiles,
  useCreateBrandProfile,
  useDeleteBrandProfile,
  useUploadAsset,
} from './hooks/useLabelDesign'
import type { components } from '../../api/generated'

type CreateBrandProfileRequest = components['schemas']['CreateBrandProfileRequest']

const FONTS = ['helvetica', 'times', 'courier'] as const

export function BrandProfilesPage() {
  const { data, isLoading, error } = useBrandProfiles()
  const create = useCreateBrandProfile()
  const del = useDeleteBrandProfile()
  const upload = useUploadAsset()

  const [form, setForm] = React.useState<{
    name: string
    primary_color: string
    secondary_color: string
    font_family: string
    logo_asset_id?: string
  }>({ name: '', primary_color: '#1a1a1a', secondary_color: '#ffffff', font_family: 'helvetica' })
  const [logoName, setLogoName] = React.useState<string>('')
  const [err, setErr] = React.useState<string | null>(null)

  function set<K extends string>(k: K, v: string) {
    setForm((f) => ({ ...f, [k]: v }))
  }

  async function handleLogo(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return
    setErr(null)
    try {
      const asset = await upload.mutateAsync(file)
      setForm((f) => ({ ...f, logo_asset_id: asset.id }))
      setLogoName(file.name)
    } catch (e) {
      setErr((e as Error).message)
    }
  }

  function handleCreate() {
    setErr(null)
    const body: CreateBrandProfileRequest = {
      name: form.name,
      primary_color: form.primary_color,
      secondary_color: form.secondary_color,
      font_family: form.font_family as CreateBrandProfileRequest['font_family'],
      logo_asset_id: form.logo_asset_id ?? null,
    }
    create.mutate(body, {
      onSuccess: () => {
        setForm({ name: '', primary_color: '#1a1a1a', secondary_color: '#ffffff', font_family: 'helvetica' })
        setLogoName('')
      },
      onError: (e) => setErr(e.message),
    })
  }

  return (
    <div className="p-6 max-w-3xl">
      <div className="flex items-center justify-between mb-4">
        <h1 className="text-xl font-bold">Brand Profiles</h1>
        <Link to="/label-design" className="text-sm text-[var(--color-accent)]">
          ← Designs
        </Link>
      </div>

      <div className="border rounded p-4 mb-6" style={{ borderColor: 'var(--color-border)' }}>
        <h2 className="font-semibold mb-3 text-sm">New brand profile</h2>
        {err && <p className="text-sm text-red-600 mb-2">{err}</p>}
        <div className="grid grid-cols-2 gap-3 text-sm">
          <label className="flex flex-col gap-1">
            Name
            <input
              value={form.name}
              onChange={(e) => set('name', e.target.value)}
              className="border rounded px-2 py-1"
              style={{ borderColor: 'var(--color-border)' }}
            />
          </label>
          <label className="flex flex-col gap-1">
            Font
            <select
              value={form.font_family}
              onChange={(e) => set('font_family', e.target.value)}
              className="border rounded px-2 py-1"
              style={{ borderColor: 'var(--color-border)' }}
            >
              {FONTS.map((f) => (
                <option key={f} value={f}>
                  {f}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1">
            Primary colour
            <input
              type="color"
              value={form.primary_color}
              onChange={(e) => set('primary_color', e.target.value)}
              className="h-9 w-full border rounded"
              style={{ borderColor: 'var(--color-border)' }}
            />
          </label>
          <label className="flex flex-col gap-1">
            Secondary colour
            <input
              type="color"
              value={form.secondary_color}
              onChange={(e) => set('secondary_color', e.target.value)}
              className="h-9 w-full border rounded"
              style={{ borderColor: 'var(--color-border)' }}
            />
          </label>
          <label className="flex flex-col gap-1 col-span-2">
            Logo (PNG/JPEG, ≤ 2 MiB)
            <input type="file" accept="image/png,image/jpeg" onChange={handleLogo} />
            {logoName && <span className="text-xs text-[var(--color-muted)]">Uploaded: {logoName}</span>}
          </label>
        </div>
        <button
          onClick={handleCreate}
          disabled={!form.name || create.isPending}
          className="mt-3 px-3 py-1.5 rounded text-sm text-white disabled:opacity-50"
          style={{ background: 'var(--color-accent)' }}
        >
          {create.isPending ? 'Saving…' : 'Create profile'}
        </button>
      </div>

      {isLoading && <p className="text-sm text-[var(--color-muted)]">Loading…</p>}
      {error && <p className="text-sm text-red-600">{error.message}</p>}
      {data && data.items && (
        <ul className="space-y-2">
          {data.items.map((p) => (
            <li
              key={p.id}
              className="flex items-center justify-between border rounded px-3 py-2 text-sm"
              style={{ borderColor: 'var(--color-border)' }}
            >
              <span className="flex items-center gap-2">
                <span
                  className="inline-block w-4 h-4 rounded"
                  style={{ background: p.primary_color }}
                />
                <strong>{p.name}</strong>
                <span className="text-[var(--color-muted)]">{p.font_family}</span>
                {p.logo_asset_id && <span className="text-xs text-[var(--color-muted)]">• logo</span>}
              </span>
              <button onClick={() => p.id && del.mutate(p.id)} className="text-red-600 text-xs">
                Delete
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

export default BrandProfilesPage
