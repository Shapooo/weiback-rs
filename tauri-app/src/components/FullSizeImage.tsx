import React, { useState, useRef } from 'react'
import { CircularProgress, Box } from '@mui/material'
import { useImageLoader } from '../hooks/useImageLoader'
import { useVideoLoader } from '../hooks/useVideoLoader'
import { AttachedImage } from '../types'

interface FullSizeImageProps {
  image: AttachedImage
  onClose: () => void
}

const FullSizeImage: React.FC<FullSizeImageProps> = ({ image, onClose }) => {
  const imageId = image.data.id
  const { status: imageStatus, imageUrl } = useImageLoader(imageId)
  const { status: videoStatus, videoUrl } = useVideoLoader(
    image.type === 'livephoto' ? image.data.video_url : undefined
  )
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

  const isLoading = imageStatus === 'idle' || imageStatus === 'loading'
  const showVideo = image.type === 'livephoto' && image.data.video_url && videoStatus === 'loaded'

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
        <video src={videoUrl} style={commonStyle} autoPlay playsInline />
      ) : (
        <img src={imageUrl} alt="Lightbox" style={commonStyle} />
      )}
    </Box>
  )
}

export default FullSizeImage
