import React from 'react'

interface Props {
  children: React.ReactNode
  fallback?: React.ReactNode
}

interface State {
  hasError: boolean
  message: string
}

export class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, message: '' }
  }

  static getDerivedStateFromError(error: unknown): State {
    return {
      hasError: true,
      message: error instanceof Error ? error.message : 'Unknown error',
    }
  }

  override render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback
      return (
        <div className="p-4 rounded border border-[var(--color-danger)] bg-red-50 text-[var(--color-danger)]">
          <p className="font-semibold">Something went wrong.</p>
          <p className="text-sm mt-1">{this.state.message}</p>
          <button
            onClick={() => window.location.reload()}
            className="mt-3 px-3 py-1 text-sm rounded bg-[var(--color-danger)] text-white"
          >
            Retry
          </button>
        </div>
      )
    }
    return this.props.children
  }
}
