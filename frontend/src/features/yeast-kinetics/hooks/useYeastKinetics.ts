import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type YeastKinetics = components['schemas']['YeastKinetics']

interface Page<T> {
  items: T[]
  total: number
}

function toQueryString(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

interface YKListParams {
  yeast_id?: string
  page?: number
  page_size?: number
}

export function useYeastKineticsList(params: YKListParams = {}) {
  return useQuery<Page<YeastKinetics>>({
    queryKey: ['yeast-kinetics', params],
    queryFn: () =>
      apiClient.get<Page<YeastKinetics>>(
        `/api/v1/yeast-kinetics${toQueryString(params as Record<string, unknown>)}`,
      ),
  })
}

export function useYeastKinetics(id: string) {
  return useQuery<YeastKinetics>({
    queryKey: ['yeast-kinetics', id],
    queryFn: () => apiClient.get<YeastKinetics>(`/api/v1/yeast-kinetics/${id}`),
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
