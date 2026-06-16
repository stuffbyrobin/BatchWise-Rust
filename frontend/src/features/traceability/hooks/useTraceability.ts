import { useQuery } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type ForwardTrace = components['schemas']['ForwardTrace']
type BackwardTrace = components['schemas']['BackwardTrace']
type RecallScope = components['schemas']['RecallScope']

export function useTraceIngredientLot(lotNumber: string) {
  return useQuery<ForwardTrace>({
    queryKey: ['trace', 'ingredient-lot', lotNumber],
    queryFn: () => apiClient.get<ForwardTrace>(`/api/v1/traceability/ingredient-lots/${encodeURIComponent(lotNumber)}`),
    enabled: !!lotNumber,
  })
}

export function useTracePackagingRun(id: string) {
  return useQuery<BackwardTrace>({
    queryKey: ['trace', 'packaging-run', id],
    queryFn: () => apiClient.get<BackwardTrace>(`/api/v1/traceability/packaging-runs/${id}`),
    enabled: !!id,
  })
}

export function useRecallScope(lotNumber: string) {
  return useQuery<RecallScope>({
    queryKey: ['trace', 'recall', lotNumber],
    queryFn: () => apiClient.get<RecallScope>(`/api/v1/traceability/recall?lot_number=${encodeURIComponent(lotNumber)}`),
    enabled: !!lotNumber,
  })
}
