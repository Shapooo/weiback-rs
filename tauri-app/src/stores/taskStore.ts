import { create } from 'zustand';
import { Task } from '../types';
import { getCurrentTaskStatus } from '../lib/api';

interface TaskState {
  currentTask: Task | null;
  setCurrentTask: (task: Task | null) => void;
  fetchCurrentTask: () => Promise<void>;
}

export const useTaskStore = create<TaskState>((set) => ({
  currentTask: null,
  setCurrentTask: (task) => set({ currentTask: task }),
  fetchCurrentTask: async () => {
    try {
      const task = await getCurrentTaskStatus();
      set({ currentTask: task });
    } catch (error) {
      console.error('Failed to fetch task status:', error);
      set({ currentTask: null });
    }
  },
}));
