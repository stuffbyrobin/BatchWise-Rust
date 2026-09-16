import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'
import { qs } from '../../../api/qs'
import type { Page } from '../../../api/types'

type YeastKinetics = components['schemas']['YeastKinetics']

interface YKListParams {
  yeast_id?: string
  page?: number
  page_size?: number
  sort?: string
}

export function useYeastKineticsList(params: YKListParams = {}) {
  return useQuery<Page<YeastKinetics>>({
    queryKey: ['yeast-kinetics', params],
    queryFn: ({ signal }) => apiClient.get<Page<YeastKinetics>>(
        `/api/v1/yeast-kinetics${qs(params as Record<string, unknown>)}`, { signal },
      ),
  })
}

export function useYeastKinetics(id: string) {
  return useQuery<YeastKinetics>({
    queryKey: ['yeast-kinetics', id],
    queryFn: ({ signal }) => apiClient.get<YeastKinetics>(`/api/v1/yeast-kinetics/${id}`, { signal }),
    enabled: !!id,
  })
}

export function useCreateYeastKinetics() {
  const qc = useQueryClient()
  return useMutation<YeastKinetics, Error, Partial<YeastKinetics>>({
    mutationFn: (body) => apiClient.post<YeastKinetics>('/api/v1/yeast-kinetics', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['yeast-kinetics'] }),
  })
}

export function useUpdateYeastKinetics(id: string) {
  const qc = useQueryClient()
  return useMutation<YeastKinetics, Error, Partial<YeastKinetics>>({
    mutationFn: (body) => apiClient.patch<YeastKinetics>(`/api/v1/yeast-kinetics/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['yeast-kinetics'] }),
  })
}

export function useDeleteYeastKinetics() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/yeast-kinetics/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['yeast-kinetics'] }),
  })
}
