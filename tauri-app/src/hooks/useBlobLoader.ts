import { useState, useEffect } from 'react'
import { LRUCache } from '../components/LRU'

export type BlobLoadStatus = 'idle' | 'loading' | 'loaded' | 'error'

export interface UseBlobLoaderResult {
  status: BlobLoadStatus
  blobUrl: string
}

export function useBlobLoader(
  id: string | null | undefined,
  cache: LRUCache<string, string>,
  fetcher: (id: string) => Promise<ArrayBuffer>,
  mimeType: string
): UseBlobLoaderResult {
  const [status, setStatus] = useState<BlobLoadStatus>('idle')
  const [blobUrl, setBlobUrl] = useState('')

  useEffect(() => {
    let isCancelled = false

    const loadBlob = async () => {
      if (!id) {
        if (!isCancelled) {
          setBlobUrl('')
          setStatus('idle')
        }
        return
      }

      if (!isCancelled) {
        setStatus('loading')
      }

      const cached = cache.get(id)
      if (cached) {
        if (!isCancelled) {
          setBlobUrl(cached)
          setStatus('loaded')
        }
        return
      }

      try {
        const blob = await fetcher(id)
        if (isCancelled) return

        const objectUrl = URL.createObjectURL(new Blob([blob], { type: mimeType }))
        cache.set(id, objectUrl)
        if (!isCancelled) {
          setBlobUrl(objectUrl)
          setStatus('loaded')
        }
      } catch (error) {
        if (isCancelled) return
        console.error(`Failed to fetch blob ${id}: ${error}`)
        setStatus('error')
        setBlobUrl('')
      }
    }

    loadBlob()
    return () => {
      isCancelled = true
    }
  }, [id, cache, fetcher, mimeType])

  return { status, blobUrl }
}
