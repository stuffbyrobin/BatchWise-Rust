import { LibraryCRUD } from './LibraryCRUD'
import {
  useMashProfiles,
  useCreateMashProfile,
  useUpdateMashProfile,
  useDeleteMashProfile,
} from './hooks/useLibrary'
import type { components } from '../../api/generated'

type MashProfile = components['schemas']['MashProfile']

export function MashProfilesPage() {
  return (
    <LibraryCRUD
      title="Mash Profiles"
      useList={useMashProfiles}
      useCreate={useCreateMashProfile}
      useUpdate={useUpdateMashProfile}
      useDelete={useDeleteMashProfile}
      fields={[
        { key: 'name', label: 'Name', type: 'text', required: true },
        { key: 'notes', label: 'Notes', type: 'textarea' },
      ]}
      extraCols={[
        {
          key: 'mash_steps',
          label: 'Steps',
          render: (row) => String((row as unknown as MashProfile).mash_steps?.length ?? 0),
        },
      ]}
    />
  )
}
