import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from './client'
import { qs } from './qs'
import type { Page } from './types'

export interface CrudConfig {
  /** Collection path, e.g. `/api/v1/suppliers`. */
  path: string
  /** Query-key prefix shared by the list and detail queries, e.g. `['suppliers']`. */
  queryKey: readonly unknown[]
  /** `patch` for partial updates (default), `put` for replace endpoints. */
  updateMethod?: 'patch' | 'put'
  /** Other query-key prefixes to refetch after any change (derived views). */
  alsoInvalidate?: readonly (readonly unknown[])[]
}

/**
 * Standard list / get / create / update / delete hooks for a REST collection.
 * Every mutation invalidates `queryKey` (covering list and detail queries) and
 * `alsoInvalidate`. The returned functions are hooks: call them from
 * components, not conditionally.
 */
export function createCrudHooks<
  T,
  TCreate = Partial<T>,
  TUpdate = Partial<T>,
  TParams extends object = Record<string, unknown>,
  TList = Page<T>,
>(config: CrudConfig) {
  const { path, queryKey, updateMethod = 'patch', alsoInvalidate = [] } = config

  function useInvalidate() {
    const qc = useQueryClient()
    return () => {
      for (const key of [queryKey, ...alsoInvalidate]) {
        void qc.invalidateQueries({ queryKey: key })
      }
    }
  }

  function useList(params: TParams = {} as TParams) {
    return useQuery<TList>({
      queryKey: [...queryKey, params],
      queryFn: ({ signal }) =>
        apiClient.get<TList>(`${path}${qs(params as Record<string, unknown>)}`, { signal }),
    })
  }

  function useOne(id: string) {
    return useQuery<T>({
      queryKey: [...queryKey, id],
      queryFn: ({ signal }) => apiClient.get<T>(`${path}/${id}`, { signal }),
      enabled: !!id,
    })
  }

  function useCreate() {
    const invalidate = useInvalidate()
    return useMutation<T, Error, TCreate>({
      mutationFn: (body) => apiClient.post<T>(path, body),
      onSuccess: invalidate,
    })
  }

  function useUpdate(id: string) {
    const invalidate = useInvalidate()
    return useMutation<T, Error, TUpdate>({
      mutationFn: (body) =>
        updateMethod === 'put'
          ? apiClient.put<T>(`${path}/${id}`, body)
          : apiClient.patch<T>(`${path}/${id}`, body),
      onSuccess: invalidate,
    })
  }

  function useDelete() {
    const invalidate = useInvalidate()
    return useMutation<void, Error, string>({
      mutationFn: (id) => apiClient.delete<void>(`${path}/${id}`),
      onSuccess: invalidate,
    })
  }

  return { useList, useOne, useCreate, useUpdate, useDelete }
}
