/**
 * The BatchWise mark: the malt-tankard icon, and optionally the wordmark next
 * to it. Shared by the landing page, the signed-in app's sidebar, and the
 * public auth pages, so the same brand appears everywhere.
 */
export function BrandMark({
  wordmark = true,
  size = 30,
  className = '',
}: {
  wordmark?: boolean
  size?: number
  className?: string
}) {
  return (
    <div className={`flex items-center gap-2.5 ${className}`}>
      <div
        className="rounded-[9px] bg-(--lp-malt) flex items-center justify-center shrink-0"
        style={{ width: size, height: size }}
      >
        <svg width={size * 0.53} height={size * 0.53} viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M3 1 L13 1 L13 5 Q13 8 10 9.5 L10 15 L6 15 L6 9.5 Q3 8 3 5 Z" fill="white" />
          <circle cx="8" cy="5" r="1.4" fill="var(--lp-malt)" />
        </svg>
      </div>
      {wordmark && (
        <span className="text-[18px] font-bold tracking-[-0.5px] text-(--lp-ink) font-dm-sans">BatchWise</span>
      )}
    </div>
  )
}
