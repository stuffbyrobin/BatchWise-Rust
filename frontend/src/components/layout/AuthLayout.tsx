import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'
import { BrandMark } from '../BrandMark'

/**
 * Shared chrome for the public auth pages (login, register, accept-invitation):
 * a centered card on the landing page's cream background, with the brand mark
 * linking home. Keeps those pages visually part of the same product as the
 * marketing site they follow.
 */
export function AuthLayout({ title, subtitle, children }: { title: string; subtitle?: string; children: ReactNode }) {
  return (
    <div className="min-h-screen flex items-center justify-center p-6 bg-(--lp-bg)">
      <div className="w-full max-w-md">
        <div className="flex justify-center mb-8">
          <Link to="/" className="no-underline">
            <BrandMark />
          </Link>
        </div>
        <div
          className="bg-(--lp-card) rounded-[24px] border border-(--lp-rule) p-8"
          style={{ boxShadow: '0 24px 60px rgba(122,59,20,.10)' }}
        >
          <h1 className="text-[26px] font-bold tracking-[-0.6px] text-(--lp-ink) font-dm-sans mb-1">{title}</h1>
          {subtitle && <p className="text-[14px] text-(--lp-muted) mb-6">{subtitle}</p>}
          <div className={subtitle ? undefined : 'mt-6'}>{children}</div>
        </div>
      </div>
    </div>
  )
}

/** Shared classes for a text/email/password field on an auth page. */
export const authInputCls =
  'w-full px-3 py-2.5 rounded-lg border border-(--lp-rule) bg-(--lp-card) text-(--lp-ink) text-[14px] ' +
  'placeholder:text-(--lp-faint) focus:outline-none focus:ring-2 focus:ring-(--lp-malt) focus:border-transparent'

/** Shared classes for an auth page's primary submit button. */
export const authButtonCls =
  'w-full bg-(--lp-malt-deep) text-white text-[15px] font-semibold py-2.5 rounded-full font-dm-sans ' +
  'hover:brightness-110 transition-[background,transform] duration-150 hover:enabled:translate-y-[-1px] ' +
  'disabled:opacity-50'

/** Shared classes for an inline error/notice banner on an auth page. */
export const authAlertCls = 'bg-(--lp-alert-soft) text-(--lp-alert) text-[13px] p-3 rounded-lg mb-4'
