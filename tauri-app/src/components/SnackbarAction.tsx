import React from 'react'
import { useSnackbar, SnackbarKey } from 'notistack'
import IconButton from '@mui/material/IconButton'
import CloseIcon from '@mui/icons-material/Close'

export const SnackbarAction = ({ id }: { id: SnackbarKey }) => {
  const { closeSnackbar } = useSnackbar()
  return (
    <IconButton size="small" aria-label="close" color="inherit" onClick={() => closeSnackbar(id)}>
      <CloseIcon fontSize="small" />
    </IconButton>
  )
}
