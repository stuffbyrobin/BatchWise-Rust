import React from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { apiClient } from '../../api/client'
import {
  useLabelDesign,
  useCreateLabelDesign,
  usePatchLabelDesign,
  useBrandProfiles,
  useRenderModel,
  fetchRenderPDF,
} from './hooks/useLabelDesign'
import type { components } from '../../api/generated'

type CreateLabelDesignRequest = components['schemas']['CreateLabelDesignRequest']
type DesignOptions = components['schemas']['DesignOptions']

const KINDS = [
  { value: 'bottle', label: 'Bottle label' },
  { value: 'can', label: 'Can label' },
  { value: 'pump_clip', label: 'Pump clip' },
  { value: 'cask_lens', label: 'Cask lens' },
] as const

const SIZES: Record<string, { key: string; label: string }[]> = {
  bottle: [{ key: 'bottle_front_90x120', label: 'Front 90×120 mm' }],
  can: [{ key: 'can_wrap_200x100', label: 'Wrap 200×100 mm' }],
  pump_clip: [
    { key: 'pumpclip_round_114', label: 'Round 114 mm' },
    { key: 'pumpclip_rect_140x90', label: 'Rectangular 140×90 mm' },
  ],
  cask_lens: [{ key: 'lens_round_100', label: 'Round 100 mm' }],
}

const TEMPLATE_FOR_KIND: Record<string, string> = {
  bottle: 'compliance_standard',
  can: 'compliance_standard',
  pump_clip: 'clip_standard',
  cask_lens: 'clip_standard',
}

function isComplianceKind(kind: string): boolean {
  return kind === 'bottle' || kind === 'can'
}

type ListResp = { items?: Array<{ id?: string; name?: string; batch_number?: string }> }

