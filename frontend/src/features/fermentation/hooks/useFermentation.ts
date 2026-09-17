import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'
import { qs } from '../../../api/qs'

type FermentationReading = components['schemas']['FermentationReading']
type FermentationReadingList = components['schemas']['FermentationReadingList']
type CreateFermentationReadingRequest = components['schemas']['CreateFermentationReadingRequest']
type PatchFermentationReadingRequest = components['schemas']['PatchFermentationReadingRequest']

export function useReadings(batchId: string, params: { stage?: string; page?: number; page_size?: number } = {}) {
  return useQuery<FermentationReadingList>({
    queryKey: ['fermentation', batchId, params],
    queryFn: ({ signal }) => apiClient.get<FermentationReadingList>(
        `/api/v1/batches/${batchId}/fermentation${qs(params as Record<string, unknown>)}`, { signal },
      ),
    enabled: !!batchId,
  })
}

export function useCreateReading(batchId: string) {
  const qc = useQueryClient()
  return useMutation<FermentationReading, Error, CreateFermentationReadingRequest>({
    mutationFn: (body) =>
      apiClient.post<FermentationReading>(`/api/v1/batches/${batchId}/fermentation`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['fermentation', batchId] }),
  })
}

export function usePatchReading(batchId: string, readingId: string) {
  const qc = useQueryClient()
  return useMutation<FermentationReading, Error, PatchFermentationReadingRequest>({
    mutationFn: (body) =>
      apiClient.patch<FermentationReading>(
        `/api/v1/batches/${batchId}/fermentation/${readingId}`,
        body,
      ),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['fermentation', batchId] }),
  })
}

export function useDeleteReading(batchId: string) {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (readingId) =>
      apiClient.delete<void>(`/api/v1/batches/${batchId}/fermentation/${readingId}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['fermentation', batchId] }),
  })
}
