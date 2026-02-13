import React, { useEffect, useState } from 'react';
import { useThemeContext } from '../ThemeContext';
import {
    Card, CardContent, Typography, FormControlLabel, Switch, Box, TextField,
    Select, MenuItem, InputLabel, FormControl, InputAdornment, Accordion,
    AccordionSummary, AccordionDetails
} from '@mui/material';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import Grid from '@mui/material/Grid';
import { useTheme } from '@mui/material/styles';
import { open } from '@tauri-apps/plugin-dialog';
import { Button } from "@mui/material";
import { SdkConfig, PictureDefinition, Config } from '../types/config';
import { getConfig, setConfig } from '../lib/api';

const pictureDefinitionMap = [
    { value: PictureDefinition.Largest, label: '原始尺寸' },
    { value: PictureDefinition.Mw2000, label: '超高清' },
    { value: PictureDefinition.Original, label: '高清' },
    { value: PictureDefinition.Large, label: '中等' },
    { value: PictureDefinition.Bmiddle, label: '标清' },
    { value: PictureDefinition.Thumbnail, label: '缩略图' },
];

const SettingsPage: React.FC = () => {
    const { toggleColorMode } = useThemeContext();
    const theme = useTheme();
    const [config, setConfigState] = useState<Config | null>(null);
    const [initialConfig, setInitialConfig] = useState<Config | null>(null);

    useEffect(() => {
        getConfig()
            .then(loadedConfig => {
                setConfigState(loadedConfig);
                setInitialConfig(loadedConfig);
            })
            .catch(console.error);
    }, []);

    const handleSave = () => {
        if (config) {
            setConfig(config)
                .then(() => {
                    console.log('Settings saved');
                    setInitialConfig(config);
                })
                .catch(console.error);
        }
    };

    const handleReset = () => {
        setConfigState(initialConfig);
    }

    const handleSelectPath = async (field: 'picture_path' | 'video_path') => {
        const selected = await open({
            directory: true,
            multiple: false,
            title: `选择${field === 'picture_path' ? '图片' : '视频'}保存路径`,
        });
        if (typeof selected === 'string' && config) {
            handleChange(field, selected);
        }
    };

    const handleChange = (field: keyof Config, value: any) => {
        if (config) {
            setConfigState({ ...config, [field]: value });
        }
    };

    const handleSdkChange = (field: keyof SdkConfig, value: any) => {
        if (config) {
            setConfigState({ ...config, sdk_config: { ...config.sdk_config, [field]: value } });
        }
    };

    if (!config) {
        return <Typography>Loading settings...</Typography>;
    }

    const isChanged = JSON.stringify(config) !== JSON.stringify(initialConfig);

    return (
        <Card sx={{ maxWidth: 800, mx: 'auto', mt: 3 }}>
            <CardContent>
                <Typography variant="h5" component="div" sx={{ mb: 2 }}>
                    设置
                </Typography>
                <Box component="form" noValidate autoComplete="off" sx={{ '& .MuiTextField-root': { my: 1 }, '& .MuiFormControl-root': { my: 1 } }}>
                    <Grid container spacing={2}>
                        <Grid size={{ xs: 12, sm: 6 }} >
                            <FormControlLabel
                                control={<Switch checked={theme.palette.mode === 'dark'} onChange={toggleColorMode} />}
                                label="暗色模式"
                            />
                        </Grid>
                        <Grid size={{ xs: 12, sm: 6 }} >
                            <FormControlLabel
                                control={<Switch checked={config.download_pictures} onChange={(e) => handleChange('download_pictures', e.target.checked)} />}
                                label="下载图片"
                            />
                        </Grid>
                        <Grid size={{ xs: 12, sm: 6 }}>
                            <FormControl fullWidth>
                                <InputLabel id="pic-def-label">图片清晰度</InputLabel>
                                <Select
                                    labelId="pic-def-label"
                                    value={config.picture_definition}
                                    label="图片清晰度"
                                    onChange={(e) => handleChange('picture_definition', e.target.value)}
                                    renderValue={(selectedValue) =>
                                        pictureDefinitionMap.find(item => item.value === selectedValue)?.label ?? selectedValue
                                    }
                                >
                                    {pictureDefinitionMap.map((item) => (
                                        <MenuItem key={item.value} value={item.value}>{item.label}</MenuItem>
                                    ))}
                                </Select>
                            </FormControl>
                        </Grid>
                        <Grid size={{ xs: 12, sm: 6 }}>
                            <TextField fullWidth
                                label="每个 HTML 文件包含的微博数"
                                type="number"
                                value={config.posts_per_html}
                                onChange={(e) => handleChange('posts_per_html', parseInt(e.target.value, 10))}
                            />
                        </Grid>
                        <Grid size={{ xs: 12 }}>
                            <Accordion>
                                <AccordionSummary
                                    expandIcon={<ExpandMoreIcon />}
                                    aria-controls="advanced-settings-content"
                                    id="advanced-settings-header"
                                >
                                    <Box>
                                        <Typography variant="h6">高级设置</Typography>
                                        <Typography variant="body2" color="text.secondary">
                                            修改前确定你知道你在做什么，不当的设置可能会导致程序异常。
                                        </Typography>
                                    </Box>
                                </AccordionSummary>
                                <AccordionDetails>
                                    <Grid container spacing={2}>
                                        {/* Task Intervals */}
                                        <Grid size={{ xs: 12 }}>
                                            <Typography variant="h6" sx={{ mt: 2 }}>任务间隔 (秒)</Typography>
                                        </Grid>
                                        <Grid size={{ xs: 12, sm: 6 }}>
                                            <TextField fullWidth
                                                label="备份任务间隔"
                                                type="number"
                                                value={config.backup_task_interval}
                                                onChange={(e) => handleChange('backup_task_interval', parseInt(e.target.value, 10))}
                                            />
                                        </Grid>
                                        <Grid size={{ xs: 12, sm: 6 }}>
                                            <TextField fullWidth
                                                label="其他任务间隔"
                                                type="number"
                                                value={config.other_task_interval}
                                                onChange={(e) => handleChange('other_task_interval', parseInt(e.target.value, 10))}
                                            />
                                        </Grid>

                                        {/* SDK Config */}
                                        <Grid size={{ xs: 12 }}>
                                            <Typography variant="h6" sx={{ mt: 2 }}>SDK 配置</Typography>
                                        </Grid>
                                        <Grid size={{ xs: 12, sm: 4 }}>
                                            <TextField fullWidth
                                                label="收藏接口单次返回数量"
                                                type="number"
                                                value={config.sdk_config.fav_count}
                                                onChange={(e) => handleSdkChange('fav_count', parseInt(e.target.value, 10))}
                                            />
                                        </Grid>
                                        <Grid size={{ xs: 12, sm: 4 }}>
                                            <TextField fullWidth
                                                label="用户微博接口单次返回数量"
                                                type="number"
                                                value={config.sdk_config.status_count}
                                                onChange={(e) => handleSdkChange('status_count', parseInt(e.target.value, 10))}
                                            />
                                        </Grid>
                                        <Grid size={{ xs: 12, sm: 4 }}>
                                            <TextField fullWidth
                                                label="接口重试次数"
                                                type="number"
                                                value={config.sdk_config.retry_times}
                                                onChange={(e) => handleSdkChange('retry_times', parseInt(e.target.value, 10))}
                                            />
                                        </Grid>

                                        {/* Paths */}
                                        <Grid size={{ xs: 12 }}>
                                            <Typography variant="h6" sx={{ mt: 2 }}>路径</Typography>
                                        </Grid>
                                        <Grid size={{ xs: 12 }}>
                                            <TextField fullWidth label="图片保存路径" value={config.picture_path}
                                                InputProps={{
                                                    readOnly: true,
                                                    endAdornment: (
                                                        <InputAdornment position="end">
                                                            <Button onClick={() => handleSelectPath('picture_path')}>
                                                                选择
                                                            </Button>
                                                        </InputAdornment>
                                                    ),
                                                }}
                                            />
                                        </Grid>
                                        <Grid size={{ xs: 12 }}>
                                            <TextField fullWidth label="视频保存路径" value={config.video_path}
                                                InputProps={{
                                                    readOnly: true,
                                                    endAdornment: (
                                                        <InputAdornment position="end">
                                                            <Button onClick={() => handleSelectPath('video_path')}>
                                                                选择
                                                            </Button>
                                                        </InputAdornment>
                                                    ),
                                                }} />
                                        </Grid>
                                        {config.dev_mode_out_dir &&
                                            <Grid size={{ xs: 12 }}>
                                                <TextField fullWidth label="开发者模式输出路径" value={config.dev_mode_out_dir} slotProps={{ htmlInput: { readOnly: true } }} />
                                            </Grid>
                                        }
                                    </Grid>
                                </AccordionDetails>
                            </Accordion>
                        </Grid>
                    </Grid>
                    <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 3 }}>
                        <Button variant="outlined" onClick={handleReset} disabled={!isChanged} sx={{ mr: 1 }}>
                            重置
                        </Button>
                        <Button variant="contained" onClick={handleSave} disabled={!isChanged}>
                            保存
                        </Button>
                    </Box>
                </Box>
            </CardContent>
        </Card>
    );
};

export default SettingsPage;