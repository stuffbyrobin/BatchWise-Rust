import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type DutyReturn = components['schemas']['DutyReturn']
type DutyReturnList = components['schemas']['DutyReturnList']
type DutyCompileRequest = components['schemas']['DutyCompileRequest']
type DutyPatchRequest = components['schemas']['DutyPatchRequest']

function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

export function useDutyReturns(params: { status?: string; page?: number; page_size?: number } = {}) {
  return useQuery<DutyReturnList>({
    queryKey: ['duty-returns', params],
    queryFn: () =>
      apiClient.get<DutyReturnList>(
        `/api/v1/duty-returns${qs(params as Record<string, unknown>)}`,
      ),
  })
}

export function useCompileDutyReturn() {
  const qc = useQueryClient()
  return useMutation<DutyReturn, Error, DutyCompileRequest>({
    mutationFn: (body) => apiClient.post<DutyReturn>('/api/v1/duty-returns/compile', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['duty-returns'] }),
  })
}

export function usePatchDutyReturn(id: string) {
  const qc = useQueryClient()
  return useMutation<DutyReturn, Error, DutyPatchRequest>({
    mutationFn: (body) => apiClient.patch<DutyReturn>(`/api/v1/duty-returns/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['duty-returns'] }),
  })
}
