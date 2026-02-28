import React, { useState } from 'react';
import {
    Box,
    Typography,
    Card,
    CardContent,
    FormControl,
    FormLabel,
    RadioGroup,
    FormControlLabel,
    Radio,
    Button,
    Alert,
    Grid,
} from '@mui/material';
import { useSnackbar } from 'notistack';
import { useTaskStore } from '../stores/taskStore';
import { TaskStatus, ResolutionPolicy } from '../types';
import { cleanupPictures, cleanupInvalidAvatars } from '../lib/api';

const DataManage: React.FC = () => {
    const { enqueueSnackbar } = useSnackbar();
    const isTaskRunning = useTaskStore(state => state.currentTask?.status === TaskStatus.InProgress);
    const fetchCurrentTask = useTaskStore(state => state.fetchCurrentTask);

    const [policy, setPolicy] = useState<ResolutionPolicy>(ResolutionPolicy.Highest);

    const handleCleanup = async () => {
        try {
            await cleanupPictures(policy);
            enqueueSnackbar('图片清理任务已启动', { variant: 'success' });
            fetchCurrentTask();
        } catch (e) {
            enqueueSnackbar(`启动清理任务失败: ${e}`, { variant: 'error' });
        }
    };

    const handleCleanupAvatars = async () => {
        try {
            await cleanupInvalidAvatars();
            enqueueSnackbar('失效头像清理任务已启动', { variant: 'success' });
            fetchCurrentTask();
        } catch (e) {
            enqueueSnackbar(`启动头像清理失败: ${e}`, { variant: 'error' });
        }
    };

    return (
        <Box sx={{ p: 3 }}>
            <Typography variant="h4" gutterBottom>
                数据管理
            </Typography>

            <Grid container spacing={3}>
                <Grid size={{ xs: 12, md: 6 }}>
                    <Card>
                        <CardContent>
                            <Typography variant="h6" gutterBottom>
                                图片清理 (清晰度去重)
                            </Typography>
                            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                                如果同一张微博图片存在多种清晰度（如缩略图和原图），此操作将根据您的选择保留其中一个，并删除多余的文件及数据库记录。
                            </Typography>

                            <Alert severity="warning" sx={{ mb: 2 }}>
                                此操作不可逆，请在执行前确认备份重要数据。
                            </Alert>

                            <FormControl component="fieldset">
                                <FormLabel component="legend">保留策略</FormLabel>
                                <RadioGroup
                                    value={policy}
                                    onChange={(e) => setPolicy(e.target.value as ResolutionPolicy)}
                                >
                                    <FormControlLabel
                                        value={ResolutionPolicy.Highest}
                                        control={<Radio />}
                                        label="保留最高清晰度 (推荐)"
                                    />
                                    <FormControlLabel
                                        value={ResolutionPolicy.Lowest}
                                        control={<Radio />}
                                        label="保留最低清晰度 (节省空间)"
                                    />
                                </RadioGroup>
                            </FormControl>

                            <Box sx={{ mt: 3 }}>
                                <Button
                                    variant="contained"
                                    color="primary"
                                    fullWidth
                                    onClick={handleCleanup}
                                    disabled={isTaskRunning}
                                >
                                    {isTaskRunning ? '任务进行中...' : '开始清理'}
                                </Button>
                            </Box>
                        </CardContent>
                    </Card>
                </Grid>

                <Grid size={{ xs: 12, md: 6 }}>
                    <Card>
                        <CardContent>
                            <Typography variant="h6" gutterBottom>
                                头像清理 (失效去重)
                            </Typography>
                            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                                微博用户更换头像后，本地可能仍保留着旧的头像文件。此操作将对比数据库中记录的最新头像，清理所有已失效的历史头像文件。
                            </Typography>

                            <Alert severity="info" sx={{ mb: 2 }}>
                                仅清理 user 表中已记录的用户的历史头像。
                            </Alert>

                            <Box sx={{ mt: 3 }}>
                                <Button
                                    variant="contained"
                                    color="primary"
                                    fullWidth
                                    onClick={handleCleanupAvatars}
                                    disabled={isTaskRunning}
                                >
                                    {isTaskRunning ? '任务进行中...' : '开始清理失效头像'}
                                </Button>
                            </Box>
                        </CardContent>
                    </Card>
                </Grid>
            </Grid>
        </Box>
    );
};

export default DataManage;
