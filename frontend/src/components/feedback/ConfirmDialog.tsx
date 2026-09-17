import { createContext, useCallback, useContext, useRef, useState, type ReactNode } from 'react'
import * as AlertDialog from '@radix-ui/react-alert-dialog'

export interface ConfirmOptions {
  title: string
  description?: ReactNode
  /** Label of the confirming button. Defaults to "Confirm". */
  confirmLabel?: string
  cancelLabel?: string
  /** Styles the confirming button as destructive, e.g. for deletes. */
  destructive?: boolean
}

interface ConfirmDialogProps extends ConfirmOptions {
  open: boolean
  /** Disables both buttons, and Escape, while the confirmed action runs. */
  busy?: boolean
  onConfirm: () => void
  onCancel: () => void
}

/**
 * Accessible confirmation dialog: focus is trapped, Cancel is focused first and
 * Escape cancels. It is controlled, and confirming does not close it, so a
 * caller can keep it open while the confirmed action runs.
 */
export function ConfirmDialog({
  open,
  title,
  description,
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
  destructive = false,
  busy = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  return (
    <AlertDialog.Root
      open={open}
      onOpenChange={(next) => {
        if (!next && !busy) onCancel()
      }}
    >
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="fixed inset-0 z-50 bg-black/40" />
        <AlertDialog.Content
          {...(description ? {} : { 'aria-describedby': undefined })}
          className="fixed left-1/2 top-1/2 z-50 w-[calc(100%-2rem)] max-w-sm -translate-x-1/2 -translate-y-1/2 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] p-6 shadow-lg"
        >
          <AlertDialog.Title className="text-lg font-semibold text-[var(--color-fg)] mb-2">{title}</AlertDialog.Title>
          {description && (
            <AlertDialog.Description className="text-sm text-[var(--color-muted)] mb-4">
              {description}
            </AlertDialog.Description>
          )}
          <div className="flex gap-3 justify-end">
            <AlertDialog.Cancel
              disabled={busy}
              className="px-4 py-2 rounded text-sm border border-[var(--color-border)] text-[var(--color-fg)] disabled:opacity-50"
            >
              {cancelLabel}
            </AlertDialog.Cancel>
            <button
              type="button"
              onClick={onConfirm}
              disabled={busy}
              className={`px-4 py-2 rounded text-sm text-white disabled:opacity-50 ${
                destructive ? 'bg-[var(--color-danger)]' : 'bg-[var(--color-accent)]'
              }`}
            >
              {confirmLabel}
            </button>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  )
}

type Confirm = (options: ConfirmOptions) => Promise<boolean>

const ConfirmContext = createContext<Confirm | null>(null)

/** Provides `useConfirm()`, an in-app replacement for `window.confirm`. */
export function ConfirmProvider({ children }: { children: ReactNode }) {
  const [options, setOptions] = useState<ConfirmOptions | null>(null)
  const resolveRef = useRef<((ok: boolean) => void) | null>(null)

  const confirm = useCallback<Confirm>((next) => {
    // A newer request supersedes one still waiting for an answer.
    resolveRef.current?.(false)
    setOptions(next)
    return new Promise<boolean>((resolve) => {
      resolveRef.current = resolve
    })
  }, [])

  const settle = useCallback((ok: boolean) => {
    resolveRef.current?.(ok)
    resolveRef.current = null
    setOptions(null)
  }, [])

  return (
    <ConfirmContext.Provider value={confirm}>
      {children}
      <ConfirmDialog
        {...(options ?? { title: '' })}
        open={options !== null}
        onConfirm={() => settle(true)}
        onCancel={() => settle(false)}
      />
    </ConfirmContext.Provider>
  )
}

/**
 * Returns `confirm(options)`, which shows the confirmation dialog and resolves
 * true if the user confirms. Must be used within a ConfirmProvider.
 */
export function useConfirm(): Confirm {
  const confirm = useContext(ConfirmContext)
  if (!confirm) throw new Error('useConfirm must be used within ConfirmProvider')
  return confirm
}
