import React from 'react'
import { APIError } from '../../api/error'
import { useTraceIngredientLot, useTracePackagingRun, useRecallScope } from './hooks/useTraceability'
import type { components } from '../../api/generated'

type ForwardTrace = components['schemas']['ForwardTrace']
type BackwardTrace = components['schemas']['BackwardTrace']
type RecallScope = components['schemas']['RecallScope']

function fmtDate(s: string | null | undefined): string {
  if (!s) return '—'
  return String(s).slice(0, 10)
}

function ForwardTraceView({ trace }: { trace: ForwardTrace }) {
  return (
    <div className="mt-4">
      <div className="p-3 rounded bg-[var(--color-surface)] border mb-3">
        <h3 className="font-medium text-sm">Ingredient Lot</h3>
        <p className="text-sm">{trace.ingredient?.name} — <span className="text-[var(--color-muted)]">{trace.lot_number}</span></p>
        {trace.ingredient?.allergens && trace.ingredient.allergens.length > 0 && (
          <p className="text-xs text-[var(--color-muted)]">Allergens: {trace.ingredient.allergens.join(', ')}</p>
        )}
      </div>
      {(!trace.batches || trace.batches.length === 0) ? (
        <p className="text-sm text-[var(--color-muted)]">No batches found for this lot.</p>
      ) : trace.batches.map((bNode, i) => (
        <div key={i} className="ml-4 border-l-2 border-[var(--color-border)] pl-3 mb-3">
          <p className="font-medium text-sm">Batch: {bNode.batch?.batch_number} — {bNode.batch?.name}
            <span className="ml-2 text-xs text-[var(--color-muted)]">{bNode.batch?.status}</span>
          </p>
          {(!bNode.packaging_runs || bNode.packaging_runs.length === 0) ? (
            <p className="text-xs text-[var(--color-muted)]">No packaging runs.</p>
          ) : bNode.packaging_runs.map((pNode, j) => (
            <div key={j} className="ml-4 border-l-2 border-[var(--color-border)] pl-3 mt-1">
              <p className="text-sm">
                {pNode.run?.format} — Lot <strong>{pNode.run?.lot_number}</strong>
                {' '}({pNode.run?.stock_remaining}/{pNode.run?.quantity} remaining)
              </p>
              {pNode.movements && pNode.movements.length > 0 && (
                <ul className="mt-1 text-xs text-[var(--color-muted)] space-y-0.5">
                  {pNode.movements.map((m, k) => (
                    <li key={k}>{fmtDate(m.moved_at)} · {m.movement_type} · {m.quantity} → {m.to_location}
                      {m.order_number && ` (${m.order_number})`}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ))}
        </div>
      ))}
    </div>
  )
}

function BackwardTraceView({ trace }: { trace: BackwardTrace }) {
  return (
    <div className="mt-4">
      <div className="p-3 rounded bg-[var(--color-surface)] border mb-3">
        <h3 className="font-medium text-sm">Packaging Run</h3>
        <p className="text-sm">Lot <strong>{trace.run?.lot_number}</strong> — {trace.run?.format} ({trace.run?.quantity} units)</p>
        <p className="text-xs text-[var(--color-muted)]">Packaged {fmtDate(trace.run?.packaged_at)}</p>
      </div>
      <div className="p-3 rounded bg-[var(--color-surface)] border mb-3">
        <h3 className="font-medium text-sm">Batch</h3>
        <p className="text-sm">{trace.batch?.batch_number} — {trace.batch?.name}
          <span className="ml-2 text-xs text-[var(--color-muted)]">{trace.batch?.status}</span>
        </p>
      </div>
      <div className="p-3 rounded bg-[var(--color-surface)] border">
        <h3 className="font-medium text-sm mb-2">Ingredient Lots Used</h3>
        {(!trace.ingredient_lots || trace.ingredient_lots.length === 0) ? (
          <p className="text-sm text-[var(--color-muted)]">No ingredient lots recorded.</p>
        ) : (
          <ul className="text-sm space-y-1">
            {trace.ingredient_lots.map((lot, i) => (
              <li key={i}>
                <strong>{lot.lot_number}</strong> — {lot.name} ({lot.type})
                {lot.supplier && <span className="text-[var(--color-muted)]"> · {lot.supplier}</span>}
                {lot.best_before_date && <span className="text-[var(--color-muted)]"> · BBE {fmtDate(lot.best_before_date)}</span>}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}

function RecallScopeView({ scope, lotNumber }: { scope: RecallScope; lotNumber: string }) {
  return (
    <div className="mt-4 border border-red-200 rounded p-4 bg-red-50">
      <h3 className="font-semibold text-sm text-red-700 mb-2">Recall Scope — {lotNumber}</h3>
      <div className="grid grid-cols-3 gap-3 mb-3 text-sm">
        <div className="text-center">
          <p className="text-2xl font-bold text-red-600">{scope.affected_batches}</p>
          <p className="text-xs text-[var(--color-muted)]">Batches</p>
        </div>
        <div className="text-center">
          <p className="text-2xl font-bold text-red-600">{scope.affected_packaging_runs}</p>
          <p className="text-xs text-[var(--color-muted)]">Packaging Runs</p>
        </div>
        <div className="text-center">
          <p className="text-2xl font-bold text-red-600">{scope.affected_orders}</p>
          <p className="text-xs text-[var(--color-muted)]">Orders</p>
        </div>
      </div>
      {scope.customers && scope.customers.length > 0 ? (
        <ul className="text-sm space-y-1">
          {scope.customers.map((c, i) => (
            <li key={i} className="flex flex-wrap gap-2 items-baseline">
              <span className="font-medium">{c.customer_name}</span>
              {c.email && <span className="text-[var(--color-muted)] text-xs">{c.email}</span>}
              {c.phone && <span className="text-[var(--color-muted)] text-xs">{c.phone}</span>}
              <span className="text-xs text-[var(--color-muted)]">Orders: {c.order_ids?.join(', ')}</span>
            </li>
          ))}
        </ul>
      ) : (
        <p className="text-sm text-[var(--color-muted)]">No customers affected.</p>
      )}
    </div>
  )
}

function IngredientLotTab() {
  const [input, setInput] = React.useState('')
  const [submitted, setSubmitted] = React.useState('')
  const [showRecall, setShowRecall] = React.useState(false)
  const traceQ = useTraceIngredientLot(submitted)
  const recallQ = useRecallScope(showRecall ? submitted : '')

  return (
    <div>
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault()
          setSubmitted(input)
          setShowRecall(false)
        }}
      >
        <input
          className="border rounded px-3 py-2 text-sm flex-1"
          placeholder="Ingredient lot number…"
          value={input}
          onChange={(e) => setInput(e.target.value)}
        />
        <button
          type="submit"
          className="px-3 py-2 rounded bg-[var(--color-accent)] text-white text-sm"
        >
          Trace
        </button>
      </form>

      {traceQ.isLoading && <p className="mt-3 text-[var(--color-muted)] text-sm">Loading…</p>}
      {traceQ.error && (
        <p className="mt-3 text-[var(--color-danger)] text-sm">
          {traceQ.error instanceof APIError ? traceQ.error.message : 'Not found or error.'}
        </p>
      )}
      {traceQ.data && (
        <>
          <ForwardTraceView trace={traceQ.data} />
          <button
            className="mt-3 px-3 py-1.5 rounded border border-red-400 text-red-600 text-sm hover:bg-red-50"
            onClick={() => setShowRecall((x) => !x)}
          >
            {showRecall ? 'Hide Recall' : 'Show Recall Scope'}
          </button>
          {showRecall && recallQ.data && (
            <RecallScopeView scope={recallQ.data} lotNumber={submitted} />
          )}
        </>
      )}
    </div>
  )
}

function PackagingRunTab() {
  const [input, setInput] = React.useState('')
  const [submitted, setSubmitted] = React.useState('')
  const traceQ = useTracePackagingRun(submitted)

  return (
    <div>
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault()
          setSubmitted(input)
        }}
      >
        <input
          className="border rounded px-3 py-2 text-sm flex-1"
          placeholder="Packaging run UUID…"
          value={input}
          onChange={(e) => setInput(e.target.value)}
        />
        <button
          type="submit"
          className="px-3 py-2 rounded bg-[var(--color-accent)] text-white text-sm"
        >
          Trace
        </button>
      </form>

      {traceQ.isLoading && <p className="mt-3 text-[var(--color-muted)] text-sm">Loading…</p>}
      {traceQ.error && (
        <p className="mt-3 text-[var(--color-danger)] text-sm">
          {traceQ.error instanceof APIError ? traceQ.error.message : 'Not found or error.'}
        </p>
      )}
      {traceQ.data && <BackwardTraceView trace={traceQ.data} />}
    </div>
  )
}

export default function TraceabilityPage() {
  const [tab, setTab] = React.useState<'ingredient' | 'packaging'>('ingredient')

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <h1 className="text-xl font-semibold mb-4">Traceability</h1>
      <div className="flex gap-2 mb-6 border-b">
        {(['ingredient', 'packaging'] as const).map((t) => (
          <button
            key={t}
            className={`px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors ${
              tab === t
                ? 'border-[var(--color-accent)] text-[var(--color-accent)]'
                : 'border-transparent text-[var(--color-muted)] hover:text-[var(--color-text)]'
            }`}
            onClick={() => setTab(t)}
          >
            {t === 'ingredient' ? 'Ingredient Lot Trace' : 'Packaging Run Trace'}
          </button>
        ))}
      </div>
      {tab === 'ingredient' ? <IngredientLotTab /> : <PackagingRunTab />}
    </div>
  )
}
