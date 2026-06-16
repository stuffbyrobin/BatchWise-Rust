import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type ContainerAsset = components['schemas']['ContainerAsset']
type ContainerAssetPage = components['schemas']['ContainerAssetPage']
type CreateContainerAssetRequest = components['schemas']['CreateContainerAssetRequest']
type PatchContainerAssetRequest = components['schemas']['PatchContainerAssetRequest']
type FillRequest = components['schemas']['FillRequest']
type DeliverRequest = components['schemas']['DeliverRequest']
type ReturnRequest = components['schemas']['ReturnRequest']
type SetStatusRequest = components['schemas']['SetStatusRequest']
type ContainerLogPage = components['schemas']['ContainerLogPage']
type QRResult = components['schemas']['QRResult']

function toQueryString(params: Record<string, unknown>): string {
  const q = Object.entries(params).filter(([, v]) => v !== undefined && v !== null && v !== '').map(([k, v]) => k + '=' + encodeURIComponent(String(v))).join('&')
  return q ? '?' + q : ''
}

export function useContainerAssetsList(params: { container_type?: string; page?: number; page_size?: number } = {}) {
  return useQuery<ContainerAssetPage>({
    queryKey: ['container-assets', params],
    queryFn: () =>
      apiClient.get<ContainerAssetPage>(`/api/v1/container-assets${toQueryString(params as Record<string, unknown>)}`),
  })
}

export function useContainerAsset(id: string) {
  return useQuery<ContainerAsset>({
    queryKey: ['container-assets', id],
    queryFn: () => apiClient.get<ContainerAsset>(`/api/v1/container-assets/${id}`),
    enabled: !!id,
  })
}

export function useCreateContainerAsset() {
  const qc = useQueryClient()
  return useMutation<ContainerAsset, Error, CreateContainerAssetRequest>({
    mutationFn: (body) => apiClient.post<ContainerAsset>('/api/v1/container-assets', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['container-assets'] }),
  })
}

export function usePatchContainerAsset(id: string) {
  const qc = useQueryClient()
  return useMutation<ContainerAsset, Error, PatchContainerAssetRequest>({
    mutationFn: (body) => apiClient.patch<ContainerAsset>(`/api/v1/container-assets/${id}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['container-assets', id] })
      qc.invalidateQueries({ queryKey: ['container-assets'] })
    },
  })
}

export function useDeleteContainerAsset() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/container-assets/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['container-assets'] }),
  })
}

export function useFillContainer(id: string) {
  const qc = useQueryClient()
  return useMutation<ContainerAsset, Error, FillRequest>({
    mutationFn: (body) => apiClient.post<ContainerAsset>(`/api/v1/container-assets/${id}/fill`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['container-assets', id] }),
  })
}

export function useDeliverContainer(id: string) {
  const qc = useQueryClient()
  return useMutation<ContainerAsset, Error, DeliverRequest>({
    mutationFn: (body) => apiClient.post<ContainerAsset>(`/api/v1/container-assets/${id}/deliver`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['container-assets', id] }),
  })
}

export function useReturnContainer(id: string) {
  const qc = useQueryClient()
  return useMutation<ContainerAsset, Error, ReturnRequest>({
    mutationFn: (body) => apiClient.post<ContainerAsset>(`/api/v1/container-assets/${id}/return`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['container-assets', id] }),
  })
}

export function useSetContainerStatus(id: string) {
  const qc = useQueryClient()
  return useMutation<ContainerAsset, Error, SetStatusRequest>({
    mutationFn: (body) => apiClient.post<ContainerAsset>(`/api/v1/container-assets/${id}/status`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['container-assets', id] }),
  })
}

export function useContainerLogs(containerId: string, params: { page?: number; page_size?: number } = {}) {
  return useQuery<ContainerLogPage>({
    queryKey: ['container-logs', containerId, params],
    queryFn: () =>
      apiClient.get<ContainerLogPage>(`/api/v1/container-logs?container_id=${containerId}${toQueryString(params as Record<string, unknown>)}`),
  })
}

export function useContainerQR(containerId: string, variant: string) {
  return useQuery<QRResult>({
    queryKey: ['qr', containerId, variant],
    queryFn: () => apiClient.get<QRResult>(`/api/v1/qr-codes/${containerId}/${variant}`),
    enabled: !!containerId,
  })
}

export const CONTAINER_TYPES = ['keg','cask','firkin','bottle_case','ibc','tank','other'] as const
export type ContainerType = typeof CONTAINER_TYPES[number]

export const CONTAINER_STATUSES = ['empty','filled','delivered','returned','lost','retired'] as const
export type ContainerStatus = typeof CONTAINER_STATUSES[number]
