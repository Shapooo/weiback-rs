import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSnackbar } from 'notistack';
import { useTaskStore } from '../stores/taskStore';
import { Task, SubTaskError } from '../types';

/**
 * A custom hook that periodically polls the backend for the current task status
 * and any non-fatal sub-task errors, updating the UI accordingly.
 * @param intervalMs The interval in milliseconds to poll the backend. Defaults to 2000ms.
 */
export function useTaskPolling(intervalMs = 2000) {
  const setCurrentTask = useTaskStore((state) => state.setCurrentTask);
  const { enqueueSnackbar } = useSnackbar();

  useEffect(() => {
    const pollTaskStatus = async () => {
      try {
        const task = await invoke<Task | null>('get_current_task_status');
        setCurrentTask(task);
      } catch (error) {
        console.error('Failed to poll task status:', error);
      }
    };

    const pollSubTaskErrors = async () => {
        try {
            const errors = await invoke<SubTaskError[]>('get_and_clear_sub_task_errors');
            errors.forEach(error => {
                const url = error.error_type.DownloadMedia;
                enqueueSnackbar(`媒体下载失败: ${url} - ${error.message}`, {
                    variant: 'error',
                    persist: true, // Keep it visible until the user dismisses it
                });
            });
        } catch (error) {
            console.error('Failed to poll sub-task errors:', error);
        }
    };

    const runPolls = () => {
        pollTaskStatus();
        pollSubTaskErrors();
    }

    // Poll immediately on mount to get the initial state
    runPolls(); 
    
    const intervalId = setInterval(runPolls, intervalMs);

    // Cleanup function to clear the interval when the component unmounts
    return () => clearInterval(intervalId);
  }, [intervalMs, setCurrentTask, enqueueSnackbar]);
}
