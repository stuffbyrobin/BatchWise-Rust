import { LibraryCRUD } from './LibraryCRUD'
import { useStyles, useCreateStyle, useUpdateStyle, useDeleteStyle } from './hooks/useLibrary'

export function StylesPage() {
  return (
    <LibraryCRUD
      title="Beer Styles"
      useList={useStyles}
      useCreate={useCreateStyle}
      useUpdate={useUpdateStyle}
      useDelete={useDeleteStyle}
      fields={[
        { key: 'name', label: 'Name', type: 'text', required: true },
        { key: 'category', label: 'Category', type: 'text' },
        { key: 'og_min', label: 'OG min', type: 'number' },
        { key: 'og_max', label: 'OG max', type: 'number' },
        { key: 'fg_min', label: 'FG min', type: 'number' },
        { key: 'fg_max', label: 'FG max', type: 'number' },
        { key: 'ibu_min', label: 'IBU min', type: 'number' },
        { key: 'ibu_max', label: 'IBU max', type: 'number' },
        { key: 'color_ebc_min', label: 'EBC min', type: 'number' },
        { key: 'color_ebc_max', label: 'EBC max', type: 'number' },
        { key: 'notes', label: 'Notes', type: 'textarea' },
      ]}
    />
  )
}
