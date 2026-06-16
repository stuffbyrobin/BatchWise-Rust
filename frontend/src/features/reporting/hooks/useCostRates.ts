import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type CostRate = components['schemas']['CostRate']
type CostRatePage = components['schemas']['CostRatePage']
type CreateCostRateRequest = components['schemas']['CreateCostRateRequest']
type PatchCostRateRequest = components['schemas']['PatchCostRateRequest']

function toQueryString(params: Record<string, unknown>): string {
  const q = Object.entries(params).filter(([, v]) => v !== undefined && v !== null && v !== '').map(([k, v]) => k + '=' + encodeURIComponent(String(v))).join('&')
  return q ? '?' + q : ''
}

export function useCostRatesList(params: { rate_type?: string; page?: number; page_size?: number } = {}) {
  return useQuery<CostRatePage>({
    queryKey: ['cost-rates', params],
    queryFn: () =>
      apiClient.get<CostRatePage>(`/api/v1/cost-rates${toQueryString(params as Record<string, unknown>)}`),
  })
}

export function useCostRate(id: string) {
  return useQuery<CostRate>({
    queryKey: ['cost-rates', id],
    queryFn: () => apiClient.get<CostRate>(`/api/v1/cost-rates/${id}`),
    enabled: !!id,
  })
}

export function useCreateCostRate() {
  const qc = useQueryClient()
  return useMutation<CostRate, Error, CreateCostRateRequest>({
    mutationFn: (body) => apiClient.post<CostRate>('/api/v1/cost-rates', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['cost-rates'] }),
  })
}

export function usePatchCostRate(id: string) {
  const qc = useQueryClient()
  return useMutation<CostRate, Error, PatchCostRateRequest>({
    mutationFn: (body) => apiClient.patch<CostRate>(`/api/v1/cost-rates/${id}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['cost-rates', id] })
      qc.invalidateQueries({ queryKey: ['cost-rates'] })
    },
  })
}

export function useReplaceCostRate(id: string) {
  const qc = useQueryClient()
  return useMutation<CostRate, Error, CreateCostRateRequest>({
    mutationFn: (body) => apiClient.put<CostRate>(`/api/v1/cost-rates/${id}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['cost-rates', id] })
      qc.invalidateQueries({ queryKey: ['cost-rates'] })
    },
  })
}

export function useDeleteCostRate() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/cost-rates/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['cost-rates'] }),
  })
}

export const RATE_TYPES = ['energy','labor','water','duty','overhead'] as const
export type RateType = typeof RATE_TYPES[number]
