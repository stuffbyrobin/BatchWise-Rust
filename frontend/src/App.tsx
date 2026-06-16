import { RouterProvider } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { AuthProvider } from './auth/AuthProvider'
import { ToastProvider } from './components/feedback/Toast'
import { ErrorBoundary } from './components/feedback/ErrorBoundary'
import router from './routes/router'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
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
            <RouterProvider router={router} />
          </ToastProvider>
        </AuthProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  )
}
