import { useEffect } from 'react';
import { useSnackbar } from 'notistack';
import { listen } from '@tauri-apps/api/event';
import { useTaskStore } from '../stores/taskStore';
import { getCurrentTaskStatus } from '../lib/api';
import { Task, TaskError } from '../types/tasks';

/**
 * A custom hook that listens for real-time task events from the backend
 * and updates the UI accordingly.
 */
export function useTaskEvents() {
  const setCurrentTask = useTaskStore((state) => state.setCurrentTask);
  const { enqueueSnackbar } = useSnackbar();

  useEffect(() => {
    let unlistenTask: (() => void) | null = null;
    let unlistenError: (() => void) | null = null;

    let unlistenConfigError: (() => void) | null = null;

    const setupListeners = async () => {
      // 1. Get initial state to ensure UI is synced on mount/refresh
      try {
        const initialTask = await getCurrentTaskStatus();
        setCurrentTask(initialTask);
      } catch (error) {
        console.error('Failed to get initial task status:', error);
      }

      // 2. Listen for task updates (progress, status changes)
      unlistenTask = await listen<Task>('task-updated', (event) => {
        setCurrentTask(event.payload);
      });

      // 3. Listen for task errors (e.g., media download failures)
      unlistenError = await listen<TaskError>('task-error', (event) => {
        const error = event.payload;
        const url = error.error_type.DownloadMedia;
        const displayUrl = url ? (url.length > 50 ? url.substring(0, 47) + '...' : url) : '未知资源';

        enqueueSnackbar(`媒体下载失败: ${displayUrl} - ${error.message}`, {
          variant: 'error',
          persist: true,
        });
      });

      // 4. Listen for configuration errors
      unlistenConfigError = await listen<string>('config-error', (event) => {
        enqueueSnackbar(`配置文件加载失败，已使用默认配置。错误详情: ${event.payload}`, {
          variant: 'warning',
          persist: true,
        });
      });
    };

    setupListeners();

    return () => {
      if (unlistenTask) unlistenTask();
      if (unlistenError) unlistenError();
      if (unlistenConfigError) unlistenConfigError();
    };
  }, [setCurrentTask, enqueueSnackbar]);
}
