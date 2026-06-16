import { LibraryCRUD } from '../library/LibraryCRUD'
import {
  useYeastKineticsList,
  useCreateYeastKinetics,
  useUpdateYeastKinetics,
  useDeleteYeastKinetics,
} from './hooks/useYeastKinetics'
import type { components } from '../../api/generated'

type YeastKinetics = components['schemas']['YeastKinetics']

export function YeastKineticsPage() {
  return (
    <LibraryCRUD<YeastKinetics>
      title="Yeast Kinetics"
      useList={() => useYeastKineticsList()}
      useCreate={() => useCreateYeastKinetics()}
      useUpdate={(id) => useUpdateYeastKinetics(id)}
      useDelete={() => useDeleteYeastKinetics()}
      fields={[
        { key: 'yeast_id', label: 'Yeast ID', type: 'text', required: true },
        { key: 'fermentation_temp_c', label: 'Fermentation Temp (°C)', type: 'number', required: true },
        { key: 'primary_fermentation_days', label: 'Primary Ferm. Days', type: 'number', required: true },
        { key: 'conditioning_days', label: 'Conditioning Days', type: 'number', required: true },
        { key: 'lag_phase_hours', label: 'Lag Phase (hours)', type: 'number' },
        { key: 'attenuation_pct', label: 'Attenuation (%)', type: 'number' },
        { key: 'notes', label: 'Notes', type: 'textarea' },
      ]}
      extraCols={[
        { key: 'fermentation_temp_c', label: 'Temp (°C)', render: (row) => String(row.fermentation_temp_c ?? '—') },
        { key: 'primary_fermentation_days', label: 'Primary days', render: (row) => String(row.primary_fermentation_days ?? '—') },
        { key: 'conditioning_days', label: 'Conditioning days', render: (row) => String(row.conditioning_days ?? '—') },
      ]}
    />
  )
}
