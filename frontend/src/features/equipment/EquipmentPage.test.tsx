import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { MemoryRouter } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import EquipmentPage from './EquipmentPage'
import MaintenanceDuePage from './MaintenanceDuePage'
import * as hooks from './hooks/useEquipment'

vi.mock('./hooks/useEquipment')

const idleMutation = { mutate: vi.fn(), mutateAsync: vi.fn(), isPending: false } as never

function listResult(items: unknown[]) {
  return { data: { items, total: items.length, page: 1, page_size: 20, total_pages: 1 }, isLoading: false, error: null } as never
}

function renderWith(ui: React.ReactElement) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>{ui}</MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('EquipmentPage', () => {
  beforeEach(() => {
    vi.mocked(hooks.useEquipmentList).mockReturnValue(listResult([]))
    vi.mocked(hooks.useCreateEquipment).mockReturnValue(idleMutation)
    vi.mocked(hooks.usePatchEquipment).mockReturnValue(idleMutation)
    vi.mocked(hooks.useDeleteEquipment).mockReturnValue(idleMutation)
  })

  it('renders the title', () => {
    renderWith(<EquipmentPage />)
    expect(screen.getByText('Equipment')).toBeInTheDocument()
  })

  it('shows an empty state', () => {
    renderWith(<EquipmentPage />)
    expect(screen.getByText(/No equipment yet/i)).toBeInTheDocument()
  })

  it('lists equipment rows with an overdue badge', () => {
    vi.mocked(hooks.useEquipmentList).mockReturnValue(
      listResult([
        { id: '1', name: 'Fermenter FV3', equipment_type: 'fermenter', status: 'active', overdue_schedule_count: 2 },
      ]),
    )
    renderWith(<EquipmentPage />)
    expect(screen.getByText('Fermenter FV3')).toBeInTheDocument()
    expect(screen.getByText(/2 overdue/i)).toBeInTheDocument()
  })
})

describe('MaintenanceDuePage', () => {
  beforeEach(() => {
    vi.mocked(hooks.useMaintenanceDue).mockReturnValue(listResult([]))
  })

  it('renders the title and empty state', () => {
    renderWith(<MaintenanceDuePage />)
    expect(screen.getByText('Maintenance Due')).toBeInTheDocument()
    expect(screen.getByText(/Nothing due in this window/i)).toBeInTheDocument()
  })

  it('renders a due row as overdue', () => {
    vi.mocked(hooks.useMaintenanceDue).mockReturnValue(
      listResult([
        {
          schedule_id: 's1', equipment_id: 'e1', equipment_name: 'Pump A', equipment_type: 'pump',
          task_name: 'Replace seals', interval_days: 30, last_performed_at: null,
          next_due_at: '2026-01-01T00:00:00Z', days_until_due: -40, is_overdue: true,
        },
      ]),
    )
    renderWith(<MaintenanceDuePage />)
    expect(screen.getByText('Pump A')).toBeInTheDocument()
    expect(screen.getByText(/Overdue 40d/i)).toBeInTheDocument()
  })
})
