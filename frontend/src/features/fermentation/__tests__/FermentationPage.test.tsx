import { render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { FermentationPage } from '../FermentationPage'

const batchStatus = vi.fn()

vi.mock('../hooks/useFermentation', () => ({
  useReadings: () => ({
    data: { items: [{ id: 'r1', recorded_at: '2026-09-01T10:00:00Z', stage: 'primary', gravity: 1.05 }], total: 1 },
    isLoading: false,
    isError: false,
    refetch: vi.fn(),
  }),
  useCreateReading: () => ({ mutate: vi.fn(), isPending: false }),
  useDeleteReading: () => ({ mutate: vi.fn(), isPending: false }),
}))
vi.mock('../../batches/hooks/useBatches', () => ({
  useBatch: () => ({ data: { id: 'b1', status: batchStatus() } }),
}))

function renderPage() {
  return render(
    <MemoryRouter initialEntries={['/batches/b1/fermentation']}>
      <Routes>
        <Route path="/batches/:batchId/fermentation" element={<FermentationPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('FermentationPage', () => {
  beforeEach(() => batchStatus.mockReset())

  it('lets readings be logged and deleted while the batch is open', () => {
    batchStatus.mockReturnValue('fermenting')
    renderPage()
    expect(screen.getByRole('button', { name: 'Log Reading' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Delete' })).toBeInTheDocument()
  })

  it.each(['completed', 'cancelled', 'spoiled'])('is read-only for a %s batch', (status) => {
    batchStatus.mockReturnValue(status)
    renderPage()
    expect(screen.queryByRole('button', { name: 'Log Reading' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Delete' })).toBeNull()
    expect(screen.getByText(`This batch is ${status}, so its readings are read-only.`)).toBeInTheDocument()
  })
})
