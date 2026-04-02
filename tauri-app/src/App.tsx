import React, { useEffect, useState, useCallback, useRef } from 'react'
import {
  Box,
  Drawer,
  CssBaseline,
  Typography,
  Backdrop,
  CircularProgress,
  Button,
  Alert,
  AlertTitle,
} from '@mui/material'
import { useSnackbar } from 'notistack'
import { MainListItems } from './listItems'
import AppRouter from './router'
import { useTaskEvents } from './hooks/useTaskEvents'
import { useTaskStore } from './stores/taskStore'
import { useAuthStore } from './stores/authStore'
import { getBackendStatus, initBackend, BackendStatus } from './lib/api'
import GlobalTaskProgress from './components/GlobalTaskProgress'
import MediaDownloaderStatus from './components/MediaDownloaderStatus'
import useCompletionNotifier from './hooks/useCompletionNotifier'
import CloseConfirmDialog from './components/CloseConfirmDialog'
import UpdateBanner from './components/UpdateBanner'
import { checkLatestRelease } from './lib/updateApi'
import { useUpdateStore } from './stores/updateStore'
import { getCurrentWindow } from '@tauri-apps/api/window'

const drawerWidth = 200
const taskProgressHeight = 80

const App: React.FC = () => {
  const { enqueueSnackbar } = useSnackbar()
  const [backendStatus, setBackendStatus] = useState<BackendStatus>({ status: 'Uninitialized' })
  const [loading, setLoading] = useState(true)
  const currentTask = useTaskStore(state => state.currentTask)
  const isTaskRunning = currentTask?.status === 'InProgress'
  const [closeDialogOpen, setCloseDialogOpen] = useState(false)
  const userConfirmedCloseRef = useRef(false)

  const checkAndInitBackend = useCallback(async () => {
    setLoading(true)
    try {
      let status = await getBackendStatus()
      if (status.status === 'Uninitialized' || status.status === 'Error') {
        status = await initBackend()
      }

      setBackendStatus(status)
      if (status.status === 'Running') {
        if (status.warning) {
          enqueueSnackbar(`配置文件加载失败，已使用默认配置。错误详情: ${status.warning}`, {
            variant: 'warning',
            persist: true,
          })
        }
        useAuthStore.getState().checkLoginState()

        // Check for updates silently after backend is ready
        const release = await checkLatestRelease()
        if (release) {
          useUpdateStore.getState().setLatestRelease(release)
          useUpdateStore.getState().setLastChecked(Date.now())
        }
      }
    } catch (e) {
      setBackendStatus({ status: 'Error', message: String(e) })
    } finally {
      setLoading(false)
    }
  }, [enqueueSnackbar])

  // Start listening for global task events
  useTaskEvents(backendStatus.status === 'Running')
  // Enable global notifications for task completion/failure
  useCompletionNotifier()

  const handleCloseConfirm = useCallback(async () => {
    setCloseDialogOpen(false)
    userConfirmedCloseRef.current = true
    await getCurrentWindow().close()
  }, [])

  const handleCloseCancel = useCallback(() => {
    setCloseDialogOpen(false)
  }, [])

  useEffect(() => {
    const setupCloseListener = async () => {
      const appWindow = getCurrentWindow()
      const unlisten = await appWindow.onCloseRequested(event => {
        const task = useTaskStore.getState()
        const hasRunning =
          task.currentTask?.status === 'InProgress' ||
          task.downloaderStatus.active_downloads.length > 0
        if (hasRunning && !userConfirmedCloseRef.current) {
          event.preventDefault()
          setCloseDialogOpen(true)
        }
      })
      return unlisten
    }

    const unlistenPromise = setupCloseListener()
    return () => {
      unlistenPromise.then(unlisten => unlisten())
    }
  }, [])

  useEffect(() => {
    checkAndInitBackend()
  }, [checkAndInitBackend])

  if (backendStatus.status !== 'Running') {
    return (
      <Backdrop
        sx={{
          color: '#fff',
          zIndex: theme => theme.zIndex.drawer + 2,
          backgroundColor: 'rgba(0, 0, 0, 0.8)',
        }}
        open={true}
      >
        <Box sx={{ textAlign: 'center', p: 4, maxWidth: 500 }}>
          {loading ? (
            <>
              <CircularProgress color="inherit" />
              <Typography sx={{ mt: 2 }}>正在启动后端服务...</Typography>
            </>
          ) : backendStatus.status === 'Error' || backendStatus.status === 'Uninitialized' ? (
            <Alert
              severity="error"
              action={
                <Button color="inherit" size="small" onClick={checkAndInitBackend}>
                  重试
                </Button>
              }
            >
              <AlertTitle>后端启动失败</AlertTitle>
              <Typography variant="body2" sx={{ mb: 1 }}>
                程序无法正常连接到后端核心服务，可能是由于配置文件错误或数据库连接失败，请查看日志。
              </Typography>
              <Typography
                variant="caption"
                sx={{ display: 'block', wordBreak: 'break-all', opacity: 0.8 }}
              >
                错误信息: {backendStatus.status === 'Error' ? backendStatus.message : '未知原因'}
              </Typography>
            </Alert>
          ) : null}
        </Box>
      </Backdrop>
    )
  }

  return (
    <Box sx={{ display: 'flex' }}>
      <CssBaseline />
      <Drawer
        variant="permanent"
        sx={{
          width: drawerWidth,
          flexShrink: 0,
          [`& .MuiDrawer-paper`]: { width: drawerWidth, boxSizing: 'border-box' },
        }}
      >
        <Box sx={{ overflow: 'auto', display: 'flex', flexDirection: 'column', height: '100%' }}>
          <MainListItems />
          <Box sx={{ mt: 'auto' }}>
            <MediaDownloaderStatus />
          </Box>
        </Box>
      </Drawer>
      <Box
        component="main"
        sx={{
          flexGrow: 1,
          p: 3,
          pb: isTaskRunning ? `${3 * 8 + taskProgressHeight}px` : 3,
          width: `calc(100% - ${drawerWidth}px)`,
        }}
      >
        <AppRouter />
      </Box>
      <GlobalTaskProgress />
      <CloseConfirmDialog
        open={closeDialogOpen}
        onConfirm={handleCloseConfirm}
        onCancel={handleCloseCancel}
      />
      <UpdateBanner />
    </Box>
  )
}

export default App
