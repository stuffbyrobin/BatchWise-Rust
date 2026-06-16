import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type YeastBankEntry = components['schemas']['YeastBankEntry']
type YeastBankList = components['schemas']['YeastBankList']
type CreateYeastBankRequest = components['schemas']['CreateYeastBankRequest']
type PatchYeastBankRequest = components['schemas']['PatchYeastBankRequest']
type HarvestRequest = components['schemas']['HarvestRequest']
type Propagation = components['schemas']['Propagation']
type PropagationList = components['schemas']['PropagationList']
type CreatePropagationRequest = components['schemas']['CreatePropagationRequest']
type PatchPropagationRequest = components['schemas']['PatchPropagationRequest']

function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

// ——— Yeast Bank Entries ——————————————————————————————————————————————————————

export function useYeastBank(params: { status?: string; sort?: string; page?: number; page_size?: number } = {}) {
  return useQuery<YeastBankList>({
    queryKey: ['yeast-bank', params],
    queryFn: () => apiClient.get<YeastBankList>(`/api/v1/yeast-bank${qs(params as Record<string, unknown>)}`),
  })
}

export function useCreateYeastBankEntry() {
  const qc = useQueryClient()
  return useMutation<YeastBankEntry, Error, CreateYeastBankRequest>({
    mutationFn: (body) => apiClient.post<YeastBankEntry>('/api/v1/yeast-bank', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['yeast-bank'] }),
  })
}

export function usePatchYeastBankEntry(id: string) {
  const qc = useQueryClient()
  return useMutation<YeastBankEntry, Error, PatchYeastBankRequest>({
    mutationFn: (body) => apiClient.patch<YeastBankEntry>(`/api/v1/yeast-bank/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['yeast-bank'] }),
  })
}

export function useDeleteYeastBankEntry() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/yeast-bank/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['yeast-bank'] }),
  })
}

export function useHarvestYeast(id: string) {
  const qc = useQueryClient()
  return useMutation<YeastBankEntry, Error, HarvestRequest>({
    mutationFn: (body) => apiClient.post<YeastBankEntry>(`/api/v1/yeast-bank/${id}/harvest`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['yeast-bank'] }),
  })
}

// ——— Propagations ————————————————————————————————————————————————————————————

export function usePropagations(bankID: string, params: { page?: number; page_size?: number } = {}) {
  return useQuery<PropagationList>({
    queryKey: ['propagations', bankID, params],
    queryFn: () => apiClient.get<PropagationList>(`/api/v1/yeast-bank/${bankID}/propagations${qs(params as Record<string, unknown>)}`),
    enabled: !!bankID,
  })
}

export function useCreatePropagation(bankID: string) {
  const qc = useQueryClient()
  return useMutation<Propagation, Error, CreatePropagationRequest>({
    mutationFn: (body) => apiClient.post<Propagation>(`/api/v1/yeast-bank/${bankID}/propagations`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['propagations', bankID] }),
  })
}

export function usePatchPropagation(bankID: string, propID: string) {
  const qc = useQueryClient()
  return useMutation<Propagation, Error, PatchPropagationRequest>({
    mutationFn: (body) => apiClient.patch<Propagation>(`/api/v1/yeast-bank/${bankID}/propagations/${propID}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['propagations', bankID] }),
  })
}

export function useDeletePropagation(bankID: string) {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (propID) => apiClient.delete<void>(`/api/v1/yeast-bank/${bankID}/propagations/${propID}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['propagations', bankID] }),
  })
}
