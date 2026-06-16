import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type Tenant = components['schemas']['Tenant']
type UpdateTenantRequest = components['schemas']['UpdateTenantRequest']

export function useTenant() {
  return useQuery<Tenant>({
    queryKey: ['tenant', 'current'],
    queryFn: () => apiClient.get<Tenant>('/api/v1/tenants/current'),
  })
}

export function useUpdateTenant() {
  const qc = useQueryClient()
  return useMutation<Tenant, Error, UpdateTenantRequest>({
    mutationFn: (body) => apiClient.patch<Tenant>('/api/v1/tenants/current', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['tenant', 'current'] }),
  })
}
