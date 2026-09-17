import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'
import { qs } from '../../../api/qs'

type Ingredient = components['schemas']['Ingredient']
type CreateIngredientRequest = components['schemas']['CreateIngredientRequest']
type PatchIngredientRequest = components['schemas']['PatchIngredientRequest']
type StockInRequest = components['schemas']['StockInRequest']
type DeductRequest = components['schemas']['DeductRequest']
type DeductResult = components['schemas']['DeductResult']
type StockMovement = components['schemas']['StockMovement']
type InventorySummaryRow = components['schemas']['InventorySummaryRow']

interface ListParams {
  type?: string
  name?: string
  lot_number?: string
  expiring_within_days?: number
  out_of_stock?: boolean
  sort?: string
  page?: number
  page_size?: number
}

interface PaginatedIngredients {
  items: Ingredient[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

interface PaginatedMovements {
  items: StockMovement[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

interface SummaryPage {
  items: InventorySummaryRow[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

export function useInventoryList(params: ListParams = {}) {
  return useQuery<PaginatedIngredients>({
    queryKey: ['inventory', 'list', params],
    queryFn: ({ signal }) => apiClient.get<PaginatedIngredients>(`/api/v1/inventory${qs(params as Record<string, unknown>)}`, { signal }),
  })
}

export function useInventoryItem(id: string) {
  return useQuery<Ingredient>({
    queryKey: ['inventory', id],
    queryFn: ({ signal }) => apiClient.get<Ingredient>(`/api/v1/inventory/${id}`, { signal }),
    enabled: !!id,
  })
}

export function useInventoryCreate() {
  const qc = useQueryClient()
  return useMutation<Ingredient, Error, CreateIngredientRequest>({
    mutationFn: (body) => apiClient.post<Ingredient>('/api/v1/inventory', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['inventory'] }),
  })
}

export function useInventoryUpdate(id: string) {
  const qc = useQueryClient()
  return useMutation<Ingredient, Error, PatchIngredientRequest>({
    mutationFn: (body) => apiClient.patch<Ingredient>(`/api/v1/inventory/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['inventory'] }),
  })
}

export function useInventoryDelete(id: string) {
  const qc = useQueryClient()
  return useMutation<void, Error>({
    mutationFn: () => apiClient.delete<void>(`/api/v1/inventory/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['inventory'] }),
  })
}

export function useStockIn(id: string) {
  const qc = useQueryClient()
  return useMutation<Ingredient, Error, StockInRequest>({
    mutationFn: (body) => apiClient.post<Ingredient>(`/api/v1/inventory/${id}/stock`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['inventory'] }),
  })
}

export function useDeduct() {
  const qc = useQueryClient()
  return useMutation<DeductResult, Error, DeductRequest>({
    mutationFn: (body) => apiClient.post<DeductResult>('/api/v1/inventory/deduct', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['inventory'] }),
  })
}

export function useInventorySummary(params: { type?: string; page?: number; page_size?: number } = {}) {
  return useQuery<SummaryPage>({
    queryKey: ['inventory', 'summary', params],
    queryFn: ({ signal }) => apiClient.get<SummaryPage>(`/api/v1/inventory/summary${qs(params as Record<string, unknown>)}`, { signal }),
  })
}

export function useStockMovements(
  params: { ingredient_id?: string; reference_type?: string; page?: number; page_size?: number } = {},
) {
  return useQuery<PaginatedMovements>({
    queryKey: ['inventory', 'movements', params],
    queryFn: ({ signal }) => apiClient.get<PaginatedMovements>(
        `/api/v1/inventory/stock-movements${qs(params as Record<string, unknown>)}`, { signal },
      ),
  })
}
