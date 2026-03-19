import { useCallback } from 'react'
import { attachedCache } from '../cache'
import { getPictureBlob } from '../lib/api'
import { useBlobLoader } from './useBlobLoader'
import { LRUCache } from '../components/LRU'

export function useImageLoader(
  imageId: string | null | undefined,
  cache: LRUCache<string, string> = attachedCache
) {
  const fetcher = useCallback((id: string) => getPictureBlob(id), [])
  const result = useBlobLoader(imageId, cache, fetcher, 'image/*')
  return { ...result, imageUrl: result.blobUrl }
}
