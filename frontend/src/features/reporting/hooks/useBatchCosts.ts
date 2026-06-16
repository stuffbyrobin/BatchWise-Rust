import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type BatchCost = components['schemas']['BatchCost']
type BatchCostPage = components['schemas']['BatchCostPage']
type ComputeBatchCostRequest = components['schemas']['ComputeBatchCostRequest']

function toQueryString(params: Record<string, unknown>): string {
  const q = Object.entries(params).filter(([, v]) => v !== undefined && v !== null && v !== '').map(([k, v]) => k + '=' + encodeURIComponent(String(v))).join('&')
  return q ? '?' + q : ''
}

export function useBatchCostsList(params: { page?: number; page_size?: number } = {}) {
  return useQuery<BatchCostPage>({
    queryKey: ['batch-costs', params],
    queryFn: () =>
      apiClient.get<BatchCostPage>(`/api/v1/batch-costs${toQueryString(params as Record<string, unknown>)}`),
  })
}

export function useBatchCost(batchId: string) {
  return useQuery<BatchCost>({
    queryKey: ['batch-costs', batchId],
    queryFn: () => apiClient.get<BatchCost>(`/api/v1/batch-costs/${batchId}`),
    enabled: !!batchId,
  })
}

export function useComputeBatchCost() {
  const qc = useQueryClient()
  return useMutation<BatchCost, Error, ComputeBatchCostRequest>({
    mutationFn: (body) => apiClient.post<BatchCost>('/api/v1/batch-costs/compute', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['batch-costs'] }),
  })
}
