import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type Supplier = components['schemas']['Supplier']
type SupplierList = components['schemas']['SupplierList']
type CreateSupplierRequest = components['schemas']['CreateSupplierRequest']
type PatchSupplierRequest = components['schemas']['PatchSupplierRequest']
type PurchaseOrder = components['schemas']['PurchaseOrder']
type PurchaseOrderList = components['schemas']['PurchaseOrderList']
type CreatePORequest = components['schemas']['CreatePORequest']
type PatchPORequest = components['schemas']['PatchPORequest']
type PurchaseOrderLine = components['schemas']['PurchaseOrderLine']
type CreateLineRequest = components['schemas']['CreateLineRequest']
type PatchLineRequest = components['schemas']['PatchLineRequest']
type ReceiveRequest = components['schemas']['ReceiveRequest']

function qs(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

// ——— Suppliers ———————————————————————————————————————————————————————————————

export function useSuppliers(params: { search?: string; sort?: string; page?: number; page_size?: number } = {}) {
  return useQuery<SupplierList>({
    queryKey: ['suppliers', params],
    queryFn: () => apiClient.get<SupplierList>(`/api/v1/suppliers${qs(params as Record<string, unknown>)}`),
  })
}

export function useCreateSupplier() {
  const qc = useQueryClient()
  return useMutation<Supplier, Error, CreateSupplierRequest>({
    mutationFn: (body) => apiClient.post<Supplier>('/api/v1/suppliers', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['suppliers'] }),
  })
}

export function usePatchSupplier(id: string) {
  const qc = useQueryClient()
  return useMutation<Supplier, Error, PatchSupplierRequest>({
    mutationFn: (body) => apiClient.patch<Supplier>(`/api/v1/suppliers/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['suppliers'] }),
  })
}

export function useDeleteSupplier() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/suppliers/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['suppliers'] }),
  })
}

// ——— Purchase Orders ——————————————————————————————————————————————————————————

export function usePurchaseOrders(params: { supplier_id?: string; status?: string; sort?: string; page?: number; page_size?: number } = {}) {
  return useQuery<PurchaseOrderList>({
    queryKey: ['purchase-orders', params],
    queryFn: () => apiClient.get<PurchaseOrderList>(`/api/v1/purchase-orders${qs(params as Record<string, unknown>)}`),
  })
}

export function usePurchaseOrder(id: string) {
  return useQuery<PurchaseOrder>({
    queryKey: ['purchase-orders', id],
    queryFn: () => apiClient.get<PurchaseOrder>(`/api/v1/purchase-orders/${id}`),
    enabled: !!id,
  })
}

export function useCreatePO() {
  const qc = useQueryClient()
  return useMutation<PurchaseOrder, Error, CreatePORequest>({
    mutationFn: (body) => apiClient.post<PurchaseOrder>('/api/v1/purchase-orders', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['purchase-orders'] }),
  })
}

export function usePatchPO(id: string) {
  const qc = useQueryClient()
  return useMutation<PurchaseOrder, Error, PatchPORequest>({
    mutationFn: (body) => apiClient.patch<PurchaseOrder>(`/api/v1/purchase-orders/${id}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['purchase-orders'] })
    },
  })
}

export function useDeletePO() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/purchase-orders/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['purchase-orders'] }),
  })
}

// ——— Lines ————————————————————————————————————————————————————————————————————

export function useAddLine(poID: string) {
  const qc = useQueryClient()
  return useMutation<PurchaseOrderLine, Error, CreateLineRequest>({
    mutationFn: (body) => apiClient.post<PurchaseOrderLine>(`/api/v1/purchase-orders/${poID}/lines`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['purchase-orders'] }),
  })
}

export function usePatchLine(poID: string, lineID: string) {
  const qc = useQueryClient()
  return useMutation<PurchaseOrderLine, Error, PatchLineRequest>({
    mutationFn: (body) => apiClient.patch<PurchaseOrderLine>(`/api/v1/purchase-orders/${poID}/lines/${lineID}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['purchase-orders'] }),
  })
}

export function useDeleteLine(poID: string) {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (lineID) => apiClient.delete<void>(`/api/v1/purchase-orders/${poID}/lines/${lineID}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['purchase-orders'] }),
  })
}

// ——— Receive —————————————————————————————————————————————————————————————————

export function useReceivePO(poID: string) {
  const qc = useQueryClient()
  return useMutation<PurchaseOrder, Error, ReceiveRequest>({
    mutationFn: (body) => apiClient.post<PurchaseOrder>(`/api/v1/purchase-orders/${poID}/receive`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['purchase-orders'] }),
  })
}
