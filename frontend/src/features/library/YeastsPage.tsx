import { LibraryCRUD } from './LibraryCRUD'
import { useYeasts, useCreateYeast, useUpdateYeast, useDeleteYeast } from './hooks/useLibrary'

export function YeastsPage() {
  return (
    <LibraryCRUD
      title="Yeasts"
      useList={useYeasts}
      useCreate={useCreateYeast}
      useUpdate={useUpdateYeast}
      useDelete={useDeleteYeast}
      fields={[
        { key: 'name', label: 'Name', type: 'text', required: true, sortable: true },
        {
          key: 'type',
          label: 'Type',
          type: 'select',
          required: true,
          options: ['ale', 'lager', 'wild', 'bacteria', 'other'],
          sortable: true,
        },
        { key: 'manufacturer', label: 'Manufacturer', type: 'text' },
        { key: 'product_code', label: 'Product code', type: 'text', sortable: true },
        { key: 'form', label: 'Form', type: 'select', options: ['dry', 'liquid', 'slant'] },
        { key: 'attenuation_min_pct', label: 'Attenuation min %', type: 'number' },
        { key: 'attenuation_max_pct', label: 'Attenuation max %', type: 'number' },
        { key: 'temp_min_c', label: 'Temp min °C', type: 'number' },
        { key: 'temp_max_c', label: 'Temp max °C', type: 'number' },
        { key: 'flocculation', label: 'Flocculation', type: 'text' },
        { key: 'notes', label: 'Notes', type: 'textarea' },
      ]}
    />
  )
}
