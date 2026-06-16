import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type PackagingRun = components['schemas']['PackagingRun']
type PackagingRunList = components['schemas']['PackagingRunList']
type CreatePackagingRunRequest = components['schemas']['CreatePackagingRunRequest']
type PatchPackagingRunRequest = components['schemas']['PatchPackagingRunRequest']
type DistributionMovement = components['schemas']['DistributionMovement']
type DistributionMovementList = components['schemas']['DistributionMovementList']
type CreateDistributionMovementRequest = components['schemas']['CreateDistributionMovementRequest']

function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

export function usePackagingRuns(params: { batch_id?: string; format?: string; page?: number; page_size?: number } = {}) {
  return useQuery<PackagingRunList>({
    queryKey: ['packaging-runs', params],
    queryFn: () => apiClient.get<PackagingRunList>(`/api/v1/packaging-runs${qs(params as Record<string, unknown>)}`),
  })
}

export function usePackagingRun(id: string) {
  return useQuery<PackagingRun>({
    queryKey: ['packaging-runs', id],
    queryFn: () => apiClient.get<PackagingRun>(`/api/v1/packaging-runs/${id}`),
    enabled: !!id,
  })
}

export function useCreatePackagingRun() {
  const qc = useQueryClient()
  return useMutation<PackagingRun, Error, CreatePackagingRunRequest>({
    mutationFn: (body) => apiClient.post<PackagingRun>('/api/v1/packaging-runs', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['packaging-runs'] }),
  })
}

export function usePatchPackagingRun(id: string) {
  const qc = useQueryClient()
  return useMutation<PackagingRun, Error, PatchPackagingRunRequest>({
    mutationFn: (body) => apiClient.patch<PackagingRun>(`/api/v1/packaging-runs/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['packaging-runs'] }),
  })
}

export function useDeletePackagingRun() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/packaging-runs/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['packaging-runs'] }),
  })
}

export function useDistributionMovements(params: { packaging_run_id?: string; order_id?: string; movement_type?: string; page?: number; page_size?: number } = {}) {
  return useQuery<DistributionMovementList>({
    queryKey: ['distribution-movements', params],
    queryFn: () => apiClient.get<DistributionMovementList>(`/api/v1/distribution-movements${qs(params as Record<string, unknown>)}`),
  })
}

export function useCreateDistributionMovement() {
  const qc = useQueryClient()
  return useMutation<DistributionMovement, Error, CreateDistributionMovementRequest>({
    mutationFn: (body) => apiClient.post<DistributionMovement>('/api/v1/distribution-movements', body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['distribution-movements'] })
      qc.invalidateQueries({ queryKey: ['packaging-runs'] })
    },
  })
}

export function useDeleteDistributionMovement() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/distribution-movements/${id}`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['distribution-movements'] })
      qc.invalidateQueries({ queryKey: ['packaging-runs'] })
    },
  })
}
