import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSnackbar } from 'notistack';
import { Card, CardContent, Typography, TextField, Button, Box, Stack, Grid } from '@mui/material';

const LocalExportPage: React.FC = () => {
  const { enqueueSnackbar } = useSnackbar();
  const [startPage, setStartPage] = useState(1);
  const [endPage, setEndPage] = useState(10);

  const handleExport = async () => {
    if (startPage > endPage) {
      enqueueSnackbar('起始页不能大于结束页', { variant: 'error' });
      return;
    }
    enqueueSnackbar('正在开始导出，请稍候...', { variant: 'info' });
    try {
      await invoke('export_from_local', { range: [startPage, endPage] });
      enqueueSnackbar('本地导出任务已成功启动', { variant: 'success' });
    } catch (e) {
      enqueueSnackbar(`导出失败: ${e}`, { variant: 'error' });
    }
  };

  return (
    <Card sx={{ maxWidth: 500, mx: 'auto', mt: 3 }}>
      <CardContent>
        <Typography variant="h5" component="div" sx={{ mb: 2 }}>
          本地导出
        </Typography>
        <Box component="form" noValidate autoComplete="off">
          <Stack spacing={2}>
            <Grid container spacing={2} alignItems="center">
              <Grid size={{ xs: 5 }}>
                <TextField
                  fullWidth
                  label="起始页"
                  type="number"
                  value={startPage}
                  onChange={(e) => setStartPage(parseInt(e.target.value, 10) || 1)}
                  slotProps={{ htmlInput: { min: 1 } }}
                />
              </Grid>
              <Grid size={{ xs: 2 }} sx={{ textAlign: 'center' }}>-</Grid>
              <Grid size={{ xs: 5 }}>
                <TextField
                  fullWidth
                  label="结束页"
                  type="number"
                  value={endPage}
                  onChange={(e) => setEndPage(parseInt(e.target.value, 10) || 1)}
                  slotProps={{ htmlInput: { min: 1 } }}
                />
              </Grid>
            </Grid>
            <Button variant="contained" onClick={handleExport}>
              开始导出
            </Button>
          </Stack>
        </Box>
      </CardContent>
    </Card>
  );
};

export default LocalExportPage;