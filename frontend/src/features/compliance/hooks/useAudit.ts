import { useQuery } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type AuditEvent = components['schemas']['AuditEvent']
type AuditEventList = components['schemas']['AuditEventList']

export interface AuditParams {
  entity_type?: string
  entity_id?: string
  event_type?: string
  from?: string
  to?: string
  page?: number
  page_size?: number
}

function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

export function useAuditEvents(params: AuditParams = {}) {
  return useQuery<AuditEventList>({
    queryKey: ['compliance-audit', params],
    queryFn: () =>
      apiClient.get<AuditEventList>(`/api/v1/compliance-audit${qs(params as Record<string, unknown>)}`),
  })
}

export function useAuditEvent(id: string) {
  return useQuery<AuditEvent>({
    queryKey: ['compliance-audit', id],
    queryFn: () => apiClient.get<AuditEvent>(`/api/v1/compliance-audit/${id}`),
    enabled: !!id,
  })
}
