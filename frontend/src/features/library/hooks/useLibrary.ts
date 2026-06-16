import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type BeerStyle = components['schemas']['Style']
type EquipmentProfile = components['schemas']['EquipmentProfile']
type MashProfile = components['schemas']['MashProfile']
type Yeast = components['schemas']['Yeast']
type Fermentable = components['schemas']['LibraryFermentable']

function toQueryString(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

interface Page<T> {
  items: T[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

type ListParams = { page?: number; page_size?: number; sort?: string }

// ── Styles ────────────────────────────────────────────────────────────────────

export function useStyles(params: ListParams = {}) {
  return useQuery<Page<BeerStyle>>({
    queryKey: ['library', 'styles', params],
    queryFn: () =>
      apiClient.get<Page<BeerStyle>>(`/api/v1/library/styles${toQueryString(params as Record<string, unknown>)}`),
  })
}

export function useStyle(id: string) {
  return useQuery<BeerStyle>({
    queryKey: ['library', 'styles', id],
    queryFn: () => apiClient.get<BeerStyle>(`/api/v1/library/styles/${id}`),
    enabled: !!id,
  })
}

export function useCreateStyle() {
  const qc = useQueryClient()
  return useMutation<BeerStyle, Error, Partial<BeerStyle>>({
    mutationFn: (body) => apiClient.post<BeerStyle>('/api/v1/library/styles', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'styles'] }),
  })
}

export function useUpdateStyle(id: string) {
  const qc = useQueryClient()
  return useMutation<BeerStyle, Error, Partial<BeerStyle>>({
    mutationFn: (body) => apiClient.put<BeerStyle>(`/api/v1/library/styles/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'styles'] }),
  })
}

export function useDeleteStyle() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/library/styles/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'styles'] }),
  })
}

// ── Equipment Profiles ────────────────────────────────────────────────────────

export function useEquipmentProfiles(params: ListParams = {}) {
  return useQuery<Page<EquipmentProfile>>({
    queryKey: ['library', 'equipment-profiles', params],
    queryFn: () =>
      apiClient.get<Page<EquipmentProfile>>(
        `/api/v1/library/equipment-profiles${toQueryString(params as Record<string, unknown>)}`,
      ),
  })
}

export function useEquipmentProfile(id: string) {
  return useQuery<EquipmentProfile>({
    queryKey: ['library', 'equipment-profiles', id],
    queryFn: () => apiClient.get<EquipmentProfile>(`/api/v1/library/equipment-profiles/${id}`),
    enabled: !!id,
  })
}

export function useCreateEquipmentProfile() {
  const qc = useQueryClient()
  return useMutation<EquipmentProfile, Error, Partial<EquipmentProfile>>({
    mutationFn: (body) => apiClient.post<EquipmentProfile>('/api/v1/library/equipment-profiles', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'equipment-profiles'] }),
  })
}

export function useUpdateEquipmentProfile(id: string) {
  const qc = useQueryClient()
  return useMutation<EquipmentProfile, Error, Partial<EquipmentProfile>>({
    mutationFn: (body) => apiClient.put<EquipmentProfile>(`/api/v1/library/equipment-profiles/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'equipment-profiles'] }),
  })
}

export function useDeleteEquipmentProfile() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/library/equipment-profiles/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'equipment-profiles'] }),
  })
}

// ── Mash Profiles ─────────────────────────────────────────────────────────────

export function useMashProfiles(params: ListParams = {}) {
  return useQuery<Page<MashProfile>>({
    queryKey: ['library', 'mash-profiles', params],
    queryFn: () =>
      apiClient.get<Page<MashProfile>>(
        `/api/v1/library/mash-profiles${toQueryString(params as Record<string, unknown>)}`,
      ),
  })
}

export function useMashProfile(id: string) {
  return useQuery<MashProfile>({
    queryKey: ['library', 'mash-profiles', id],
    queryFn: () => apiClient.get<MashProfile>(`/api/v1/library/mash-profiles/${id}`),
    enabled: !!id,
  })
}

export function useCreateMashProfile() {
  const qc = useQueryClient()
  return useMutation<MashProfile, Error, Partial<MashProfile>>({
    mutationFn: (body) => apiClient.post<MashProfile>('/api/v1/library/mash-profiles', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'mash-profiles'] }),
  })
}

export function useUpdateMashProfile(id: string) {
  const qc = useQueryClient()
  return useMutation<MashProfile, Error, Partial<MashProfile>>({
    mutationFn: (body) => apiClient.put<MashProfile>(`/api/v1/library/mash-profiles/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'mash-profiles'] }),
  })
}

export function useDeleteMashProfile() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/library/mash-profiles/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'mash-profiles'] }),
  })
}

// ── Yeasts ────────────────────────────────────────────────────────────────────

export function useYeasts(params: ListParams = {}) {
  return useQuery<Page<Yeast>>({
    queryKey: ['library', 'yeasts', params],
    queryFn: () =>
      apiClient.get<Page<Yeast>>(`/api/v1/library/yeasts${toQueryString(params as Record<string, unknown>)}`),
  })
}

export function useYeast(id: string) {
  return useQuery<Yeast>({
    queryKey: ['library', 'yeasts', id],
    queryFn: () => apiClient.get<Yeast>(`/api/v1/library/yeasts/${id}`),
    enabled: !!id,
  })
}

export function useCreateYeast() {
  const qc = useQueryClient()
  return useMutation<Yeast, Error, Partial<Yeast>>({
    mutationFn: (body) => apiClient.post<Yeast>('/api/v1/library/yeasts', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'yeasts'] }),
  })
}

export function useUpdateYeast(id: string) {
  const qc = useQueryClient()
  return useMutation<Yeast, Error, Partial<Yeast>>({
    mutationFn: (body) => apiClient.put<Yeast>(`/api/v1/library/yeasts/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'yeasts'] }),
  })
}

export function useDeleteYeast() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/library/yeasts/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'yeasts'] }),
  })
}

// ── Fermentables ──────────────────────────────────────────────────────────────

export function useFermentables(params: ListParams & { name?: string; supplier?: string; type?: string } = {}) {
  return useQuery<Page<Fermentable>>({
    queryKey: ['library', 'fermentables', params],
    queryFn: () =>
      apiClient.get<Page<Fermentable>>(
        `/api/v1/library/fermentables${toQueryString(params as Record<string, unknown>)}`,
      ),
  })
}

export function useCreateFermentable() {
  const qc = useQueryClient()
  return useMutation<Fermentable, Error, Partial<Fermentable>>({
    mutationFn: (body) => apiClient.post<Fermentable>('/api/v1/library/fermentables', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'fermentables'] }),
  })
}

export function useUpdateFermentable(id: string) {
  const qc = useQueryClient()
  return useMutation<Fermentable, Error, Partial<Fermentable>>({
    mutationFn: (body) => apiClient.put<Fermentable>(`/api/v1/library/fermentables/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'fermentables'] }),
  })
}

export function useDeleteFermentable() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/library/fermentables/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['library', 'fermentables'] }),
  })
}
