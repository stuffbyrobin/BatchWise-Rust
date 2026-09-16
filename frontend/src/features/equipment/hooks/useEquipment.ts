import { createCrudHooks } from '../../../api/crud'
import type { components } from '../../../api/generated'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import { qs } from '../../../api/qs'

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

// ——— Equipment ———————————————————————————————————————————————————————————————

// Changing equipment also changes the maintenance-due view.
const equipment = createCrudHooks<Equipment, CreateEquipmentRequest, PatchEquipmentRequest, { status?: string; equipment_type?: string; sort?: string; page?: number; page_size?: number }, EquipmentList>({
  path: '/api/v1/equipment',
  queryKey: ['equipment'],
  alsoInvalidate: [['maintenance-due']],
})
export const useEquipmentList = equipment.useList
export const useCreateEquipment = equipment.useCreate
export const usePatchEquipment = equipment.useUpdate
export const useDeleteEquipment = equipment.useDelete

// ——— Maintenance schedules ———————————————————————————————————————————————————

export function useSchedules(equipmentID: string, params: { active?: boolean; sort?: string } = {}) {
  return useQuery<ScheduleList>({
    queryKey: ['schedules', equipmentID, params],
    queryFn: ({ signal }) => apiClient.get<ScheduleList>(`/api/v1/equipment/${equipmentID}/schedules${qs(params as Record<string, unknown>)}`, { signal }),
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
    queryFn: ({ signal }) => apiClient.get<EventList>(`/api/v1/equipment/${equipmentID}/events${qs(params as Record<string, unknown>)}`, { signal }),
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
    queryFn: ({ signal }) => apiClient.get<MaintenanceDueList>(`/api/v1/maintenance-due${qs(params as Record<string, unknown>)}`, { signal }),
  })
}
