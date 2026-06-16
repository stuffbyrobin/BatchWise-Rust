import React from 'react'
import * as RadixToast from '@radix-ui/react-toast'

interface ToastItem {
  id: string
  title: string
  description?: string
  variant?: 'default' | 'destructive'
}

interface ToastContextValue {
  toast: (opts: { title: string; description?: string; variant?: 'default' | 'destructive' }) => void
}

const ToastContext = React.createContext<ToastContextValue | null>(null)

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = React.useState<ToastItem[]>([])

  const toast = React.useCallback(
    (opts: { title: string; description?: string; variant?: 'default' | 'destructive' }) => {
      const id = Math.random().toString(36).slice(2)
      setToasts((prev) => [...prev, { id, ...opts }])
    },
    [],
  )

  return (
    <ToastContext.Provider value={{ toast }}>
      <RadixToast.Provider swipeDirection="right">
        {children}
        {toasts.map((t) => (
          <RadixToast.Root
            key={t.id}
            onOpenChange={(open) => {
              if (!open) setToasts((prev) => prev.filter((x) => x.id !== t.id))
            }}
            className={`rounded p-4 shadow-lg border ${
              t.variant === 'destructive'
                ? 'bg-[var(--color-danger)] text-white border-red-700'
                : 'bg-[var(--color-surface)] text-[var(--color-fg)] border-[var(--color-border)]'
            }`}
          >
            <RadixToast.Title className="font-semibold text-sm">{t.title}</RadixToast.Title>
            {t.description && (
              <RadixToast.Description className="mt-1 text-sm opacity-90">
                {t.description}
              </RadixToast.Description>
            )}
          </RadixToast.Root>
        ))}
        <RadixToast.Viewport className="fixed bottom-4 right-4 flex flex-col gap-2 z-50 w-80" />
      </RadixToast.Provider>
    </ToastContext.Provider>
  )
}

export function useToast(): ToastContextValue {
  const ctx = React.useContext(ToastContext)
  if (!ctx) throw new Error('useToast must be used within ToastProvider')
  return ctx
}
