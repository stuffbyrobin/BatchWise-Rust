import { useParams, Link } from 'react-router-dom'
import { useContainerQR } from './hooks/useContainerAssets'
import { APIError } from '../../api/error'

export function ContainerAssetQRPage() {
  const { id } = useParams<{ id: string }>()
  const {
    data: dataA,
    isLoading: isLoadingA,
    isError: isErrorA,
    error: errorA,
    refetch: refetchA,
  } = useContainerQR(id!, 'a')
  const {
    data: dataB,
    isLoading: isLoadingB,
    isError: isErrorB,
    error: errorB,
    refetch: refetchB,
  } = useContainerQR(id!, 'b')

  if (isErrorA || isErrorB) {
    const err = errorA || errorB
    return (
      <div className="p-6">
        <div className="p-4 border border-[var(--color-danger)] rounded bg-[var(--color-danger)/10]">
          <p className="text-[var(--color-danger)]">{err instanceof APIError ? err.message : 'An error occurred'}</p>
          <button
            onClick={() => {
              refetchA()
              refetchB()
            }}
            className="mt-2 px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
          >
            Retry
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <h1 className="text-2xl font-bold text-[var(--color-fg)]">QR Codes</h1>
      <Link
        to={'/container-assets/' + id}
        className="inline-block px-4 py-2 rounded text-sm bg-[var(--color-border)] text-[var(--color-fg)] hover:bg-[var(--color-border)/50]"
      >
        &lt;- Back to asset
      </Link>

      {(isLoadingA || isLoadingB) && (
        <div className="space-y-2 animate-pulse">
          <div className="h-64 rounded bg-[var(--color-border)/20]" />
        </div>
      )}

      {!isLoadingA && !isLoadingB && (
        <div className="flex gap-8">
          <div className="flex flex-col items-center">
            {dataA && (
              <>
                <img
                  src={'data:image/png;base64,' + dataA.png_base64}
                  alt="QR A"
                  style={{ width: 200, height: 200 }}
                />
                <p className="mt-2 text-[var(--color-muted)]">QR Code A</p>
              </>
            )}
          </div>
          <div className="flex flex-col items-center">
            {dataB && (
              <>
                <img
                  src={'data:image/png;base64,' + dataB.png_base64}
                  alt="QR B"
                  style={{ width: 200, height: 200 }}
                />
                <p className="mt-2 text-[var(--color-muted)]">QR Code B</p>
              </>
            )}
          </div>
        </div>
      )}

      <div className="pt-4">
        <button
          onClick={() => window.print()}
          className="px-4 py-2 rounded text-sm bg-[var(--color-accent)] text-white hover:opacity-90"
        >
          Print
        </button>
      </div>
    </div>
  )
}
