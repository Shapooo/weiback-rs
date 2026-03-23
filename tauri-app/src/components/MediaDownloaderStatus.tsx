import React from 'react'
import { Box, Typography, Tooltip } from '@mui/material'
import { useTaskStore } from '../stores/taskStore'

const MediaDownloaderStatus: React.FC = () => {
  const { active_downloads, queue_length } = useTaskStore(state => state.downloaderStatus)

  // Truncate URL: show first 20 chars + "..." + last 20 chars
  const formatUrl = (url: string): string => {
    if (url.length <= 45) return url
    return url.substring(0, 20) + '...' + url.substring(url.length - 20)
  }

  if (active_downloads.length === 0 && queue_length === 0) {
    return null
  }

  const isProcessing = active_downloads.length > 0

  return (
    <Tooltip
      title={
        isProcessing
          ? active_downloads.map((url, i) => (
              <Box key={i} component="span">
                {url}
              </Box>
            ))
          : '等待下载'
      }
      placement="right"
      arrow
    >
      <Box
        sx={{
          px: 1.5,
          py: 0.75,
          borderTop: '1px solid',
          borderTopColor: 'divider',
        }}
      >
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 0.25 }}>
          {/* Download icon indicator */}
          <Box
            sx={{
              width: 6,
              height: 6,
              borderRadius: '50%',
              bgcolor: isProcessing ? 'primary.main' : 'warning.main',
              ...(isProcessing && {
                animation: 'bounce 1s infinite',
                '@keyframes bounce': {
                  '0%, 100%': { transform: 'translateY(0)' },
                  '50%': { transform: 'translateY(-2px)' },
                },
              }),
            }}
          />
          <Typography
            variant="caption"
            sx={{
              fontSize: '0.6rem',
              color: 'text.secondary',
              textTransform: 'uppercase',
              letterSpacing: '0.5px',
            }}
          >
            媒体资源下载
          </Typography>
        </Box>

        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.1 }}>
          {active_downloads.map((url, i) => (
            <Typography
              key={i}
              variant="caption"
              sx={{
                display: 'block',
                fontFamily: 'monospace',
                fontSize: '0.6rem',
                color: 'text.primary',
                lineHeight: 1.2,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                maxWidth: '170px',
              }}
            >
              {formatUrl(url)}
            </Typography>
          ))}
        </Box>

        {queue_length > 0 && (
          <Typography
            variant="caption"
            sx={{
              fontSize: '0.6rem',
              color: 'text.secondary',
            }}
          >
            队列 {queue_length}
          </Typography>
        )}
      </Box>
    </Tooltip>
  )
}

export default MediaDownloaderStatus
