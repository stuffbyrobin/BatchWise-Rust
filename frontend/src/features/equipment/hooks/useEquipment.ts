import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type Equipment = components['schemas']['Equipment']
type EquipmentList = components['schemas']['EquipmentList']
type CreateEquipmentRequest = components['schemas']['CreateEquipmentRequest']
type PatchEquipmentRequest = components['schemas']['PatchEquipmentRequest']
type MaintenanceSchedule = components['schemas']['MaintenanceSchedule']
type ScheduleList = components['schemas']['ScheduleList']
type CreateScheduleRequest = components['schemas']['CreateScheduleRequest']
type PatchScheduleRequest = components['schemas']['PatchScheduleRequest']
type MaintenanceEvent = components['schemas']['MaintenanceEvent']
type EventList = components['schemas']['EventList']
type CreateEventRequest = components['schemas']['CreateEventRequest']
type MaintenanceDueList = components['schemas']['MaintenanceDueList']

function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

// ——— Equipment ———————————————————————————————————————————————————————————————

export function useEquipmentList(params: { status?: string; equipment_type?: string; sort?: string; page?: number; page_size?: number } = {}) {
  return useQuery<EquipmentList>({
    queryKey: ['equipment', params],
    queryFn: () => apiClient.get<EquipmentList>(`/api/v1/equipment${qs(params as Record<string, unknown>)}`),
  })
}

export function useCreateEquipment() {
  const qc = useQueryClient()
  return useMutation<Equipment, Error, CreateEquipmentRequest>({
    mutationFn: (body) => apiClient.post<Equipment>('/api/v1/equipment', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['equipment'] }),
  })
}

export function usePatchEquipment(id: string) {
  const qc = useQueryClient()
  return useMutation<Equipment, Error, PatchEquipmentRequest>({
    mutationFn: (body) => apiClient.patch<Equipment>(`/api/v1/equipment/${id}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['equipment'] })
      qc.invalidateQueries({ queryKey: ['maintenance-due'] })
    },
  })
}

export function useDeleteEquipment() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/equipment/${id}`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['equipment'] })
      qc.invalidateQueries({ queryKey: ['maintenance-due'] })
    },
  })
}

// ——— Maintenance schedules ———————————————————————————————————————————————————

export function useSchedules(equipmentID: string, params: { active?: boolean; sort?: string } = {}) {
  return useQuery<ScheduleList>({
    queryKey: ['schedules', equipmentID, params],
    queryFn: () => apiClient.get<ScheduleList>(`/api/v1/equipment/${equipmentID}/schedules${qs(params as Record<string, unknown>)}`),
    enabled: !!equipmentID,
  })
}

export function useCreateSchedule(equipmentID: string) {
  const qc = useQueryClient()
  return useMutation<MaintenanceSchedule, Error, CreateScheduleRequest>({
    mutationFn: (body) => apiClient.post<MaintenanceSchedule>(`/api/v1/equipment/${equipmentID}/schedules`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['schedules', equipmentID] })
      qc.invalidateQueries({ queryKey: ['equipment'] })
      qc.invalidateQueries({ queryKey: ['maintenance-due'] })
    },
  })
}

export function usePatchSchedule(equipmentID: string, scheduleID: string) {
  const qc = useQueryClient()
  return useMutation<MaintenanceSchedule, Error, PatchScheduleRequest>({
    mutationFn: (body) => apiClient.patch<MaintenanceSchedule>(`/api/v1/equipment/${equipmentID}/schedules/${scheduleID}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['schedules', equipmentID] })
      qc.invalidateQueries({ queryKey: ['equipment'] })
      qc.invalidateQueries({ queryKey: ['maintenance-due'] })
    },
  })
}

export function useDeleteSchedule(equipmentID: string) {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (scheduleID) => apiClient.delete<void>(`/api/v1/equipment/${equipmentID}/schedules/${scheduleID}`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['schedules', equipmentID] })
      qc.invalidateQueries({ queryKey: ['equipment'] })
      qc.invalidateQueries({ queryKey: ['maintenance-due'] })
    },
  })
}

// ——— Maintenance events ——————————————————————————————————————————————————————

export function useEvents(equipmentID: string, params: { event_type?: string; sort?: string } = {}) {
  return useQuery<EventList>({
    queryKey: ['events', equipmentID, params],
    queryFn: () => apiClient.get<EventList>(`/api/v1/equipment/${equipmentID}/events${qs(params as Record<string, unknown>)}`),
    enabled: !!equipmentID,
  })
}

export function useCreateEvent(equipmentID: string) {
  const qc = useQueryClient()
  return useMutation<MaintenanceEvent, Error, CreateEventRequest>({
    mutationFn: (body) => apiClient.post<MaintenanceEvent>(`/api/v1/equipment/${equipmentID}/events`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['events', equipmentID] })
      qc.invalidateQueries({ queryKey: ['schedules', equipmentID] })
      qc.invalidateQueries({ queryKey: ['equipment'] })
      qc.invalidateQueries({ queryKey: ['maintenance-due'] })
    },
  })
}

export function useDeleteEvent(equipmentID: string) {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (eventID) => apiClient.delete<void>(`/api/v1/equipment/${equipmentID}/events/${eventID}`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['events', equipmentID] })
      qc.invalidateQueries({ queryKey: ['equipment'] })
    },
  })
}

// ——— Maintenance due feed ————————————————————————————————————————————————————

export function useMaintenanceDue(params: { window_days?: number; overdue_only?: boolean; page?: number; page_size?: number } = {}) {
  return useQuery<MaintenanceDueList>({
    queryKey: ['maintenance-due', params],
    queryFn: () => apiClient.get<MaintenanceDueList>(`/api/v1/maintenance-due${qs(params as Record<string, unknown>)}`),
  })
}
