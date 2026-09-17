import { createCrudHooks } from '../../../api/crud'
import type { components } from '../../../api/generated'

type BeerStyle = components['schemas']['Style']
type EquipmentProfile = components['schemas']['EquipmentProfile']
type MashProfile = components['schemas']['MashProfile']
type Yeast = components['schemas']['Yeast']
type Fermentable = components['schemas']['LibraryFermentable']

type ListParams = { page?: number; page_size?: number; sort?: string }

// ── Styles ────────────────────────────────────────────────────────────────────

const styles = createCrudHooks<BeerStyle, Partial<BeerStyle>, Partial<BeerStyle>, ListParams>({
  path: '/api/v1/library/styles',
  queryKey: ['library', 'styles'],
  updateMethod: 'put',
})
export const useStyles = styles.useList
export const useStyle = styles.useOne
export const useCreateStyle = styles.useCreate
export const useUpdateStyle = styles.useUpdate
export const useDeleteStyle = styles.useDelete

// ── Equipment Profiles ────────────────────────────────────────────────────────

const equipmentProfiles = createCrudHooks<EquipmentProfile, Partial<EquipmentProfile>, Partial<EquipmentProfile>, ListParams>({
  path: '/api/v1/library/equipment-profiles',
  queryKey: ['library', 'equipment-profiles'],
  updateMethod: 'put',
})
export const useEquipmentProfiles = equipmentProfiles.useList
export const useEquipmentProfile = equipmentProfiles.useOne
export const useCreateEquipmentProfile = equipmentProfiles.useCreate
export const useUpdateEquipmentProfile = equipmentProfiles.useUpdate
export const useDeleteEquipmentProfile = equipmentProfiles.useDelete

// ── Mash Profiles ─────────────────────────────────────────────────────────────

const mashProfiles = createCrudHooks<MashProfile, Partial<MashProfile>, Partial<MashProfile>, ListParams>({
  path: '/api/v1/library/mash-profiles',
  queryKey: ['library', 'mash-profiles'],
  updateMethod: 'put',
})
export const useMashProfiles = mashProfiles.useList
export const useMashProfile = mashProfiles.useOne
export const useCreateMashProfile = mashProfiles.useCreate
export const useUpdateMashProfile = mashProfiles.useUpdate
export const useDeleteMashProfile = mashProfiles.useDelete

// ── Yeasts ────────────────────────────────────────────────────────────────────

const yeasts = createCrudHooks<Yeast, Partial<Yeast>, Partial<Yeast>, ListParams>({
  path: '/api/v1/library/yeasts',
  queryKey: ['library', 'yeasts'],
  updateMethod: 'put',
})
export const useYeasts = yeasts.useList
export const useYeast = yeasts.useOne
export const useCreateYeast = yeasts.useCreate
export const useUpdateYeast = yeasts.useUpdate
export const useDeleteYeast = yeasts.useDelete

// ── Fermentables ──────────────────────────────────────────────────────────────

const fermentables = createCrudHooks<Fermentable, Partial<Fermentable>, Partial<Fermentable>, ListParams & { name?: string; supplier?: string; type?: string }>({
  path: '/api/v1/library/fermentables',
  queryKey: ['library', 'fermentables'],
  updateMethod: 'put',
})
export const useFermentables = fermentables.useList
export const useCreateFermentable = fermentables.useCreate
export const useUpdateFermentable = fermentables.useUpdate
export const useDeleteFermentable = fermentables.useDelete
