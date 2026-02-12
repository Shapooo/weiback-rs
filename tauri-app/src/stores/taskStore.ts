import { create } from 'zustand';
import { Task } from '../types';

interface TaskState {
  currentTask: Task | null;
  setCurrentTask: (task: Task | null) => void;
}

export const useTaskStore = create<TaskState>((set) => ({
  currentTask: null,
  setCurrentTask: (task) => set({ currentTask: task }),
}));
