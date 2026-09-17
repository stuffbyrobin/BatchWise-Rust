import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'
import { qs } from '../../../api/qs'

type BrandAsset = components['schemas']['BrandAsset']
type BrandProfile = components['schemas']['BrandProfile']
type BrandProfileList = components['schemas']['BrandProfileList']
type CreateBrandProfileRequest = components['schemas']['CreateBrandProfileRequest']
type PatchBrandProfileRequest = components['schemas']['PatchBrandProfileRequest']
type LabelDesign = components['schemas']['LabelDesign']
type LabelDesignList = components['schemas']['LabelDesignList']
type CreateLabelDesignRequest = components['schemas']['CreateLabelDesignRequest']
type PatchLabelDesignRequest = components['schemas']['PatchLabelDesignRequest']
type RenderModel = components['schemas']['RenderModel']

// ——— brand profiles ————————————————————————————————————————————————————————

export function useBrandProfiles() {
  return useQuery<BrandProfileList>({
    queryKey: ['brand-profiles'],
    queryFn: ({ signal }) => apiClient.get<BrandProfileList>('/api/v1/brand-profiles', { signal }),
  })
}

export function useCreateBrandProfile() {
  const qc = useQueryClient()
  return useMutation<BrandProfile, Error, CreateBrandProfileRequest>({
    mutationFn: (body) => apiClient.post<BrandProfile>('/api/v1/brand-profiles', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['brand-profiles'] }),
  })
}

export function usePatchBrandProfile(id: string) {
  const qc = useQueryClient()
  return useMutation<BrandProfile, Error, PatchBrandProfileRequest>({
    mutationFn: (body) => apiClient.patch<BrandProfile>(`/api/v1/brand-profiles/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['brand-profiles'] }),
  })
}

export function useDeleteBrandProfile() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/brand-profiles/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['brand-profiles'] }),
  })
}

// ——— brand assets (logos) ——————————————————————————————————————————————————

export function useUploadAsset() {
  return useMutation<BrandAsset, Error, File>({
    mutationFn: (file) => {
      const form = new FormData()
      form.append('file', file)
      return apiClient.postForm<BrandAsset>('/api/v1/brand-assets', form)
    },
  })
}

// ——— designs ——————————————————————————————————————————————————————————————

export function useLabelDesigns(
  params: { kind?: string; batch_id?: string; recipe_id?: string; page?: number; page_size?: number } = {},
) {
  return useQuery<LabelDesignList>({
    queryKey: ['label-designs', params],
    queryFn: ({ signal }) => apiClient.get<LabelDesignList>(`/api/v1/label-designs${qs(params as Record<string, unknown>)}`, { signal }),
  })
}

export function useLabelDesign(id: string | undefined) {
  return useQuery<LabelDesign>({
    queryKey: ['label-design', id],
    queryFn: ({ signal }) => apiClient.get<LabelDesign>(`/api/v1/label-designs/${id}`, { signal }),
    enabled: !!id,
    // Hydrates an editor; a focus refetch would overwrite unsaved edits.
    refetchOnWindowFocus: false,
  })
}

export function useCreateLabelDesign() {
  const qc = useQueryClient()
  return useMutation<LabelDesign, Error, CreateLabelDesignRequest>({
    mutationFn: (body) => apiClient.post<LabelDesign>('/api/v1/label-designs', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['label-designs'] }),
  })
}

export function usePatchLabelDesign(id: string) {
  const qc = useQueryClient()
  return useMutation<LabelDesign, Error, PatchLabelDesignRequest>({
    mutationFn: (body) => apiClient.patch<LabelDesign>(`/api/v1/label-designs/${id}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['label-designs'] })
      qc.invalidateQueries({ queryKey: ['label-design', id] })
      qc.invalidateQueries({ queryKey: ['render-model', id] })
    },
  })
}

export function useDeleteLabelDesign() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/label-designs/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['label-designs'] }),
  })
}

// ——— render ————————————————————————————————————————————————————————————————

export function useRenderModel(id: string | undefined) {
  return useQuery<RenderModel>({
    queryKey: ['render-model', id],
    queryFn: ({ signal }) => apiClient.get<RenderModel>(`/api/v1/label-designs/${id}/render`, { signal }),
    enabled: !!id,
    retry: false,
  })
}

// fetchRenderPDF returns an object URL for the rendered PDF (caller must revoke it).
export async function fetchRenderPDF(id: string): Promise<string> {
  const blob = await apiClient.getBlob(`/api/v1/label-designs/${id}/render.pdf`)
  return URL.createObjectURL(blob)
}
