import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type Member = components['schemas']['Member']
type MemberList = components['schemas']['MemberList']
type PatchMemberRequest = components['schemas']['PatchMemberRequest']

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
