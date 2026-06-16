import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type WaterProfile = components['schemas']['WaterProfile']
type WaterAdjustment = components['schemas']['WaterAdjustment']
type WaterResult = components['schemas']['WaterResult']

interface Page<T> {
  items: T[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

// ── Water profiles ────────────────────────────────────────────────────────────

export function useWaterProfiles(params: { page?: number; page_size?: number; sort?: string } = {}) {
  return useQuery<Page<WaterProfile>>({
    queryKey: ['water-profiles', params],
    queryFn: () =>
      apiClient.get<Page<WaterProfile>>(
        `/api/v1/water-profiles${qs(params as Record<string, unknown>)}`,
      ),
  })
}

export function useCreateWaterProfile() {
  const qc = useQueryClient()
  return useMutation<WaterProfile, Error, Partial<WaterProfile>>({
    mutationFn: (body) => apiClient.post<WaterProfile>('/api/v1/water-profiles', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['water-profiles'] }),
  })
}

export function useUpdateWaterProfile(id: string) {
  const qc = useQueryClient()
  return useMutation<WaterProfile, Error, Partial<WaterProfile>>({
    mutationFn: (body) => apiClient.patch<WaterProfile>(`/api/v1/water-profiles/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['water-profiles'] }),
  })
}

export function useDeleteWaterProfile() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/water-profiles/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['water-profiles'] }),
  })
}

// ── Water adjustments ─────────────────────────────────────────────────────────

export function useWaterAdjustments(
  params: { page?: number; page_size?: number; sort?: string; batch_id?: string; recipe_id?: string } = {},
) {
  return useQuery<Page<WaterAdjustment>>({
    queryKey: ['water-adjustments', params],
    queryFn: () =>
      apiClient.get<Page<WaterAdjustment>>(
        `/api/v1/water-adjustments${qs(params as Record<string, unknown>)}`,
      ),
  })
}

export function useCreateWaterAdjustment() {
  const qc = useQueryClient()
  return useMutation<WaterAdjustment, Error, Record<string, unknown>>({
    mutationFn: (body) => apiClient.post<WaterAdjustment>('/api/v1/water-adjustments', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['water-adjustments'] }),
  })
}

export function useUpdateWaterAdjustment(id: string) {
  const qc = useQueryClient()
  return useMutation<WaterAdjustment, Error, Record<string, unknown>>({
    mutationFn: (body) => apiClient.patch<WaterAdjustment>(`/api/v1/water-adjustments/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['water-adjustments'] }),
  })
}

export function useDeleteWaterAdjustment() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/water-adjustments/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['water-adjustments'] }),
  })
}

// ── Stateless calculate ───────────────────────────────────────────────────────

export function useCalculateWater() {
  return useMutation<WaterResult, Error, Record<string, unknown>>({
    mutationFn: (body) => apiClient.post<WaterResult>('/api/v1/water-adjustments/calculate', body),
  })
}
