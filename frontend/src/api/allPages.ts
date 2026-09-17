import { useQuery } from '@tanstack/react-query'
import { apiClient } from './client'
import { qs } from './qs'
import type { Page } from './types'

/** The API's maximum page size. */
export const MAX_PAGE_SIZE = 100

/**
 * Fetches every page of a list endpoint and returns all items. Stops at
 * `total_pages`, on an empty page, or after `maxPages` as a safety bound.
 */
export async function fetchAllPages<T>(
  path: string,
  params: Record<string, unknown> = {},
  signal?: AbortSignal,
  maxPages = 50,
): Promise<T[]> {
  const items: T[] = []
  for (let page = 1; page <= maxPages; page++) {
    const res = await apiClient.get<Page<T>>(
      `${path}${qs({ ...params, page, page_size: MAX_PAGE_SIZE })}`,
      { signal },
    )
    items.push(...res.items)
    if (res.items.length === 0 || page >= res.total_pages) break
  }
  return items
}

/**
 * Every item of a list endpoint, for dropdowns and whole-period views that must
 * not be truncated to one page. `queryKey` should start with the resource's
 * usual key prefix so existing mutations invalidate it.
 */
export function useAllPages<T>(
  queryKey: readonly unknown[],
  path: string,
  params: Record<string, unknown> = {},
) {
  return useQuery<T[]>({
    queryKey: [...queryKey, 'all', params],
    queryFn: ({ signal }) => fetchAllPages<T>(path, params, signal),
  })
}
