import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type Member = components['schemas']['Member']
type MemberList = components['schemas']['MemberList']
type PatchMemberRequest = components['schemas']['PatchMemberRequest']
type InvitationList = components['schemas']['InvitationList']
type CreatedInvitation = components['schemas']['CreatedInvitation']
type CreateInvitationRequest = components['schemas']['CreateInvitationRequest']

export function useMembers() {
  return useQuery<MemberList>({
    queryKey: ['members'],
    queryFn: ({ signal }) => apiClient.get<MemberList>('/api/v1/members', { signal }),
  })
}

export function usePatchMember() {
  const qc = useQueryClient()
  return useMutation<Member, Error, { id: string; body: PatchMemberRequest }>({
    mutationFn: ({ id, body }) => apiClient.patch<Member>(`/api/v1/members/${id}`, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['members'] }),
  })
}

export function useInvitations() {
  return useQuery<InvitationList>({
    queryKey: ['members', 'invitations'],
    queryFn: ({ signal }) => apiClient.get<InvitationList>('/api/v1/members/invitations', { signal }),
  })
}

/** Creates an invitation; the result carries the one-time token for the link. */
export function useCreateInvitation() {
  const qc = useQueryClient()
  return useMutation<CreatedInvitation, Error, CreateInvitationRequest>({
    mutationFn: (body) => apiClient.post<CreatedInvitation>('/api/v1/members/invitations', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['members', 'invitations'] }),
  })
}

export function useRevokeInvitation() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/members/invitations/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['members', 'invitations'] }),
  })
}
