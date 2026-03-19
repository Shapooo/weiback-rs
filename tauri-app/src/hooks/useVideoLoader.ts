import { useCallback } from 'react'
import { videoCache } from '../cache'
import { getVideoBlob } from '../lib/api'
import { useBlobLoader } from './useBlobLoader'

export function useVideoLoader(videoUrl: string | null | undefined) {
  const fetcher = useCallback((url: string) => getVideoBlob(url), [])
  const result = useBlobLoader(videoUrl, videoCache, fetcher, 'video/mp4')
  return { ...result, videoUrl: result.blobUrl }
}
