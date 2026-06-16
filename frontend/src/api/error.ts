export class APIError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
    public readonly requestId: string,
    public readonly details?: Record<string, unknown>,
  ) {
    super(message);
    this.name = 'APIError';
  }
}
export function parseAPIError(status: number, body: unknown): APIError {
  if (typeof body === 'object' && body !== null && 'code' in body) {
    const b = body as Record<string, unknown>;
    return new APIError(
      status,
      String(b['code'] ?? 'internal_error'),
      String(b['message'] ?? 'An unexpected error occurred.'),
      String(b['request_id'] ?? ''),
      b['details'] as Record<string, unknown> | undefined,
    );
  }
  return new APIError(status, 'internal_error', 'An unexpected error occurred.', '');
}
