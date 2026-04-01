import React from 'react'
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogContentText,
  DialogActions,
  Button,
} from '@mui/material'

interface CloseConfirmDialogProps {
  open: boolean
  onConfirm: () => void
  onCancel: () => void
}

const CloseConfirmDialog: React.FC<CloseConfirmDialogProps> = ({ open, onConfirm, onCancel }) => {
  return (
    <Dialog open={open} onClose={onCancel} maxWidth="xs" fullWidth>
      <DialogTitle>确认退出</DialogTitle>
      <DialogContent>
        <DialogContentText>程序正在运行中，确定要退出吗？</DialogContentText>
      </DialogContent>
      <DialogActions>
        <Button onClick={onCancel} color="primary">
          取消
        </Button>
        <Button onClick={onConfirm} color="error" autoFocus>
          退出
        </Button>
      </DialogActions>
    </Dialog>
  )
}

export default CloseConfirmDialog
