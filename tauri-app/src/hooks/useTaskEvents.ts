import { useEffect } from 'react';
import { useSnackbar } from 'notistack';
import { listen } from '@tauri-apps/api/event';
import { useTaskStore } from '../stores/taskStore';
import { getCurrentTaskStatus } from '../lib/api';
import { Task, TaskError } from '../types/tasks';

/**
 * A custom hook that listens for real-time task events from the backend
 * and updates the UI accordingly.
 * 
 * @param isBackendRunning Whether the backend is currently in the 'Running' state.
 *                         Only when true will it attempt to fetch the initial task status.
 */
export function useTaskEvents(isBackendRunning: boolean) {
  const setCurrentTask = useTaskStore((state) => state.setCurrentTask);
  const { enqueueSnackbar } = useSnackbar();

  useEffect(() => {
    let unlistenTask: (() => void) | null = null;
    let unlistenError: (() => void) | null = null;

    const setupListeners = async () => {
      // 1. Listen for task updates (progress, status changes)
      unlistenTask = await listen<Task>('task-updated', (event) => {
        setCurrentTask(event.payload);
      });

      // 2. Listen for task errors (e.g., media download failures)
      unlistenError = await listen<TaskError>('task-error', (event) => {
        const error = event.payload;
        const url = error.error_type.DownloadMedia;
        const displayUrl = url ? (url.length > 50 ? url.substring(0, 47) + '...' : url) : '未知资源';

        enqueueSnackbar(`媒体下载失败: ${displayUrl} - ${error.message}`, {
          variant: 'error',
          persist: true,
        });
      });
    };

    setupListeners();

    return () => {
      if (unlistenTask) unlistenTask();
      if (unlistenError) unlistenError();
    };
  }, [setCurrentTask, enqueueSnackbar]);

  // Sync initial task status when backend becomes running
  useEffect(() => {
    if (isBackendRunning) {
      getCurrentTaskStatus().then((initialTask) => {
        setCurrentTask(initialTask);
      }).catch((error) => {
        console.error('Failed to get initial task status:', error);
      });
    }
  }, [isBackendRunning, setCurrentTask]);
}