export function LabelDesignEditorPage() {
  const { id } = useParams<{ id: string }>()
  const editing = !!id && id !== 'new'
  const navigate = useNavigate()

  const existing = useLabelDesign(editing ? id : undefined)
  const profiles = useBrandProfiles()
  const create = useCreateLabelDesign()
  const patch = usePatchLabelDesign(editing ? (id as string) : '')

  const batches = useQueryList('/api/v1/batches?page_size=100')
  const recipes = useQueryList('/api/v1/recipes?page_size=100')

  const [kind, setKind] = React.useState<string>('bottle')
  const [name, setName] = React.useState('')
  const [sourceId, setSourceId] = React.useState('')
  const [brandProfileId, setBrandProfileId] = React.useState('')
  const [sizeKey, setSizeKey] = React.useState('bottle_front_90x120')
  const [opts, setOpts] = React.useState<DesignOptions>({})
  const [err, setErr] = React.useState<string | null>(null)

  // Hydrate from an existing design.
  React.useEffect(() => {
    const d = existing.data
    if (!d) return
    setKind(d.kind ?? 'bottle')
    setName(d.name ?? '')
    setSourceId((d.batch_id ?? d.recipe_id ?? '') as string)
    setBrandProfileId((d.brand_profile_id ?? '') as string)
    setSizeKey(d.size_key ?? '')
    setOpts(d.options ?? {})
  }, [existing.data])

  // Reset size when kind changes (only for new designs).
  React.useEffect(() => {
    if (!editing) {
      setSizeKey(SIZES[kind]?.[0]?.key ?? '')
      setSourceId('')
    }
  }, [kind, editing])

  function toggle(k: keyof DesignOptions) {
    setOpts((o) => ({ ...o, [k]: !o[k] }))
  }

  function handleSave() {
    setErr(null)
    if (editing) {
      patch.mutate(
        { name, brand_profile_id: brandProfileId || null, size_key: sizeKey, template_key: TEMPLATE_FOR_KIND[kind], options: opts },
        { onError: (e) => setErr(e.message) },
      )
      return
    }
    const body: CreateLabelDesignRequest = {
      kind: kind as CreateLabelDesignRequest['kind'],
      name,
      size_key: sizeKey,
      template_key: TEMPLATE_FOR_KIND[kind],
      brand_profile_id: brandProfileId || null,
      options: opts,
    }
    if (isComplianceKind(kind)) body.batch_id = sourceId
    else body.recipe_id = sourceId
    create.mutate(body, {
      onSuccess: (d) => navigate(`/label-design/${d.id}`),
      onError: (e) => setErr(e.message),
    })
  }

  const sourceList: ListResp = isComplianceKind(kind) ? batches : recipes

  return (
    <div className="p-6 max-w-5xl">
      <div className="flex items-center justify-between mb-4">
        <h1 className="text-xl font-bold">{editing ? 'Edit design' : 'New design'}</h1>
        <Link to="/label-design" className="text-sm text-[var(--color-accent)]">
          ← Designs
        </Link>
      </div>

      {err && <p className="text-sm text-red-600 mb-3">{err}</p>}

      <div className="grid grid-cols-2 gap-6">
        {/* ── form ── */}
        <div className="space-y-3 text-sm">
          <label className="flex flex-col gap-1">
            Kind
            <select
              value={kind}
              onChange={(e) => setKind(e.target.value)}
              disabled={editing}
              className="border rounded px-2 py-1"
              style={{ borderColor: 'var(--color-border)' }}
            >
              {KINDS.map((k) => (
                <option key={k.value} value={k.value}>
                  {k.label}
                </option>
              ))}
            </select>
          </label>

          <label className="flex flex-col gap-1">
            Name
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="border rounded px-2 py-1"
              style={{ borderColor: 'var(--color-border)' }}
            />
          </label>

          {!editing && (
            <label className="flex flex-col gap-1">
              {isComplianceKind(kind) ? 'Batch' : 'Recipe'}
              <select
                value={sourceId}
                onChange={(e) => setSourceId(e.target.value)}
                className="border rounded px-2 py-1"
                style={{ borderColor: 'var(--color-border)' }}
              >
                <option value="">Select…</option>
                {(sourceList.items ?? []).map((it) => (
                  <option key={it.id} value={it.id}>
                    {it.name ?? it.batch_number ?? it.id}
                  </option>
                ))}
              </select>
            </label>
          )}

          <label className="flex flex-col gap-1">
            Brand profile
            <select
              value={brandProfileId}
              onChange={(e) => setBrandProfileId(e.target.value)}
              className="border rounded px-2 py-1"
              style={{ borderColor: 'var(--color-border)' }}
            >
              <option value="">Default (no branding)</option>
              {(profiles.data?.items ?? []).map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>

          <label className="flex flex-col gap-1">
            Size
            <select
              value={sizeKey}
              onChange={(e) => setSizeKey(e.target.value)}
              className="border rounded px-2 py-1"
              style={{ borderColor: 'var(--color-border)' }}
            >
              {(SIZES[kind] ?? []).map((s) => (
                <option key={s.key} value={s.key}>
                  {s.label}
                </option>
              ))}
            </select>
          </label>

          <fieldset className="border rounded p-2" style={{ borderColor: 'var(--color-border)' }}>
            <legend className="px-1 text-xs text-[var(--color-muted)]">Optional fields</legend>
            {isComplianceKind(kind) ? (
              <>
                <Toggle label="Ingredient list" checked={!!opts.show_ingredient_list} onChange={() => toggle('show_ingredient_list')} />
                <Toggle label="Energy (kJ/kcal)" checked={!!opts.show_energy} onChange={() => toggle('show_energy')} />
                <Toggle label="Alcohol units" checked={!!opts.show_units} onChange={() => toggle('show_units')} />
                <Toggle label="Drink responsibly" checked={!!opts.show_responsible_drinking} onChange={() => toggle('show_responsible_drinking')} />
              </>
            ) : (
              <Toggle label="Tasting notes" checked={!!opts.show_tasting_notes} onChange={() => toggle('show_tasting_notes')} />
            )}
          </fieldset>

          <button
            onClick={handleSave}
            disabled={!name || (!editing && !sourceId) || create.isPending || patch.isPending}
            className="px-3 py-1.5 rounded text-sm text-white disabled:opacity-50"
            style={{ background: 'var(--color-accent)' }}
          >
            {create.isPending || patch.isPending ? 'Saving…' : editing ? 'Save changes' : 'Create design'}
          </button>
        </div>

        {/* ── preview ── */}
        <div>{editing ? <Preview id={id as string} /> : <p className="text-sm text-[var(--color-muted)]">Save the design to preview and print.</p>}</div>
      </div>
    </div>
  )
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: () => void }) {
  return (
    <label className="flex items-center gap-2 py-0.5">
      <input type="checkbox" checked={checked} onChange={onChange} />
      {label}
    </label>
  )
}

