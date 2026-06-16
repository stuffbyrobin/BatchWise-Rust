import { useQuery } from '@tanstack/react-query'
import { apiClient } from '../../api/client'
import type { components } from '../../api/generated'

type DashboardStats = components['schemas']['DashboardStats']

export function useDashboardStats() {
  return useQuery<DashboardStats>({
    queryKey: ['dashboard', 'stats'],
    queryFn: () => apiClient.get<DashboardStats>('/api/v1/dashboard/stats'),
  })
}
