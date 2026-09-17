import { createCrudHooks } from '../../../api/crud'
import type { components } from '../../../api/generated'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'

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

// ——— Suppliers ———————————————————————————————————————————————————————————————

const suppliers = createCrudHooks<Supplier, CreateSupplierRequest, PatchSupplierRequest, { search?: string; sort?: string; page?: number; page_size?: number }, SupplierList>({
  path: '/api/v1/suppliers',
  queryKey: ['suppliers'],
})
export const useSuppliers = suppliers.useList
export const useCreateSupplier = suppliers.useCreate
export const usePatchSupplier = suppliers.useUpdate
export const useDeleteSupplier = suppliers.useDelete

// ——— Purchase Orders ——————————————————————————————————————————————————————————

const purchaseOrders = createCrudHooks<PurchaseOrder, CreatePORequest, PatchPORequest, { supplier_id?: string; status?: string; sort?: string; page?: number; page_size?: number }, PurchaseOrderList>({
  path: '/api/v1/purchase-orders',
  queryKey: ['purchase-orders'],
})
export const usePurchaseOrders = purchaseOrders.useList
export const usePurchaseOrder = purchaseOrders.useOne
export const useCreatePO = purchaseOrders.useCreate
export const usePatchPO = purchaseOrders.useUpdate
export const useDeletePO = purchaseOrders.useDelete

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
