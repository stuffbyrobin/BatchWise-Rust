import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'
import { qs } from '../../../api/qs'

type CalendarEvent = components['schemas']['CalendarEvent']
type CreateCalendarEventRequest = components['schemas']['CreateCalendarEventRequest']
type UpdateCalendarEventRequest = components['schemas']['UpdateCalendarEventRequest']
type PaginatedCalendarEvents = components['schemas']['PaginatedCalendarEvents']

interface CalendarListParams {
  batch_id?: string
  event_type?: string
  status?: string
  from?: string
  to?: string
  page?: number
  page_size?: number
}

export function useCalendarEvents(params: CalendarListParams = {}) {
  return useQuery<PaginatedCalendarEvents>({
    queryKey: ['calendar-events', params],
    queryFn: ({ signal }) => apiClient.get<PaginatedCalendarEvents>(
        `/api/v1/calendar-events${qs(params as Record<string, unknown>)}`, { signal },
      ),
  })
}

export function useCalendarEvent(id: string) {
  return useQuery<CalendarEvent>({
    queryKey: ['calendar-events', id],
    queryFn: ({ signal }) => apiClient.get<CalendarEvent>(`/api/v1/calendar-events/${id}`, { signal }),
    enabled: !!id,
  })
}

export function useCreateCalendarEvent() {
  const qc = useQueryClient()
  return useMutation<CalendarEvent, Error, CreateCalendarEventRequest>({
    mutationFn: (body) => apiClient.post<CalendarEvent>('/api/v1/calendar-events', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['calendar-events'] }),
  })
}

export function useUpdateCalendarEvent(id: string) {
  const qc = useQueryClient()
  return useMutation<CalendarEvent, Error, UpdateCalendarEventRequest>({
    mutationFn: (body) => apiClient.patch<CalendarEvent>(`/api/v1/calendar-events/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['calendar-events'] }),
  })
}

export function useDeleteCalendarEvent(id: string) {
  const qc = useQueryClient()
  return useMutation<void, Error, void>({
    mutationFn: () => apiClient.delete<void>(`/api/v1/calendar-events/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['calendar-events'] }),
  })
}

export function useCompleteCalendarEvent(id: string) {
  const qc = useQueryClient()
  return useMutation<CalendarEvent, Error, void>({
    mutationFn: () =>
      apiClient.patch<CalendarEvent>(`/api/v1/calendar-events/${id}`, { status: 'completed' }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['calendar-events'] }),
  })
}
