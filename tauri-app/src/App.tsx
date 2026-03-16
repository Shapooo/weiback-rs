import React, { useRef, useEffect, useState } from 'react';
import { Box, Drawer, CssBaseline, LinearProgress, Typography, Backdrop, CircularProgress, Button, Alert, AlertTitle } from '@mui/material';
import { useSnackbar } from 'notistack';
import { MainListItems } from './listItems';
import AppRouter from './router';
import { useTaskEvents } from './hooks/useTaskEvents';
import { useTaskStore } from './stores/taskStore';
import { useAuthStore } from './stores/authStore';
import { Task } from './types';
import { getBackendStatus, initBackend, BackendStatus } from './lib/api';


const drawerWidth = 200;
const taskProgressHeight = 80; // GlobalTaskProgress height + padding

function useCompletionNotifier() {
    const { enqueueSnackbar } = useSnackbar();
    const task = useTaskStore((state) => state.currentTask);
    const prevTaskRef = useRef<Task | null>(null);

    useEffect(() => {
        const prevTask = prevTaskRef.current;
        // Check for task completion or failure by comparing previous and current state
        if (prevTask && !task) { // Task just finished (current is null, prev was not)
            if (prevTask.status === 'Completed') {
                enqueueSnackbar(`任务 "${prevTask.description}" 已完成！`, { variant: 'success' });
            } else if (prevTask.status === 'Failed') {
                enqueueSnackbar(`任务 "${prevTask.description}" 失败: ${prevTask.error || '未知错误'}`, { variant: 'error', persist: true });
            }
        }
        // Update the ref for the next render
        prevTaskRef.current = task;
    }, [task, enqueueSnackbar]);
}

function GlobalTaskProgress() {
    const task = useTaskStore((state) => state.currentTask);

    if (!task || task.status !== 'InProgress') {
        return null; // Don't show anything if there's no active task
    }

    const progress = task.total > 0 ? (task.progress / task.total) * 100 : 0;

    return (
        <Box sx={{
            position: 'fixed',
            bottom: 0,
            left: drawerWidth, // Align with the main content area
            width: `calc(100% - ${drawerWidth}px)`,
            p: 2,
            bgcolor: 'background.paper',
            zIndex: (theme) => theme.zIndex.drawer + 1, // Appear above the drawer
            borderTop: '1px solid',
            borderColor: 'divider'
        }}>
            <Typography variant="body2" gutterBottom>{task.description}</Typography>
            <LinearProgress variant="determinate" value={progress} />
            <Typography variant="caption" color="text.secondary">{`${task.progress} / ${task.total}`}</Typography>
        </Box>
    );
}


const App: React.FC = () => {
    const { enqueueSnackbar } = useSnackbar();
    const [backendStatus, setBackendStatus] = useState<BackendStatus>({ status: 'Uninitialized' });
    const [loading, setLoading] = useState(true);
    const currentTask = useTaskStore((state) => state.currentTask);
    const isTaskRunning = currentTask?.status === 'InProgress';

    const checkAndInitBackend = async () => {
        setLoading(true);
        try {
            let status = await getBackendStatus();
            if (status.status === 'Uninitialized' || status.status === 'Error') {
                status = await initBackend();
            }

            setBackendStatus(status);
            if (status.status === 'Running') {
                if (status.warning) {
                    enqueueSnackbar(`配置文件加载失败，已使用默认配置。错误详情: ${status.warning}`, {
                        variant: 'warning',
                        persist: true,
                    });
                }
                useAuthStore.getState().checkLoginState();
            }
        } catch (e) {
            setBackendStatus({ status: 'Error', message: String(e) });
        } finally {
            setLoading(false);
        }
    };

    // Start listening for global task events
    useTaskEvents(backendStatus.status === 'Running');
    // Enable global notifications for task completion/failure
    useCompletionNotifier();

    useEffect(() => {
        checkAndInitBackend();
    }, []);

    if (backendStatus.status !== 'Running') {
        return (
            <Backdrop
                sx={{ color: '#fff', zIndex: (theme) => theme.zIndex.drawer + 2, backgroundColor: 'rgba(0, 0, 0, 0.8)' }}
                open={true}
            >
                <Box sx={{ textAlign: 'center', p: 4, maxWidth: 500 }}>
                    {loading ? (
                        <>
                            <CircularProgress color="inherit" />
                            <Typography sx={{ mt: 2 }}>正在启动后端服务...</Typography>
                        </>
                    ) : (backendStatus.status === 'Error' || backendStatus.status === 'Uninitialized') ? (
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
                            <Typography variant="caption" sx={{ display: 'block', wordBreak: 'break-all', opacity: 0.8 }}>
                                错误信息: {backendStatus.status === 'Error' ? backendStatus.message : '未知原因'}
                            </Typography>
                        </Alert>
                    ) : null}
                </Box>
            </Backdrop>
        );
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
                <Box sx={{ overflow: 'auto' }}>
                    <MainListItems />
                </Box>
            </Drawer>
            <Box
                component="main"
                sx={{ flexGrow: 1, p: 3, pb: isTaskRunning ? `${3 * 8 + taskProgressHeight}px` : 3, width: `calc(100% - ${drawerWidth}px)` }}
            >
                <AppRouter />
            </Box>
            <GlobalTaskProgress />
        </Box>
    );
};

export default App;
