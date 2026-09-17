import { useState, type FormEvent } from 'react'
import { APIError } from '../../api/error'
import type { components } from '../../api/generated'
import { ROLE_LABELS, ROLES, STAFF_ROLES, type Role } from '../../auth/permissions'
import { useAuth } from '../../auth/useAuth'
import { useConfirm } from '../../components/feedback/ConfirmDialog'
import { useToast } from '../../components/feedback/Toast'
import { fmtDateTime } from '../../utils/format'
import { useCreateInvitation, useInvitations, useMembers, usePatchMember, useRevokeInvitation } from './hooks/useMembers'

type Member = components['schemas']['Member']
type PatchMemberRequest = components['schemas']['PatchMemberRequest']

const ROLE_HELP: Record<Role, string> = {
  owner: 'Everything, including tenant settings.',
  manager: 'All operational data, compliance sign-off, and Brewer, Sales and Viewer members.',
  brewer: 'Recipes, stock, batches and packaging.',
  sales: 'Customers, orders and distribution.',
  viewer: 'Reads everything and changes nothing.',
}

/**
 * Creates an invitation and shows its one-time link. The link carries the token
 * in the URL fragment, so opening it never sends the token in a request line.
 */
function InviteForm({ isOwner }: { isOwner: boolean }) {
  const createInvitation = useCreateInvitation()
  const [email, setEmail] = useState('')
  const [role, setRole] = useState<Role>('brewer')
  const [error, setError] = useState<string | null>(null)
  const [created, setCreated] = useState<{ link: string; email: string; expiresAt: string } | null>(null)
  const [copied, setCopied] = useState(false)
  const options = isOwner ? ROLES : STAFF_ROLES

  const submit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    setError(null)
    setCopied(false)
    try {
      const invitation = await createInvitation.mutateAsync({ email, role })
      setCreated({
        link: `${window.location.origin}/invite#${invitation.token}`,
        email: invitation.email,
        expiresAt: invitation.expires_at,
      })
      setEmail('')
    } catch (err) {
      setError(err instanceof APIError ? err.message : 'Could not create the invitation.')
    }
  }

  const copy = async () => {
    if (!created) return
    try {
      await navigator.clipboard.writeText(created.link)
      setCopied(true)
    } catch {
      setCopied(false)
    }
  }

  return (
    <section className="mb-8">
      <h2 className="text-lg font-semibold mb-2">Invite a member</h2>
      <form onSubmit={submit} className="flex flex-wrap items-end gap-2 text-sm">
        <label className="flex flex-col gap-1">
          <span className="text-xs text-[var(--color-muted)]">Email</span>
          <input
            type="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="border rounded px-2 py-1 bg-[var(--color-surface)]"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs text-[var(--color-muted)]">Role</span>
          <select
            value={role}
            onChange={(e) => setRole(e.target.value as Role)}
            className="border rounded px-2 py-1 bg-[var(--color-surface)]"
          >
            {options.map((r) => (
              <option key={r} value={r}>
                {ROLE_LABELS[r]}
              </option>
            ))}
          </select>
        </label>
        <button
          type="submit"
          disabled={createInvitation.isPending}
          className="px-3 py-1.5 rounded bg-[var(--color-accent)] text-white disabled:opacity-50"
        >
          {createInvitation.isPending ? 'Creating…' : 'Create invite link'}
        </button>
      </form>
      {error && (
        <p role="alert" className="mt-2 text-sm text-[var(--color-danger)]">
          {error}
        </p>
      )}
      {created && (
        <div className="mt-3 rounded border border-[var(--color-border)] bg-[var(--color-surface)] p-3 text-sm">
          <p className="mb-2">
            Send this link to {created.email}. It works once, expires {fmtDateTime(created.expiresAt)}, and is not
            shown again.
          </p>
          <div className="flex gap-2">
            <input
              readOnly
              aria-label="Invite link"
              value={created.link}
              onFocus={(e) => e.target.select()}
              className="flex-1 border rounded px-2 py-1 font-mono text-xs"
            />
            <button type="button" onClick={copy} className="px-3 py-1 rounded border text-sm">
              {copied ? 'Copied' : 'Copy'}
            </button>
          </div>
        </div>
      )}
    </section>
  )
}

