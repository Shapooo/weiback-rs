import React from 'react'
import { Box, Alert, Button, Typography, IconButton, Collapse } from '@mui/material'
import CloseIcon from '@mui/icons-material/Close'
import OpenInNewIcon from '@mui/icons-material/OpenInNew'
import { getVersion } from '@tauri-apps/api/app'
import { useUpdateStore } from '../stores/updateStore'
import { openReleasePage } from '../lib/updateApi'
import { compareVersions } from 'compare-versions'

const CURRENT_VERSION = await getVersion()

const UpdateBanner: React.FC = () => {
  const latestRelease = useUpdateStore(s => s.latestRelease)
  const dismissedVersion = useUpdateStore(s => s.dismissedVersion)
  const dismissVersion = useUpdateStore(s => s.dismissVersion)

  if (!latestRelease) return null

  const hasUpdate = compareVersions(latestRelease.tag_name, CURRENT_VERSION) > 0
  if (!hasUpdate) return null

  if (dismissedVersion === latestRelease.tag_name) return null

  return (
    <Collapse in>
      <Box sx={{ position: 'fixed', top: 16, right: 16, zIndex: 9999, maxWidth: 400 }}>
        <Alert
          severity="info"
          variant="filled"
          sx={{ bgcolor: 'primary.dark' }}
          action={
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
              <Button
                color="inherit"
                size="small"
                endIcon={<OpenInNewIcon />}
                onClick={openReleasePage}
                sx={{ color: 'inherit' }}
              >
                查看
              </Button>
              <IconButton
                color="inherit"
                size="small"
                onClick={() => dismissVersion(latestRelease.tag_name)}
              >
                <CloseIcon fontSize="small" />
              </IconButton>
            </Box>
          }
        >
          <Typography variant="body2">
            发现新版本 <strong>{latestRelease.tag_name}</strong>
          </Typography>
          <Typography variant="caption">点击「查看」前往下载</Typography>
        </Alert>
      </Box>
    </Collapse>
  )
}

export default UpdateBanner
