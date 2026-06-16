import { useQuery } from '@tanstack/react-query'
import { apiClient } from '../../../api/client'
import type { components } from '../../../api/generated'

type AllergenResult = components['schemas']['AllergenResult']

export function useRecipeAllergens(recipeId: string | undefined) {
  return useQuery<AllergenResult>({
    queryKey: ['recipe-allergens', recipeId],
    enabled: !!recipeId,
    queryFn: () => apiClient.get<AllergenResult>(`/api/v1/recipes/${recipeId}/allergens`),
  })
}
