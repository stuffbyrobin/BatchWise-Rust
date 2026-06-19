import { LibraryCRUD } from './LibraryCRUD'
import {
  useEquipmentProfiles,
  useCreateEquipmentProfile,
  useUpdateEquipmentProfile,
  useDeleteEquipmentProfile,
} from './hooks/useLibrary'

export function EquipmentProfilesPage() {
  return (
    <LibraryCRUD
      title="Equipment Profiles"
      useList={useEquipmentProfiles}
      useCreate={useCreateEquipmentProfile}
      useUpdate={useUpdateEquipmentProfile}
      useDelete={useDeleteEquipmentProfile}
      fields={[
        { key: 'name', label: 'Name', type: 'text', required: true, sortable: true },
        { key: 'batch_size_liters', label: 'Batch Size (L)', type: 'number', sortable: true },
        { key: 'batch_volume_target_liters', label: 'Batch Volume Target (L)', type: 'number' },
        { key: 'element_power_watts', label: 'Element Power (W)', type: 'number' },
        { key: 'boil_time_minutes', label: 'Boil Time (min)', type: 'number' },
        { key: 'pre_boil_volume_liters', label: 'Pre-Boil Volume (L)', type: 'number' },
        { key: 'boil_off_rate_liters_per_hour', label: 'Boil-Off Rate (L/hr)', type: 'number' },
        { key: 'boil_temp_c', label: 'Boil Temperature (°C)', type: 'number' },
        { key: 'trub_loss_liters', label: 'Trub / Chiller Loss (L)', type: 'number' },
        { key: 'mash_tun_deadspace_liters', label: 'Mash Tun Deadspace (L)', type: 'number' },
        { key: 'mash_tun_loss_liters', label: 'Mash Tun Loss (L)', type: 'number' },
        { key: 'hlt_deadspace_liters', label: 'HLT Deadspace (L)', type: 'number' },
        { key: 'fermenter_loss_liters', label: 'Fermenter Loss (L)', type: 'number' },
        { key: 'top_up_liters', label: 'Top Up (L)', type: 'number' },
        { key: 'mash_time_minutes', label: 'Mash Time (min)', type: 'number' },
        { key: 'brewhouse_efficiency_pct', label: 'Brewhouse Efficiency (%)', type: 'number' },
        { key: 'mash_efficiency_pct', label: 'Mash Efficiency (%)', type: 'number' },
        { key: 'hop_utilisation_pct', label: 'Hop Utilisation (%)', type: 'number' },
        { key: 'aroma_hop_utilisation_pct', label: 'Aroma Hop Utilisation (%)', type: 'number' },
        { key: 'hop_stand_temp_c', label: 'Hop Stand Temperature (°C)', type: 'number' },
        { key: 'altitude_m', label: 'Altitude Adjustment (m)', type: 'number' },
        { key: 'cooling_shrinkage_pct', label: 'Cooling Shrinkage / Boil Expansion (%)', type: 'number' },
        { key: 'grain_absorption_l_per_kg', label: 'Grain Absorption (L/kg)', type: 'number' },
        { key: 'water_to_grain_ratio', label: 'Water to Grain Ratio (L/kg)', type: 'number' },
        { key: 'sparge_water_reminder_liters', label: 'Sparge Water Reminder (L)', type: 'number' },
        { key: 'notes', label: 'Notes', type: 'textarea' },
      ]}
    />
  )
}
