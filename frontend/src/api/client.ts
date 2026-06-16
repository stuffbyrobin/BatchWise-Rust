import { parseAPIError, APIError } from './error'

let _getToken: () => string | null = () => null
let _getRefresh: () => string | null = () => null
let _setTokens: (access: string, refresh: string) => void = () => {}
let _clear: () => void = () => {}

export function _initTokenStore(
  getToken: () => string | null,
  getRefresh: () => string | null,
  setTokens: (access: string, refresh: string) => void,
  clear: () => void,
): void {
  _getToken = getToken
  _getRefresh = getRefresh
  _setTokens = setTokens
  _clear = clear
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  init?: RequestInit,
  _isRetry = false,
): Promise<T> {
  const token = _getToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((init?.headers ?? {}) as Record<string, string>),
  }
  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }

  const res = await fetch(path, {
    ...init,
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })

  if (res.ok) {
    if (res.status === 204) return undefined as T
    return res.json() as Promise<T>
  }

  if (res.status === 401 && !_isRetry) {
    const refreshToken = _getRefresh()
    if (refreshToken) {
      const refreshRes = await fetch('/api/v1/auth/refresh', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refresh_token: refreshToken }),
      })
      if (refreshRes.ok) {
        const data = (await refreshRes.json()) as { access_token: string; refresh_token: string }
        _setTokens(data.access_token, data.refresh_token)
        return request<T>(method, path, body, init, true)
      }
    }
    _clear()
    window.location.href = '/login'
    throw new APIError(401, 'unauthorized', 'Session expired. Please log in again.', '')
  }

  let errBody: unknown
  try {
    errBody = await res.json()
  } catch {
    errBody = null
  }
  throw parseAPIError(res.status, errBody)
}

async function authFetch(method: string, path: string, body?: BodyInit): Promise<Response> {
  const token = _getToken()
  const headers: Record<string, string> = {}
  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }
  return fetch(path, { method, headers, body })
}

export const apiClient = {
  get: <T>(path: string, init?: RequestInit) => request<T>('GET', path, undefined, init),
  post: <T>(path: string, body?: unknown, init?: RequestInit) => request<T>('POST', path, body, init),
  put: <T>(path: string, body?: unknown, init?: RequestInit) => request<T>('PUT', path, body, init),
  patch: <T>(path: string, body?: unknown, init?: RequestInit) => request<T>('PATCH', path, body, init),
  delete: <T>(path: string, init?: RequestInit) => request<T>('DELETE', path, undefined, init),

  // postForm uploads multipart/form-data (browser sets the boundary Content-Type).
  postForm: async <T>(path: string, form: FormData): Promise<T> => {
    const res = await authFetch('POST', path, form)
    if (res.ok) {
      return (res.status === 204 ? undefined : await res.json()) as T
    }
    let errBody: unknown = null
    try {
      errBody = await res.json()
    } catch {
      errBody = null
    }
    throw parseAPIError(res.status, errBody)
  },

  // getBlob fetches a binary response (e.g. PDF / image) with auth.
  getBlob: async (path: string): Promise<Blob> => {
    const res = await authFetch('GET', path)
    if (res.ok) {
      return res.blob()
    }
    let errBody: unknown = null
    try {
      errBody = await res.json()
    } catch {
      errBody = null
    }
    throw parseAPIError(res.status, errBody)
  },
}
