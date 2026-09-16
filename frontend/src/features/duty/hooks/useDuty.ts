import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'
import { qs } from '../../../api/qs'

type DutyReturn = components['schemas']['DutyReturn']
type DutyReturnList = components['schemas']['DutyReturnList']
type DutyCompileRequest = components['schemas']['DutyCompileRequest']
type DutyPatchRequest = components['schemas']['DutyPatchRequest']

export function useDutyReturns(params: { status?: string; page?: number; page_size?: number } = {}) {
  return useQuery<DutyReturnList>({
    queryKey: ['duty-returns', params],
    queryFn: ({ signal }) => apiClient.get<DutyReturnList>(
        `/api/v1/duty-returns${qs(params as Record<string, unknown>)}`, { signal },
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
