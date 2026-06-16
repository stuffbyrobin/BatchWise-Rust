import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type Recipe = components['schemas']['Recipe']
type RecipeWithIngredients = components['schemas']['RecipeWithIngredients']
type CreateRecipeRequest = components['schemas']['CreateRecipeRequest']
type PatchRecipeRequest = components['schemas']['PatchRecipeRequest']
type ImportRecipeRequest = components['schemas']['ImportRecipeRequest']

interface RecipePage {
  items: Recipe[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

interface ListParams {
  name?: string
  type?: string
  sort?: string
  page?: number
  page_size?: number
}

function toQueryString(params: Record<string, unknown>): string {
  const q = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&')
  return q ? `?${q}` : ''
}

export function useRecipesList(params: ListParams = {}) {
  return useQuery<RecipePage>({
    queryKey: ['recipes', 'list', params],
    queryFn: () =>
      apiClient.get<RecipePage>(`/api/v1/recipes${toQueryString(params as Record<string, unknown>)}`),
  })
}

export function useRecipe(id: string) {
  return useQuery<RecipeWithIngredients>({
    queryKey: ['recipes', id],
    queryFn: () => apiClient.get<RecipeWithIngredients>(`/api/v1/recipes/${id}`),
    enabled: !!id,
  })
}

export function useCreateRecipe() {
  const qc = useQueryClient()
  return useMutation<RecipeWithIngredients, Error, CreateRecipeRequest>({
    mutationFn: (body) => apiClient.post<RecipeWithIngredients>('/api/v1/recipes', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['recipes'] }),
  })
}

export function useUpdateRecipe(id: string) {
  const qc = useQueryClient()
  return useMutation<RecipeWithIngredients, Error, PatchRecipeRequest>({
    mutationFn: (body) => apiClient.put<RecipeWithIngredients>(`/api/v1/recipes/${id}`, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['recipes'] })
      qc.invalidateQueries({ queryKey: ['recipes', id] })
    },
  })
}

export function useDeleteRecipe() {
  const qc = useQueryClient()
  return useMutation<void, Error, string>({
    mutationFn: (id) => apiClient.delete<void>(`/api/v1/recipes/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['recipes'] }),
  })
}

export function useImportRecipe() {
  const qc = useQueryClient()
  return useMutation<RecipeWithIngredients, Error, ImportRecipeRequest>({
    mutationFn: (body) => apiClient.post<RecipeWithIngredients>('/api/v1/recipes/import', body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['recipes'] }),
  })
}
