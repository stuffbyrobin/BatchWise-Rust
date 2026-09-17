import { describe, expect, it } from 'vitest'
import { qs } from '../qs'

describe('qs', () => {
  it('returns an empty string when there is nothing to send', () => {
    expect(qs({})).toBe('')
    expect(qs({ a: undefined, b: null, c: '' })).toBe('')
  })

  it('keeps zero and false, and encodes keys and values', () => {
    expect(qs({ page: 0, active: false })).toBe('?page=0&active=false')
    expect(qs({ search: 'Maris & Otter', 'a b': '1' })).toBe('?search=Maris%20%26%20Otter&a%20b=1')
  })
})
