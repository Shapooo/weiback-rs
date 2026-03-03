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
    Checkbox,
} from '@mui/material';
import { useSnackbar } from 'notistack';
import { useTaskStore } from '../stores/taskStore';
import { TaskStatus, ResolutionPolicy } from '../types';
import { cleanupPictures, cleanupInvalidAvatars, cleanupInvalidPosts } from '../lib/api';

const DataManage: React.FC = () => {
    const { enqueueSnackbar } = useSnackbar();
    const isTaskRunning = useTaskStore(state => state.currentTask?.status === TaskStatus.InProgress);
    const fetchCurrentTask = useTaskStore(state => state.fetchCurrentTask);

    const [policy, setPolicy] = useState<ResolutionPolicy>(ResolutionPolicy.Highest);
    const [cleanRetweetedInvalid, setCleanRetweetedInvalid] = useState(false);

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

    const handleCleanupInvalidPosts = async () => {
        try {
            await cleanupInvalidPosts({ clean_retweeted_invalid: cleanRetweetedInvalid });
            enqueueSnackbar('失效内容清理任务已启动', { variant: 'success' });
            fetchCurrentTask();
        } catch (e) {
            enqueueSnackbar(`启动失效内容清理失败: ${e}`, { variant: 'error' });
        }
    };

    return (
        <Box sx={{ p: 3 }}>
            <Typography variant="h4" gutterBottom>
                全局数据维护
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

                <Grid size={{ xs: 12, md: 6 }}>
                    <Card>
                        <CardContent>
                            <Typography variant="h6" gutterBottom>
                                失效微博清理
                            </Typography>
                            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                                清理数据库中的失效微博。这些内容通常由于原作者注销或删除或者不可抗力而无法正常显示。
                            </Typography>

                            <Alert severity="warning" sx={{ mb: 2 }}>
                                此操作将永久删除失效微博及其关联媒体。
                            </Alert>

                            <Box sx={{ mb: 2 }}>
                                <FormControlLabel
                                    control={
                                        <Checkbox
                                            checked={cleanRetweetedInvalid}
                                            onChange={(e) => setCleanRetweetedInvalid(e.target.checked)}
                                        />
                                    }
                                    label={<strong>深度清理模式</strong>}
                                />
                                <Typography variant="caption" display="block" color="text.secondary" sx={{ ml: 4 }}>
                                    {cleanRetweetedInvalid ? (
                                        <span>
                                            <strong>当前模式：</strong> 清理所有失效内容。同时，如果某条正常微博转发的内容已失效，该正常微博也将被一并删除（保持数据库绝对纯净）。
                                        </span>
                                    ) : (
                                        <span>
                                            <strong>当前模式（默认）：</strong> 仅清理“独立”的失效内容。如果某条失效微博被你保存的其他微博转发了，为了保持转发链条的完整性，将予以保留。
                                        </span>
                                    )}
                                </Typography>
                            </Box>

                            <Box sx={{ mt: 3 }}>
                                <Button
                                    variant="contained"
                                    color="primary"
                                    fullWidth
                                    onClick={handleCleanupInvalidPosts}
                                    disabled={isTaskRunning}
                                >
                                    {isTaskRunning ? '任务进行中...' : '开始清理失效内容'}
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
