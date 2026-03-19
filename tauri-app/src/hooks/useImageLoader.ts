import { useState, useEffect } from 'react'
import { LRUCache } from '../components/LRU'
import { getPictureBlob } from '../lib/api'

export type ImageLoadStatus = 'idle' | 'loading' | 'loaded' | 'error' | 'not-found'

interface UseImageLoaderResult {
  status: ImageLoadStatus
  imageUrl: string
}

export function useImageLoader(
  imageId: string | null | undefined,
  cache: LRUCache<string, string>
): UseImageLoaderResult {
  const [status, setStatus] = useState<ImageLoadStatus>('idle')
  const [imageUrl, setImageUrl] = useState('')

  useEffect(() => {
    let isCancelled = false

    const loadImage = async () => {
      // Early return if imageId is null/undefined
      if (!imageId) {
        if (!isCancelled) {
          setImageUrl('')
          setStatus('idle')
        }
        return
      }

      if (!isCancelled) {
        setStatus('loading')
      }

      // Check cache first
      const cached = cache.get(imageId)
      if (cached) {
        if (!isCancelled) {
          setImageUrl(cached)
          setStatus('loaded')
        }
        return
      }

      try {
        const blob = await getPictureBlob(imageId)
        if (isCancelled) return

        const objectUrl = URL.createObjectURL(new Blob([blob]))
        cache.set(imageId, objectUrl)
        setImageUrl(objectUrl)
        setStatus('loaded')
      } catch (error) {
        if (isCancelled) return
        if (
          error &&
          typeof error === 'object' &&
          'kind' in error &&
          (error as any).kind === 'NotFound'
        ) {
          setStatus('not-found')
        } else {
          console.error(`Failed to fetch image ${imageId}: ${error}`)
          setStatus('error')
        }
        setImageUrl('')
      }
    }

    loadImage()
    return () => {
      isCancelled = true
    }
  }, [imageId, cache])

  return { status, imageUrl }
}