/** Open invitations, with Revoke for the ones the user may manage. */
function OpenInvitations({ isOwner }: { isOwner: boolean }) {
  const { data } = useInvitations()
  const revoke = useRevokeInvitation()
  const confirm = useConfirm()
  const { toast } = useToast()
  const items = data?.items ?? []
  if (items.length === 0) return null

  const revokeInvitation = async (id: string, email: string) => {
    const ok = await confirm({
      title: `Revoke the invitation for ${email}?`,
      description: 'The link stops working.',
      confirmLabel: 'Revoke',
      destructive: true,
    })
    if (!ok) return
    try {
      await revoke.mutateAsync(id)
    } catch (e) {
      toast({ title: 'Revoke failed', description: e instanceof APIError ? e.message : undefined, variant: 'destructive' })
    }
  }

  return (
    <section className="mt-8" aria-labelledby="open-invitations">
      <h2 id="open-invitations" className="text-lg font-semibold mb-2">
        Open invitations
      </h2>
      <table className="w-full text-sm">
        <thead>
          <tr className="text-left text-xs uppercase text-[var(--color-muted)] border-b">
            <th className="py-2 pr-3">Email</th>
            <th className="pr-3">Role</th>
            <th className="pr-3">Expires</th>
            <th />
          </tr>
        </thead>
        <tbody className="divide-y divide-[var(--color-border)]">
          {items.map((invitation) => {
            const role = invitation.role as Role
            const expired = new Date(invitation.expires_at).getTime() <= Date.now()
            return (
              <tr key={invitation.id}>
                <td className="py-2 pr-3">{invitation.email}</td>
                <td className="pr-3">{ROLE_LABELS[role]}</td>
                <td className="pr-3">{expired ? 'Expired' : fmtDateTime(invitation.expires_at)}</td>
                <td className="text-right">
                  {(isOwner || STAFF_ROLES.includes(role)) && (
                    <button
                      type="button"
                      className="text-xs hover:underline disabled:opacity-50"
                      disabled={revoke.isPending}
                      onClick={() => revokeInvitation(invitation.id, invitation.email)}
                    >
                      Revoke
                    </button>
                  )}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </section>
  )
}

/**
 * Lists the tenant's members for Owners and Managers, who can invite members,
 * change roles and deactivate members. Managers only manage Brewer, Sales and
 * Viewer members.
 */
export function MembersPage() {
  const { user } = useAuth()
  const { data, isLoading, isError } = useMembers()
  const patchMember = usePatchMember()
  const confirm = useConfirm()
  const { toast } = useToast()
  const isOwner = user?.role === 'owner'

  const update = async (member: Member, body: PatchMemberRequest) => {
    try {
      await patchMember.mutateAsync({ id: member.id, body })
    } catch (e) {
      toast({
        title: 'Update failed',
        description: e instanceof APIError ? e.message : undefined,
        variant: 'destructive',
      })
    }
  }

  const toggleActive = async (member: Member) => {
    if (member.is_active) {
      const ok = await confirm({
        title: `Deactivate ${member.display_name}?`,
        description: 'They are signed out at once and cannot sign in until reactivated.',
        confirmLabel: 'Deactivate',
        destructive: true,
      })
      if (!ok) return
    }
    await update(member, { is_active: !member.is_active })
  }

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <h1 className="text-xl font-semibold mb-2">Members</h1>
      <dl className="mb-6 grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-sm">
        {ROLES.map((role) => (
          <div key={role} className="contents">
            <dt className="font-medium">{ROLE_LABELS[role]}</dt>
            <dd className="text-[var(--color-muted)]">{ROLE_HELP[role]}</dd>
          </div>
        ))}
      </dl>

      <InviteForm isOwner={isOwner} />

      {isLoading && <p className="text-[var(--color-muted)]">Loading…</p>}
      {isError && <p className="text-[var(--color-danger)]">Failed to load members.</p>}

      {data && (
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-xs uppercase text-[var(--color-muted)] border-b">
              <th className="py-2 pr-3">Name</th>
              <th className="pr-3">Email</th>
              <th className="pr-3">Role</th>
              <th className="pr-3">Status</th>
              <th />
            </tr>
          </thead>
          <tbody className="divide-y divide-[var(--color-border)]">
            {data.items.map((member) => {
              const role = member.role as Role
              const manageable = isOwner || STAFF_ROLES.includes(role)
              const options = isOwner ? ROLES : STAFF_ROLES
              const isYou = member.id === user?.user_id
              return (
                <tr key={member.id} className={member.is_active ? undefined : 'text-[var(--color-muted)]'}>
                  <td className="py-2 pr-3">
                    {member.display_name}
                    {isYou && <span className="ml-1 text-xs text-[var(--color-muted)]">(you)</span>}
                  </td>
                  <td className="pr-3">{member.email}</td>
                  <td className="pr-3">
                    {manageable ? (
                      <select
                        aria-label={`Role for ${member.display_name}`}
                        className="border rounded px-2 py-1 text-sm bg-[var(--color-surface)]"
                        value={role}
                        disabled={patchMember.isPending}
                        onChange={(e) => update(member, { role: e.target.value as Role })}
                      >
                        {options.map((r) => (
                          <option key={r} value={r}>
                            {ROLE_LABELS[r]}
                          </option>
                        ))}
                      </select>
                    ) : (
                      ROLE_LABELS[role]
                    )}
                  </td>
                  <td className="pr-3">{member.is_active ? 'Active' : 'Inactive'}</td>
                  <td className="text-right">
                    {manageable && (
                      <button
                        type="button"
                        className="text-xs hover:underline disabled:opacity-50"
                        disabled={patchMember.isPending}
                        onClick={() => toggleActive(member)}
                      >
                        {member.is_active ? 'Deactivate' : 'Reactivate'}
                      </button>
                    )}
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      )}

      <OpenInvitations isOwner={isOwner} />
    </div>
  )
}
