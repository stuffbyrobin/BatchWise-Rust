import { formatEbc } from '../../utils/ebc'

export function EBCSwatch({ ebc }: { ebc: number }) {
  const step = Math.min(10, Math.max(1, Math.round((ebc / 80) * 9) + 1))
  return (
    <span
      title={'EBC: ' + formatEbc(ebc)}
      style={{
        background: 'var(--srm-' + step + ')',
        display: 'inline-block',
        width: 24,
        height: 24,
        borderRadius: 4,
        border: '1px solid var(--color-border)',
      }}
    />
  )
}
