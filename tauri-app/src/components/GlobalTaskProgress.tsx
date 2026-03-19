import React from 'react'
import { Box, LinearProgress, Typography } from '@mui/material'
import { useTaskStore } from '../stores/taskStore'

const drawerWidth = 200

const GlobalTaskProgress: React.FC = () => {
  const task = useTaskStore(state => state.currentTask)

  if (!task || task.status !== 'InProgress') {
    return null
  }

  const progress = task.total > 0 ? (task.progress / task.total) * 100 : 0

  return (
    <Box
      sx={{
        position: 'fixed',
        bottom: 0,
        left: drawerWidth,
        width: `calc(100% - ${drawerWidth}px)`,
        p: 2,
        bgcolor: 'background.paper',
        zIndex: theme => theme.zIndex.drawer + 1,
        borderTop: '1px solid',
        borderColor: 'divider',
      }}
    >
      <Typography variant="body2" gutterBottom>
        {task.description}
      </Typography>
      <LinearProgress variant="determinate" value={progress} />
      <Typography variant="caption" color="text.secondary">
        {`${task.progress} / ${task.total}`}
      </Typography>
    </Box>
  )
}

export default GlobalTaskProgress
