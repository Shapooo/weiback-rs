import React, { useRef, useEffect } from 'react';
import { Box, Drawer, CssBaseline, LinearProgress, Typography } from '@mui/material';
import { useSnackbar } from 'notistack';
import { MainListItems } from './listItems';
import AppRouter from './router';
import { useTaskPolling } from './hooks/useTaskPolling';
import { useTaskStore } from './stores/taskStore';
import { useAuthStore } from './stores/authStore';
import { Task } from './types';


const drawerWidth = 200;

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
    // Start global polling for task status
    useTaskPolling();
    // Enable global notifications for task completion/failure
    useCompletionNotifier();

    useEffect(() => {
        // Initialize auth state on app startup
        useAuthStore.getState().checkLoginState();
    }, []);


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
                sx={{ flexGrow: 1, p: 3, width: `calc(100% - ${drawerWidth}px)` }}
            >
                <AppRouter />
            </Box>
            <GlobalTaskProgress />
        </Box>
    );
};

export default App;
