import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type CostReport = components['schemas']['CostReport']
type CostReportPage = components['schemas']['CostReportPage']
type GenerateReportRequest = components['schemas']['GenerateReportRequest']

function toQueryString(params: Record<string, unknown>): string {
  const q = Object.entries(params).filter(([, v]) => v !== undefined && v !== null && v !== '').map(([k, v]) => k + '=' + encodeURIComponent(String(v))).join('&')
  return q ? '?' + q : ''
}

export function useCostReportsList(params: { page?: number; page_size?: number } = {}) {
  return useQuery<CostReportPage>({
    queryKey: ['cost-reports', params],
    queryFn: () =>
      apiClient.get<CostReportPage>(`/api/v1/cost-reports${toQueryString(params as Record<string, unknown>)}`),
  })
}

export function useCostReport(id: string) {
  return useQuery<CostReport>({
    queryKey: ['cost-reports', id],
    queryFn: () => apiClient.get<CostReport>(`/api/v1/cost-reports/${id}`),
    enabled: !!id,
  })
}

export function useGenerateCostReport() {
  const qc = useQueryClient()
  return useMutation<CostReport, Error, GenerateReportRequest>({
    mutationFn: (body) => apiClient.post<CostReport>('/api/v1/cost-reports/generate', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['cost-reports'] }),
  })
}

export function useDeleteCostReport() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/cost-reports/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['cost-reports'] }),
  })
}

export const REPORT_TYPES = ['batch','recipe','period','inventory'] as const
export type ReportType = typeof REPORT_TYPES[number]
