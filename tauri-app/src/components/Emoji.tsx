import React from 'react'
import { emojiCache } from '../cache'
import { Box } from '@mui/material'
import { useImageLoader } from '../hooks/useImageLoader'

interface EmojiProps {
  imageId: string
  emojiText: string // Add emojiText prop for fallback
}

const Emoji: React.FC<EmojiProps> = ({ imageId, emojiText }) => {
  const { status, imageUrl } = useImageLoader(imageId, emojiCache)

  if (status === 'loading' || status === 'idle') {
    return <Box component="span">{emojiText}</Box> // Show text while loading
  }

  if (!imageUrl) {
    return <Box component="span">{emojiText}</Box> // Fallback to text if image fails
  }

  return (
    <Box
      component="img"
      src={imageUrl}
      alt={emojiText} // Use emojiText as alt text
      sx={{
        width: '1.2em',
        height: '1.2em',
        verticalAlign: 'middle',
        display: 'inline-block',
      }}
    />
  )
}

export default Emoji
