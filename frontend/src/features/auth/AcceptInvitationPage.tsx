import { useEffect, useState, type FormEvent } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { apiClient } from '../../api/client'
import { APIError } from '../../api/error'
import type { components } from '../../api/generated'
import { PASSWORD_HINT, PASSWORD_MIN_LENGTH, passwordProblem } from '../../auth/password'
import { ROLE_LABELS, type Role } from '../../auth/permissions'
import { useAuth } from '../../auth/useAuth'
import { AuthLayout, authAlertCls, authButtonCls, authInputCls } from '../../components/layout/AuthLayout'

type InvitationPreview = components['schemas']['InvitationPreview']

/**
 * Public page behind an invite link (`/invite#<token>`). The token travels in
 * the URL fragment, so it is never sent in a request line or written to logs.
 * Shows who is inviting and with which role, then creates the account.
 */
export function AcceptInvitationPage() {
  const { hash } = useLocation()
  const token = hash.replace(/^#/, '')
  const navigate = useNavigate()
  const { acceptInvitation } = useAuth()
  const [preview, setPreview] = useState<InvitationPreview | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [displayName, setDisplayName] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    if (!token) return
    let active = true
    apiClient
      .post<InvitationPreview>('/api/v1/auth/invitation', { token })
      .then((p) => {
        if (active) setPreview(p)
      })
      .catch((err) => {
        if (!active) return
        // An expired invitation says so; anything else stays vague.
        setLoadError(
          err instanceof APIError && err.status === 422
            ? err.message
            : 'This invitation is not valid. It may have been used or revoked.',
        )
      })
    return () => {
      active = false
    }
  }, [token])

  const submit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    if (password !== confirmPassword) {
      setError('The passwords do not match.')
      return
    }
    const problem = passwordProblem(password)
    if (problem) {
      setError(problem)
      return
    }
    setSubmitting(true)
    setError(null)
    try {
      await acceptInvitation(token, displayName, password)
      navigate('/app')
    } catch (err) {
      setError(err instanceof APIError ? err.message : 'Could not create your account. Please try again.')
    } finally {
      setSubmitting(false)
    }
  }

  const problem = token ? loadError : 'This invite link is incomplete.'

  return (
    <AuthLayout title="Join a brewery">
      {problem && (
        <div role="alert" className={authAlertCls}>
          {problem}{' '}
          <Link to="/login" className="font-semibold underline">
            Sign in
          </Link>
        </div>
      )}
      {!problem && !preview && <p className="text-sm text-(--lp-muted)">Checking your invitation…</p>}
      {!problem && preview && (
        <>
          <p className="mb-4 text-sm text-(--lp-muted)">
            You have been invited to <strong className="text-(--lp-ink)">{preview.tenant_name}</strong> as{' '}
            <strong className="text-(--lp-ink)">{ROLE_LABELS[preview.role as Role]}</strong>. Your account will use{' '}
            <strong className="text-(--lp-ink)">{preview.email}</strong>.
          </p>
          {error && (
            <div role="alert" className={authAlertCls}>
              {error}
            </div>
          )}
          <form className="space-y-4" onSubmit={submit}>
            <div>
              <label htmlFor="invite-name" className="block text-sm font-medium text-(--lp-ink) mb-1">
                Your Name
              </label>
              <input
                id="invite-name"
                type="text"
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                className={authInputCls}
                autoComplete="name"
                required
              />
            </div>
            <div>
              <label htmlFor="invite-password" className="block text-sm font-medium text-(--lp-ink) mb-1">
                Password
              </label>
              <input
                id="invite-password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className={authInputCls}
                autoComplete="new-password"
                minLength={PASSWORD_MIN_LENGTH}
                aria-describedby="invite-password-hint"
                required
              />
              <p id="invite-password-hint" className="mt-1 text-xs text-(--lp-muted)">
                {PASSWORD_HINT}
              </p>
            </div>
            <div>
              <label htmlFor="invite-confirm" className="block text-sm font-medium text-(--lp-ink) mb-1">
                Confirm password
              </label>
              <input
                id="invite-confirm"
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                className={authInputCls}
                autoComplete="new-password"
                required
              />
            </div>
            <button type="submit" disabled={submitting} className={authButtonCls}>
              {submitting ? 'Creating account…' : 'Create account and join'}
            </button>
          </form>
        </>
      )}
    </AuthLayout>
  )
}
