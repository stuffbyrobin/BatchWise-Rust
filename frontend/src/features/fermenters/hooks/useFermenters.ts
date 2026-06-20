import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'

export interface Fermenter {
  id: string
  tenant_id: string
  name: string
  capacity_liters: number | null
  notes: string | null
  created_at: string
  updated_at: string
}

export interface PaginatedFermenters {
  items: Fermenter[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

export interface FermenterRequest {
  name?: string
  capacity_liters?: number | null
  notes?: string | null
}

interface ListParams {
  name?: string
  sort?: string
  page?: number
  page_size?: number
}

function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

export function useFermenters(params: ListParams = {}) {
  return useQuery<PaginatedFermenters>({
    queryKey: ['fermenters', params],
    queryFn: () => apiClient.get<PaginatedFermenters>(`/api/v1/fermenters${qs(params as Record<string, unknown>)}`),
  })
}

export function useCreateFermenter() {
  const qc = useQueryClient()
  return useMutation<Fermenter, Error, FermenterRequest>({
    mutationFn: (body) => apiClient.post<Fermenter>('/api/v1/fermenters', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['fermenters'] }),
  })
}

export function useUpdateFermenter(id: string) {
  const qc = useQueryClient()
  return useMutation<Fermenter, Error, FermenterRequest>({
    mutationFn: (body) => apiClient.patch<Fermenter>(`/api/v1/fermenters/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['fermenters'] }),
  })
}

export function useDeleteFermenter() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/fermenters/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['fermenters'] }),
  })
}
