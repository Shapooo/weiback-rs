import React from 'react'
import { Box, Card, CardContent, Typography, Button, Chip } from '@mui/material'
import Grid from '@mui/material/Grid'
import InfoOutlined from '@mui/icons-material/InfoOutlined'
import OpenInNewIcon from '@mui/icons-material/OpenInNew'
import GitHubIcon from '@mui/icons-material/GitHub'
import { getVersion } from '@tauri-apps/api/app'
import { useUpdateStore } from '../stores/updateStore'
import { checkLatestRelease, openReleasePage, openProjectPage } from '../lib/updateApi'
import { compareVersions } from 'compare-versions'
import { useSnackbar } from 'notistack'

const CURRENT_VERSION = await getVersion()

const AboutPage: React.FC = () => {
  const { enqueueSnackbar } = useSnackbar()
  const latestRelease = useUpdateStore(s => s.latestRelease)
  const setLatestRelease = useUpdateStore(s => s.setLatestRelease)
  const lastChecked = useUpdateStore(s => s.lastChecked)
  const setLastChecked = useUpdateStore(s => s.setLastChecked)

  const hasUpdate = latestRelease && compareVersions(latestRelease.tag_name, CURRENT_VERSION) > 0

  const handleCheckUpdate = async () => {
    const release = await checkLatestRelease()
    if (release) {
      setLatestRelease(release)
      setLastChecked(Date.now())
      if (compareVersions(release.tag_name, CURRENT_VERSION) > 0) {
        enqueueSnackbar(`发现新版本 ${release.tag_name}，点击查看详情。`, {
          variant: 'info',
          action: () => (
            <Button color="inherit" size="small" onClick={openReleasePage}>
              查看
            </Button>
          ),
        })
      } else {
        enqueueSnackbar('当前已是最新版本。', { variant: 'success' })
      }
    } else {
      enqueueSnackbar('检查更新失败，请稍后重试。', { variant: 'error' })
    }
  }

  const formatDate = (isoString: string) => {
    const d = new Date(isoString)
    return d.toLocaleDateString('zh-CN', { year: 'numeric', month: 'long', day: 'numeric' })
  }

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" gutterBottom>
        关于
      </Typography>
      <Grid container justifyContent="center">
        <Grid size={{ xs: 12, md: 10, lg: 8 }}>
          <Card sx={{ mt: 3 }}>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
                <InfoOutlined color="primary" />
                <Typography variant="h6">WeiBack</Typography>
                <Chip label={`v${CURRENT_VERSION}`} size="small" />
              </Box>

              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                用于备份微博（Weibo）数据的桌面应用程序，基于 Tauri 2 构建。
              </Typography>

              {hasUpdate && latestRelease && (
                <Box
                  sx={{
                    bgcolor: 'primary.main',
                    color: 'primary.contrastText',
                    borderRadius: 1,
                    p: 2,
                    mb: 2,
                  }}
                >
                  <Typography variant="body2" sx={{ fontWeight: 600 }}>
                    发现新版本 {latestRelease.tag_name}
                  </Typography>
                  <Typography variant="caption" sx={{ display: 'block', mb: 1 }}>
                    发布于 {formatDate(latestRelease.published_at)}
                  </Typography>
                  <Button
                    variant="contained"
                    color="inherit"
                    size="small"
                    endIcon={<OpenInNewIcon />}
                    onClick={openReleasePage}
                  >
                    查看更新详情
                  </Button>
                </Box>
              )}

              <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
                <Button variant="outlined" size="small" onClick={handleCheckUpdate}>
                  检查更新
                </Button>
                <Button
                  variant="outlined"
                  size="small"
                  startIcon={<GitHubIcon />}
                  onClick={openProjectPage}
                >
                  项目主页
                </Button>
                <Button
                  variant="outlined"
                  size="small"
                  startIcon={<OpenInNewIcon />}
                  onClick={openReleasePage}
                >
                  Releases
                </Button>
              </Box>

              {lastChecked && (
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ display: 'block', mt: 2 }}
                >
                  上次检查: {new Date(lastChecked).toLocaleString('zh-CN')}
                </Typography>
              )}
            </CardContent>
          </Card>

          <Typography variant="body2" color="text.secondary" sx={{ mt: 2 }}>
            本程序依据 Apache License 2.0 开源。
          </Typography>
        </Grid>
      </Grid>
    </Box>
  )
}

export default AboutPage
