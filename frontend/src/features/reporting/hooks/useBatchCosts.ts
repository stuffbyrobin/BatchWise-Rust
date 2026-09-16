import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'
import { qs } from '../../../api/qs'

type BatchCost = components['schemas']['BatchCost']
type BatchCostPage = components['schemas']['BatchCostPage']
type ComputeBatchCostRequest = components['schemas']['ComputeBatchCostRequest']

export function useBatchCostsList(params: { page?: number; page_size?: number } = {}) {
  return useQuery<BatchCostPage>({
    queryKey: ['batch-costs', params],
    queryFn: ({ signal }) => apiClient.get<BatchCostPage>(`/api/v1/batch-costs${qs(params as Record<string, unknown>)}`, { signal }),
  })
}

export function useBatchCost(batchId: string) {
  return useQuery<BatchCost>({
    queryKey: ['batch-costs', batchId],
    queryFn: ({ signal }) => apiClient.get<BatchCost>(`/api/v1/batch-costs/${batchId}`, { signal }),
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
