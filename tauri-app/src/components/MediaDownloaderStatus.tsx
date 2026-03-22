import React from 'react'
import { Box, Typography, Tooltip } from '@mui/material'
import { useTaskStore } from '../stores/taskStore'

const MediaDownloaderStatus: React.FC = () => {
  const { current_url, queue_length, is_processing } = useTaskStore(state => state.downloaderStatus)

  // Truncate URL: show first 20 chars + "..." + last 20 chars
  const formatUrl = (url: string | null): string => {
    if (!url) return ''
    if (url.length <= 45) return url
    return url.substring(0, 20) + '...' + url.substring(url.length - 20)
  }

  if (!is_processing && queue_length === 0) {
    return null
  }

  return (
    <Tooltip title={current_url || '等待下载'} placement="right" arrow>
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
              bgcolor: is_processing ? 'primary.main' : 'warning.main',
              ...(is_processing && {
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

        <Typography
          variant="caption"
          sx={{
            display: 'block',
            fontFamily: 'monospace',
            fontSize: '0.65rem',
            color: 'text.primary',
            lineHeight: 1.3,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            maxWidth: '170px',
          }}
        >
          {is_processing && current_url ? formatUrl(current_url) : '等待中'}
        </Typography>

        <Typography
          variant="caption"
          sx={{
            fontSize: '0.6rem',
            color: 'text.secondary',
          }}
        >
          {queue_length > 0 ? `剩余 ${queue_length}` : '完成'}
        </Typography>
      </Box>
    </Tooltip>
  )
}

export default MediaDownloaderStatus
