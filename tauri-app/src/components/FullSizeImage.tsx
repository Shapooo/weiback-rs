import React, { useState, useEffect, useRef } from 'react'
import { CircularProgress, Box } from '@mui/material'
import { attachedCache } from '../cache'
import { getVideoBlob } from '../lib/api'
import { useImageLoader } from '../hooks/useImageLoader'
import { AttachedImage } from '../types'

interface FullSizeImageProps {
  image: AttachedImage
  onClose: () => void
}

const FullSizeImage: React.FC<FullSizeImageProps> = ({ image, onClose }) => {
  const imageId = image.data.id
  const { status: imageStatus, imageUrl } = useImageLoader(imageId, attachedCache)
  const [videoUrl, setVideoUrl] = useState<string>('')
  const [showVideo, setShowVideo] = useState(false)
  const [videoLoading, setVideoLoading] = useState(false)
  const [transform, setTransform] = useState({
    scale: 1,
    originX: '50%',
    originY: '50%',
  })
  const [position, setPosition] = useState({ x: 0, y: 0 })
  const [isDragging, setIsDragging] = useState(false)
  const dragStartRef = useRef({ startX: 0, startY: 0, initialX: 0, initialY: 0 })
  const didDragRef = useRef(false)
  const containerRef = useRef<HTMLDivElement>(null)

  // Video loading with special cache key logic
  useEffect(() => {
    if (image.type !== 'livephoto' || !image.data.video_url) return

    let isCancelled = false
    setVideoLoading(true)
    const videoCacheKey = `video-${image.data.video_url}`

    const fetchVideo = async () => {
      const cachedVideoUrl = attachedCache.get(videoCacheKey)
      if (cachedVideoUrl) {
        if (!isCancelled) {
          setVideoUrl(cachedVideoUrl)
          setShowVideo(true)
          setVideoLoading(false)
        }
        return
      }

      try {
        const blob = await getVideoBlob(image.data.video_url)
        if (!isCancelled && blob.byteLength > 0) {
          const videoBlob = new Blob([blob], { type: 'video/mp4' })
          const objectUrl = URL.createObjectURL(videoBlob)
          attachedCache.set(videoCacheKey, objectUrl)
          setVideoUrl(objectUrl)
          setShowVideo(true)
        }
      } catch (error) {
        console.error(`Failed to fetch video ${image.data.video_url}: ${error}`)
      } finally {
        if (!isCancelled) setVideoLoading(false)
      }
    }

    fetchVideo()
    return () => {
      isCancelled = true
    }
  }, [image])

  const isLoading = imageStatus === 'idle' || imageStatus === 'loading' || videoLoading

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault()

    const zoomFactor = 1.1
    const newScale = e.deltaY < 0 ? transform.scale * zoomFactor : transform.scale / zoomFactor
    const clampedScale = Math.min(Math.max(1, newScale), 10)

    if (clampedScale === 1) {
      // Reset position and origin when zoomed back to original size
      setPosition({ x: 0, y: 0 })
      setTransform({ scale: 1, originX: '50%', originY: '50%' })
    } else if (containerRef.current) {
      const rect = containerRef.current.getBoundingClientRect()
      const newOriginX = `${((e.clientX - rect.left) / rect.width) * 100}%`
      const newOriginY = `${((e.clientY - rect.top) / rect.height) * 100}%`
      setTransform({
        scale: clampedScale,
        originX: newOriginX,
        originY: newOriginY,
      })
    }
  }

  const handleMouseDown = (e: React.MouseEvent) => {
    didDragRef.current = false
    if (transform.scale <= 1) return
    e.preventDefault()
    setIsDragging(true)
    dragStartRef.current = {
      startX: e.clientX,
      startY: e.clientY,
      initialX: position.x,
      initialY: position.y,
    }
  }

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging) return
    didDragRef.current = true
    e.preventDefault()
    const dx = e.clientX - dragStartRef.current.startX
    const dy = e.clientY - dragStartRef.current.startY
    setPosition({
      x: dragStartRef.current.initialX + dx,
      y: dragStartRef.current.initialY + dy,
    })
  }

  const handleMouseUpOrLeave = () => {
    setIsDragging(false)
  }

  const handleClick = (e: React.MouseEvent) => {
    if (didDragRef.current) {
      e.preventDefault()
      return
    }
    onClose()
  }

  if (isLoading) {
    return <CircularProgress sx={{ color: 'white' }} />
  }

  const commonStyle: React.CSSProperties = {
    maxHeight: '90vh',
    maxWidth: '90vw',
    borderRadius: '4px',
    display: 'block',
  }

  return (
    <Box
      ref={containerRef}
      onClick={handleClick}
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUpOrLeave}
      onMouseLeave={handleMouseUpOrLeave}
      sx={{
        transform: `translate(${position.x}px, ${position.y}px) scale(${transform.scale})`,
        transformOrigin: `${transform.originX} ${transform.originY}`,
        transition: isDragging ? 'none' : 'transform 0.1s ease-out',
        cursor: transform.scale > 1 ? (isDragging ? 'grabbing' : 'grab') : 'zoom-in',
        position: 'relative',
      }}
    >
      {showVideo ? (
        <video
          src={videoUrl}
          style={commonStyle}
          autoPlay
          playsInline
          onEnded={() => setShowVideo(false)}
        />
      ) : (
        <img src={imageUrl} alt="Lightbox" style={commonStyle} />
      )}
    </Box>
  )
}

export default FullSizeImage
