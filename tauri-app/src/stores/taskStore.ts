import { create } from 'zustand'
import { Task, DownloaderStatus } from '../types'
import { getCurrentTaskStatus } from '../lib/api'

interface TaskState {
  currentTask: Task | null
  setCurrentTask: (task: Task | null) => void
  fetchCurrentTask: () => Promise<void>
}

interface DownloaderState {
  downloaderStatus: DownloaderStatus
  setDownloaderStatus: (status: DownloaderStatus) => void
}

export const useTaskStore = create<TaskState & DownloaderState>(set => ({
  currentTask: null,
  setCurrentTask: task => set({ currentTask: task }),
  fetchCurrentTask: async () => {
    try {
      const task = await getCurrentTaskStatus()
      set({ currentTask: task })
    } catch (error) {
      console.error('Failed to fetch task status:', error)
      set({ currentTask: null })
    }
  },
  downloaderStatus: { current_url: null, queue_length: 0, is_processing: false },
  setDownloaderStatus: status => set({ downloaderStatus: status }),
}))
