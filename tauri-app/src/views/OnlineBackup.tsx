import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSnackbar } from 'notistack';
import { Card, CardContent, Typography, TextField, Button, Box, Stack, Grid } from '@mui/material';

const UserBackupSection: React.FC = () => {
    const { enqueueSnackbar } = useSnackbar();
    const [userId, setUserId] = useState('');
    const [startPage, setStartPage] = useState(1);
    const [endPage, setEndPage] = useState(10);

    const handleBackup = async () => {
        if (!userId) {
            enqueueSnackbar('请输入用户ID', { variant: 'error' });
            return;
        }
        if (startPage > endPage) {
            enqueueSnackbar('起始页不能大于结束页', { variant: 'error' });
            return;
        }
        enqueueSnackbar('正在开始备份，请稍候...', { variant: 'info' });
        try {
            await invoke('backup_user', { uid: userId, range: [startPage, endPage] });
            enqueueSnackbar('用户备份任务已成功启动', { variant: 'success' });
        } catch (e) {
            enqueueSnackbar(`备份失败: ${e}`, { variant: 'error' });
        }
    };

    return (
        <Card sx={{ maxWidth: 500, mx: 'auto', mt: 3 }}>
            <CardContent>
                <Typography variant="h5" component="div" sx={{ mb: 2 }}>
                    用户备份
                </Typography>
                <Box component="form" noValidate autoComplete="off">
                    <Stack spacing={2}>
                        <TextField
                            fullWidth
                            label="用户ID"
                            value={userId}
                            defaultValue={"1234"}
                            onChange={(e) => setUserId(e.target.value)}
                        />
                        <Grid container spacing={2} alignItems="center">
                            <Grid size={{ xs: 5 }} >
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
                        <Button variant="contained" onClick={handleBackup}>
                            开始备份
                        </Button>
                    </Stack>
                </Box>
            </CardContent>
        </Card>
    );
};

const FavoritesBackupSection: React.FC = () => {
    const { enqueueSnackbar } = useSnackbar();
    const [startPage, setStartPage] = useState(1);
    const [endPage, setEndPage] = useState(10);

    const handleBackup = async () => {
        if (startPage > endPage) {
            enqueueSnackbar('起始页不能大于结束页', { variant: 'error' });
            return;
        }
        enqueueSnackbar('正在开始备份，请稍候...', { variant: 'info' });
        try {
            await invoke('backup_favorites', { range: [startPage, endPage] });
            enqueueSnackbar('收藏备份任务已成功启动', { variant: 'success' });
        } catch (e) {
            enqueueSnackbar(`备份失败: ${e}`, { variant: 'error' });
        }
    };

    const handleUnfavorite = async () => {
        try {
            await invoke("unfavorite_posts");
            enqueueSnackbar('开始取消所有收藏', { variant: 'success' })
        } catch (e) {
            enqueueSnackbar(`取消收藏失败：${e}`, { variant: 'error' })
        }
    }

    return (
        <Card sx={{ maxWidth: 500, mx: 'auto', mt: 3 }}>
            <CardContent>
                <Typography variant="h5" component="div" sx={{ mb: 2 }}>
                    收藏备份
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
                            <Grid size={{ xs: 2 }} sx={{ textAlign: 'center' }}>至</Grid>
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
                        <Button variant="contained" onClick={handleBackup}>
                            开始备份
                        </Button>
                        <Button variant="contained" onClick={handleUnfavorite}>
                            取消所有收藏
                        </Button>
                    </Stack>
                </Box>
            </CardContent>
        </Card>
    );
};


const OnlineBackupPage: React.FC = () => {
    return (
        <Box>
            <UserBackupSection />
            <FavoritesBackupSection />
        </Box>
    );
};

export default OnlineBackupPage;