import { APIError } from '../../api/error'
import type { components } from '../../api/generated'
import { ROLE_LABELS, ROLES, STAFF_ROLES, type Role } from '../../auth/permissions'
import { useAuth } from '../../auth/useAuth'
import { useConfirm } from '../../components/feedback/ConfirmDialog'
import { useToast } from '../../components/feedback/Toast'
import { useMembers, usePatchMember } from './hooks/useMembers'

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
 * Lists the tenant's members for Owners and Managers, who can change roles and
 * deactivate members. Managers only manage Brewer, Sales and Viewer members.
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
    </div>
  )
}
