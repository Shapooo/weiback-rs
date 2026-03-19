import { useRef, useEffect } from 'react'
import { useSnackbar } from 'notistack'
import { useTaskStore } from '../stores/taskStore'
import { Task } from '../types'

function useCompletionNotifier() {
  const { enqueueSnackbar } = useSnackbar()
  const task = useTaskStore(state => state.currentTask)
  const prevTaskRef = useRef<Task | null>(null)

  useEffect(() => {
    const prevTask = prevTaskRef.current
    if (prevTask && !task) {
      if (prevTask.status === 'Completed') {
        enqueueSnackbar(`任务 "${prevTask.description}" 已完成！`, { variant: 'success' })
      } else if (prevTask.status === 'Failed') {
        enqueueSnackbar(`任务 "${prevTask.description}" 失败: ${prevTask.error || '未知错误'}`, {
          variant: 'error',
          persist: true,
        })
      }
    }
    prevTaskRef.current = task
  }, [task, enqueueSnackbar])
}

export default useCompletionNotifier
