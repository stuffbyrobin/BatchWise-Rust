import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type FermentationReading = components['schemas']['FermentationReading']
type FermentationReadingList = components['schemas']['FermentationReadingList']
type CreateFermentationReadingRequest = components['schemas']['CreateFermentationReadingRequest']
type PatchFermentationReadingRequest = components['schemas']['PatchFermentationReadingRequest']

function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

export function useReadings(batchId: string, params: { stage?: string; page?: number; page_size?: number } = {}) {
  return useQuery<FermentationReadingList>({
    queryKey: ['fermentation', batchId, params],
    queryFn: () =>
      apiClient.get<FermentationReadingList>(
        `/api/v1/batches/${batchId}/fermentation${qs(params as Record<string, unknown>)}`,
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
