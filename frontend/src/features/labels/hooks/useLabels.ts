import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type LabelRecord = components['schemas']['LabelRecord']
type LabelRecordList = components['schemas']['LabelRecordList']
type CreateLabelRecordRequest = components['schemas']['CreateLabelRecordRequest']
type PatchLabelRecordRequest = components['schemas']['PatchLabelRecordRequest']

function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

export function useLabelRecords(params: { batch_id?: string; status?: string; page?: number; page_size?: number } = {}) {
  return useQuery<LabelRecordList>({
    queryKey: ['label-records', params],
    queryFn: () =>
      apiClient.get<LabelRecordList>(
        `/api/v1/label-records${qs(params as Record<string, unknown>)}`,
      ),
  })
}

export function useCreateLabelRecord() {
  const qc = useQueryClient()
  return useMutation<LabelRecord, Error, CreateLabelRecordRequest>({
    mutationFn: (body) => apiClient.post<LabelRecord>('/api/v1/label-records', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['label-records'] }),
  })
}

export function usePatchLabelRecord(id: string) {
  const qc = useQueryClient()
  return useMutation<LabelRecord, Error, PatchLabelRecordRequest>({
    mutationFn: (body) => apiClient.patch<LabelRecord>(`/api/v1/label-records/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['label-records'] }),
  })
}

export function useDeleteLabelRecord(id: string) {
  const qc = useQueryClient()
  return useMutation<void, Error, void>({
    mutationFn: () => apiClient.delete<void>(`/api/v1/label-records/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['label-records'] }),
  })
}
