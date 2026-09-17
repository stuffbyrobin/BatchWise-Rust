import { createCrudHooks } from '../../../api/crud'
import type { components } from '../../../api/generated'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import { qs } from '../../../api/qs'

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

const containerAssets = createCrudHooks<ContainerAsset, CreateContainerAssetRequest, PatchContainerAssetRequest, { container_type?: string; page?: number; page_size?: number }, ContainerAssetPage>({
  path: '/api/v1/container-assets',
  queryKey: ['container-assets'],
})
export const useContainerAssetsList = containerAssets.useList
export const useContainerAsset = containerAssets.useOne
export const useCreateContainerAsset = containerAssets.useCreate
export const usePatchContainerAsset = containerAssets.useUpdate
export const useDeleteContainerAsset = containerAssets.useDelete

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
    queryFn: ({ signal }) => apiClient.get<ContainerLogPage>(`/api/v1/container-logs?container_id=${containerId}${qs(params as Record<string, unknown>)}`, { signal }),
  })
}

export function useContainerQR(containerId: string, variant: string) {
  return useQuery<QRResult>({
    queryKey: ['qr', containerId, variant],
    queryFn: ({ signal }) => apiClient.get<QRResult>(`/api/v1/qr-codes/${containerId}/${variant}`, { signal }),
    enabled: !!containerId,
  })
}

export const CONTAINER_TYPES = ['keg','cask','firkin','bottle_case','ibc','tank','other'] as const
export type ContainerType = typeof CONTAINER_TYPES[number]

export const CONTAINER_STATUSES = ['empty','filled','delivered','returned','lost','retired'] as const
export type ContainerStatus = typeof CONTAINER_STATUSES[number]
