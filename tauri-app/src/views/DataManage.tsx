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
import { cleanupPictures } from '../lib/api';

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
            </Grid>
        </Box>
    );
};

export default DataManage;
