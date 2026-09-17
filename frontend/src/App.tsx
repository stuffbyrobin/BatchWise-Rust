import { RouterProvider } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { AuthProvider } from './auth/AuthProvider'
import { ToastProvider } from './components/feedback/Toast'
import { ConfirmProvider } from './components/feedback/ConfirmDialog'
import { ErrorBoundary } from './components/feedback/ErrorBoundary'
import { APIError } from './api/error'
import router from './routes/router'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // Retry once on network errors and 5xx only: a 4xx will not change on retry.
      retry: (failureCount, error) =>
        failureCount < 1 && (!(error instanceof APIError) || error.status >= 500),
      staleTime: 30_000,
    },
  },
})

export default function App() {
  return (
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <AuthProvider>
          <ToastProvider>
            <ConfirmProvider>
              <RouterProvider router={router} />
            </ConfirmProvider>
          </ToastProvider>
        </AuthProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  )
}