// useQueryList is a tiny inline list fetch for source pickers.
function useQueryList(path: string): ListResp {
  const [data, setData] = React.useState<ListResp>({})
  React.useEffect(() => {
    let active = true
    apiClient
      .get<ListResp>(path)
      .then((r) => active && setData(r))
      .catch(() => active && setData({}))
    return () => {
      active = false
    }
  }, [path])
  return data
}

function Preview({ id }: { id: string }) {
  const { data: m, error } = useRenderModel(id)
  const [pdfUrl, setPdfUrl] = React.useState<string | null>(null)
  const [pdfErr, setPdfErr] = React.useState<string | null>(null)

  React.useEffect(() => {
    return () => {
      if (pdfUrl) URL.revokeObjectURL(pdfUrl)
    }
  }, [pdfUrl])

  async function openPdf(print: boolean) {
    setPdfErr(null)
    try {
      const url = await fetchRenderPDF(id)
      setPdfUrl(url)
      const w = window.open(url, '_blank')
      if (print && w) w.addEventListener('load', () => w.print())
    } catch (e) {
      setPdfErr((e as Error).message)
    }
  }

  if (error) {
    return (
      <div className="border rounded p-4 text-sm" style={{ borderColor: 'var(--color-border)' }}>
        <p className="text-red-600">{error.message}</p>
        <p className="text-[var(--color-muted)] mt-1">
          Compliance labels require an approved label record for the batch.
        </p>
      </div>
    )
  }
  if (!m) return <p className="text-sm text-[var(--color-muted)]">Loading preview…</p>

  const f = m.fields
  return (
    <div className="space-y-3">
      <div
        className="border rounded p-4 mx-auto"
        style={{
          borderColor: 'var(--color-border)',
          background: m.brand?.secondary_color ?? '#fff',
          color: m.brand?.primary_color ?? '#000',
          aspectRatio: `${m.width_mm} / ${m.height_mm}`,
          maxWidth: 320,
          borderRadius: m.shape === 'circle' ? '50%' : undefined,
        }}
      >
        <div className="font-bold text-lg leading-tight">{f?.product_name}</div>
        {f?.style && <div className="text-sm italic">{f.style}</div>}
        <div className="text-sm">ABV {f?.abv_percent}%</div>
        {f?.allergens && f.allergens.length > 0 && (
          <div className="text-xs font-bold mt-1">Allergens: {f.allergens.join(', ')}</div>
        )}
        {f?.net_volume_ml != null && <div className="text-xs mt-1">{f.net_volume_ml} ml</div>}
        {f?.best_before_date && <div className="text-xs">Best before: {f.best_before_date}</div>}
        {f?.lot_identifier && <div className="text-xs">Lot: {f.lot_identifier}</div>}
        {m.options?.show_tasting_notes && f?.tasting && (
          <div className="text-xs mt-1">
            {f.tasting.aroma && <div>Aroma: {f.tasting.aroma}</div>}
            {f.tasting.flavour && <div>Flavour: {f.tasting.flavour}</div>}
          </div>
        )}
      </div>
      <p className="text-xs text-[var(--color-muted)] text-center">
        On-screen preview — the PDF is the print-accurate artifact. Compliance fields are locked from the
        approved label record.
      </p>
      <div className="flex gap-2 justify-center">
        <button
          onClick={() => openPdf(false)}
          className="px-3 py-1.5 rounded text-sm text-white"
          style={{ background: 'var(--color-accent)' }}
        >
          Open PDF
        </button>
        <button
          onClick={() => openPdf(true)}
          className="px-3 py-1.5 rounded text-sm border"
          style={{ borderColor: 'var(--color-border)' }}
        >
          Print
        </button>
      </div>
      {pdfErr && <p className="text-xs text-red-600 text-center">{pdfErr}</p>}
    </div>
  )
}

export default LabelDesignEditorPage
