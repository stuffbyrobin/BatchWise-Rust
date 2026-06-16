import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type Batch = components['schemas']['Batch']
type CreateBatchRequest = components['schemas']['CreateBatchRequest']
type UpdateBatchRequest = components['schemas']['UpdateBatchRequest']
type TransitionRequest = components['schemas']['TransitionRequest']
type PatchIngredientsRequest = components['schemas']['PatchIngredientsRequest']
type CreateBatchResponse = components['schemas']['CreateBatchResponse']
type PaginatedBatches = components['schemas']['PaginatedBatches']

export type { Batch, PatchIngredientsRequest }

function toQueryString(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

interface BatchListParams {
  status?: string
  recipe_id?: string
  brew_date_from?: string
  brew_date_to?: string
  page?: number
  page_size?: number
  sort?: string
}

export function useBatchesList(params: BatchListParams = {}) {
  return useQuery<PaginatedBatches>({
    queryKey: ['batches', params],
    queryFn: () =>
      apiClient.get<PaginatedBatches>(`/api/v1/batches${toQueryString(params as Record<string, unknown>)}`),
  })
}

export function useBatch(id: string) {
  return useQuery<Batch>({
    queryKey: ['batches', id],
    queryFn: () => apiClient.get<Batch>(`/api/v1/batches/${id}`),
    enabled: !!id,
  })
}

export function useCreateBatch() {
  const qc = useQueryClient()
  return useMutation<CreateBatchResponse, Error, CreateBatchRequest>({
    mutationFn: (body) => apiClient.post<CreateBatchResponse>('/api/v1/batches', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['batches'] }),
  })
}

export function useUpdateBatch(id: string) {
  const qc = useQueryClient()
  return useMutation<Batch, Error, UpdateBatchRequest>({
    mutationFn: (body) => apiClient.patch<Batch>(`/api/v1/batches/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['batches'] }),
  })
}

export function useDeleteBatch(id: string) {
  const qc = useQueryClient()
  return useMutation<void, Error, void>({
    mutationFn: () => apiClient.delete<void>(`/api/v1/batches/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['batches'] }),
  })
}

export function useTransitionBatch(id: string) {
  const qc = useQueryClient()
  return useMutation<Batch, Error, TransitionRequest>({
    mutationFn: (body) => apiClient.post<Batch>(`/api/v1/batches/${id}/transition`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['batches'] }),
  })
}

export function usePatchBatchIngredients(id: string) {
  const qc = useQueryClient()
  return useMutation<Batch, Error, PatchIngredientsRequest>({
    mutationFn: (body) => apiClient.patch<Batch>(`/api/v1/batches/${id}/ingredients`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['batches', id] }),
  })
}

export const BATCH_STATUSES = [
  'planned', 'brewing', 'fermenting', 'conditioning', 'packaging', 'completed', 'cancelled', 'spoiled',
] as const

export type BatchStatus = typeof BATCH_STATUSES[number]

export const ALLOWED_NEXT: Record<BatchStatus, BatchStatus[]> = {
  planned:      ['brewing', 'cancelled'],
  brewing:      ['fermenting', 'cancelled'],
  fermenting:   ['conditioning', 'cancelled', 'spoiled'],
  conditioning: ['packaging', 'cancelled', 'spoiled'],
  packaging:    ['completed', 'cancelled', 'spoiled'],
  completed:    ['spoiled'],
  cancelled:    [],
  spoiled:      [],
}

export const STATUS_LABELS: Record<BatchStatus, string> = {
  planned:      'Planned',
  brewing:      'Brewing',
  fermenting:   'Fermenting',
  conditioning: 'Conditioning',
  packaging:    'Packaging',
  completed:    'Completed',
  cancelled:    'Cancelled',
  spoiled:      'Spoiled',
}

export const STATUS_COLORS: Record<BatchStatus, string> = {
  planned:      'var(--srm-3)',
  brewing:      'var(--srm-5)',
  fermenting:   'var(--srm-6)',
  conditioning: 'var(--srm-7)',
  packaging:    'var(--srm-8)',
  completed:    'var(--color-success)',
  cancelled:    'var(--color-muted)',
  spoiled:      'var(--color-danger)',
}
