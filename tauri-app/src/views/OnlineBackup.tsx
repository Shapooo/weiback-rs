import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSnackbar } from 'notistack';
import { Card, CardContent, Typography, TextField, Button, Box, Stack, Select, MenuItem, InputLabel, FormControl } from '@mui/material';
import { useTaskStore } from '../stores/taskStore';
import { User } from '../types';
import UserSelector from '../components/UserSelector';


interface LoggedInUser {
    id: number;
    screen_name: string;
}

const UserBackupSection: React.FC = () => {
    const { enqueueSnackbar } = useSnackbar();
    const [userInput, setUserInput] = useState<User | string | null>(null);
    const [userName, setUserName] = useState<string | null>(null);
    const [numPages, setNumPages] = useState(1);
    const [backupType, setBackupType] = useState('Normal');
    const isTaskRunning = useTaskStore(state => !!state.currentTask);


    useEffect(() => {
        const handler = setTimeout(() => {
            if (userInput && typeof userInput === 'string') {
                invoke<string | null>('get_username_by_id', { uid: userInput })
                    .then(setUserName)
                    .catch(console.error);
            } else if (userInput && typeof userInput === 'object') {
                setUserName(userInput.screen_name);
            }
            else {
                setUserName(null);
            }
        }, 500); // 500ms debounce

        return () => {
            clearTimeout(handler);
        };
    }, [userInput]);


    const handleBackup = async () => {
        let backupId: string | null = null;
        if (userInput) {
            if (typeof userInput === 'object') {
                backupId = userInput.id.toString();
            } else {
                backupId = userInput;
            }
        }


        if (!backupId) {
            const loggedInUser: LoggedInUser | null = await invoke('login_state');
            if (loggedInUser && loggedInUser.id) {
                backupId = loggedInUser.id.toString();
            } else {
                enqueueSnackbar('请输入用户ID或选择一个用户', { variant: 'error' });
                return;
            }
        }

        if (numPages <= 0) {
            enqueueSnackbar('备份页数必须为正数', { variant: 'error' });
            return;
        }
        try {
            await invoke('backup_user', { uid: backupId, numPages, backupType });
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
                        <UserSelector
                            value={userInput}
                            onChange={setUserInput}
                            label="用户 (不填写默认为当前登录用户)"
                        />
                        {userName && (
                            <Typography variant="body2" color="text.secondary" sx={{ pl: 1, mt: 0 }}>
                                用户名: {userName}
                            </Typography>
                        )}
                        <FormControl fullWidth>
                            <InputLabel id="backup-type-select-label">备份类型</InputLabel>
                            <Select
                                labelId="backup-type-select-label"
                                id="backup-type-select"
                                value={backupType}
                                label="备份类型"
                                onChange={(e) => setBackupType(e.target.value)}
                            >
                                <MenuItem value={'Normal'}>全部</MenuItem>
                                <MenuItem value={'Original'}>原创</MenuItem>
                                <MenuItem value={'Picture'}>图片</MenuItem>
                                <MenuItem value={'Video'}>视频</MenuItem>
                                <MenuItem value={'Article'}>文章</MenuItem>
                            </Select>
                        </FormControl>
                        <TextField
                            fullWidth
                            label="备份页数"
                            type="number"
                            value={numPages}
                            onChange={(e) => setNumPages(parseInt(e.target.value, 10) || 1)}
                            slotProps={{ htmlInput: { min: 1 } }}
                        />
                        <Button variant="contained" onClick={handleBackup} disabled={isTaskRunning}>
                            {isTaskRunning ? '任务进行中...' : '开始备份'}
                        </Button>
                    </Stack>
                </Box>
            </CardContent>
        </Card>
    );
};

const FavoritesBackupSection: React.FC = () => {
    const { enqueueSnackbar } = useSnackbar();
    const [numPages, setNumPages] = useState(1);
    const isTaskRunning = useTaskStore(state => !!state.currentTask);

    const handleBackup = async () => {
        if (numPages <= 0) {
            enqueueSnackbar('备份页数必须为正数', { variant: 'error' });
            return;
        }
        try {
            await invoke('backup_favorites', { numPages });
            enqueueSnackbar('收藏备份任务已成功启动', { variant: 'success' });
        } catch (e) {
            enqueueSnackbar(`备份失败: ${e}`, { variant: 'error' });
        }
    };

    const handleUnfavorite = async () => {
        try {
            await invoke("unfavorite_posts");
            enqueueSnackbar('开始取消已备份收藏', { variant: 'success' })
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
                        <TextField
                            fullWidth
                            label="备份页数"
                            type="number"
                            value={numPages}
                            onChange={(e) => setNumPages(parseInt(e.target.value, 10) || 1)}
                            slotProps={{ htmlInput: { min: 1 } }}
                        />
                        <Button variant="contained" onClick={handleBackup} disabled={isTaskRunning}>
                            {isTaskRunning ? '任务进行中...' : '开始备份'}
                        </Button>
                        <Button variant="contained" onClick={handleUnfavorite} disabled={isTaskRunning}>
                            {isTaskRunning ? '任务进行中...' : '取消已备份收藏'}
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