/** One page of a list endpoint (every `/api/v1` list returns this envelope). */
export interface Page<T> {
  items: T[]
  total: number
  page: number
  page_size: number
  total_pages: number
}
