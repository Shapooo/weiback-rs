import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useSnackbar } from 'notistack';
import {
    Box,
    Typography,
    Card,
    CardContent,
    Grid,
    Pagination,
    Accordion,
    AccordionSummary,
    AccordionDetails,
    TextField,
    Button,
    Stack,
    CircularProgress,
    FormControlLabel,
    Checkbox,
} from '@mui/material';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import { LocalizationProvider, DatePicker } from '@mui/x-date-pickers';
import { AdapterDateFns } from '@mui/x-date-pickers/AdapterDateFns';

// --- Type Definitions based on Rust structs ---

interface User {
    id: number;
    screen_name: string;
}

interface Post {
    id: number;
    text: string;
    favorited: boolean;
    created_at: string;
    user: User | null;
}

interface PaginatedPosts {
    posts: Post[];
    total_items: number;
}

interface PostQuery {
    user_id?: number;
    start_date?: number; // Unix timestamp
    end_date?: number;   // Unix timestamp
    is_favorited: boolean;
    reverse_order: boolean;
    page: number;
    posts_per_page: number;
}

interface ExportOutputConfig {
    task_name: string;
    export_dir: string;
}

interface ExportJobOptions {
    query: PostQuery;
    output: ExportOutputConfig;
}

const POSTS_PER_PAGE = 12;

// --- Main Component ---

const LocalExportPage: React.FC = () => {
    const { enqueueSnackbar } = useSnackbar();

    // State for UI controls
    const [filters, setFilters] = useState({
        userId: '',
        startDate: null as Date | null,
        endDate: null as Date | null,
        isFavorited: false,
    });
    // State to hold the filters that are actually applied
    const [appliedFilters, setAppliedFilters] = useState(filters);

    // State for data and pagination
    const [posts, setPosts] = useState<Post[]>([]);
    const [page, setPage] = useState(1);
    const [totalPages, setTotalPages] = useState(0);

    // State for loading indicators
    const [loading, setLoading] = useState(false);
    const [exporting, setExporting] = useState(false);

    const fetchPosts = useCallback(async (currentPage: number, currentFilters: typeof filters) => {
        setLoading(true);
        try {
            const query: PostQuery = {
                page: currentPage,
                posts_per_page: POSTS_PER_PAGE,
                is_favorited: currentFilters.isFavorited,
                reverse_order: false, // Show newest first
                user_id: currentFilters.userId ? parseInt(currentFilters.userId, 10) : undefined,
                start_date: currentFilters.startDate ? Math.floor(currentFilters.startDate.getTime() / 1000) : undefined,
                end_date: currentFilters.endDate ? Math.floor(currentFilters.endDate.getTime() / 1000) : undefined,
            };

            const result: PaginatedPosts = await invoke('query_local_posts', { query });
            setPosts(result.posts);
            setTotalPages(Math.ceil(result.total_items / POSTS_PER_PAGE));
        } catch (e) {
            enqueueSnackbar(`查询帖子失败: ${e}`, { variant: 'error' });
            setPosts([]);
            setTotalPages(0);
        } finally {
            setLoading(false);
        }
    }, [enqueueSnackbar]);

    // Fetch posts when page or applied filters change
    useEffect(() => {
        fetchPosts(page, appliedFilters);
    }, [page, appliedFilters, fetchPosts]);

    const handleSearch = () => {
        setPage(1); // Reset to first page on new search
        setAppliedFilters(filters);
    };

    const handleClearFilters = () => {
        const clearedFilters = {
            userId: '',
            startDate: null,
            endDate: null,
            isFavorited: false,
        };
        setFilters(clearedFilters);
        if (JSON.stringify(appliedFilters) !== JSON.stringify(clearedFilters)) {
            setPage(1);
            setAppliedFilters(clearedFilters);
        }
    };

    const handlePageChange = (_event: React.ChangeEvent<unknown>, value: number) => {
        setPage(value);
    };

    const handleExport = async () => {
        const selectedPath = await open({
            directory: true,
            multiple: false,
            title: '选择导出目录',
        });

        if (typeof selectedPath !== 'string' || !selectedPath) {
            enqueueSnackbar('已取消导出', { variant: 'info' });
            return;
        }

        setExporting(true);
        enqueueSnackbar('正在准备导出...', { variant: 'info' });

        try {
            const query: PostQuery = {
                page: 1, // Export should start from page 1
                posts_per_page: 1_000_000, // A large number to signify "all"
                is_favorited: appliedFilters.isFavorited,
                reverse_order: true,
                user_id: appliedFilters.userId ? parseInt(appliedFilters.userId, 10) : undefined,
                start_date: appliedFilters.startDate ? Math.floor(appliedFilters.startDate.getTime() / 1000) : undefined,
                end_date: appliedFilters.endDate ? Math.floor(appliedFilters.endDate.getTime() / 1000) : undefined,
            };

            const options: ExportJobOptions = {
                query,
                output: {
                    task_name: `weiback_export_${Date.now()}`,
                    export_dir: selectedPath,
                }
            };

            await invoke('export_posts', { options });
            enqueueSnackbar('导出任务已成功启动', { variant: 'success' });
        } catch (e) {
            enqueueSnackbar(`导出失败: ${e}`, { variant: 'error' });
        } finally {
            setExporting(false);
        }
    };

    return (
        <LocalizationProvider dateAdapter={AdapterDateFns}>
            <Box sx={{ width: '100%', p: 3 }}>
                <Typography variant="h4" gutterBottom>
                    本地导出与浏览
                </Typography>

                <Accordion defaultExpanded>
                    <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                        <Typography variant="h6">筛选条件</Typography>
                    </AccordionSummary>
                    <AccordionDetails>
                        <Stack spacing={2}>
                            <Grid container spacing={2}>
                                <Grid size={{ xs: 12, sm: 6, md: 3 }}>
                                    <DatePicker
                                        label="起始日期"
                                        value={filters.startDate}
                                        onChange={(date) => setFilters(f => ({ ...f, startDate: date }))}
                                    />
                                </Grid>
                                <Grid size={{ xs: 12, sm: 6, md: 3 }}>
                                    <DatePicker
                                        label="结束日期"
                                        value={filters.endDate}
                                        onChange={(date) => setFilters(f => ({ ...f, endDate: date }))}
                                    />
                                </Grid>
                                <Grid size={{ xs: 12, sm: 6, md: 3 }}>
                                    <TextField
                                        fullWidth
                                        label="用户ID"
                                        value={filters.userId}
                                        onChange={(e) => setFilters(f => ({ ...f, userId: e.target.value }))}
                                        type="number"
                                    />
                                </Grid>
                                <Grid size={{ xs: 12, sm: 6, md: 3 }} sx={{ display: 'flex', alignItems: 'center' }}>
                                    <FormControlLabel
                                        control={
                                            <Checkbox
                                                checked={filters.isFavorited}
                                                onChange={(e) => setFilters(f => ({ ...f, isFavorited: e.target.checked }))}
                                            />
                                        }
                                        label="只看收藏"
                                    />
                                </Grid>
                            </Grid>
                            <Stack direction="row" spacing={2} justifyContent="space-between">
                                <Stack direction="row" spacing={2}>
                                    <Button variant="contained" onClick={handleSearch}>查询</Button>
                                    <Button variant="outlined" onClick={handleClearFilters}>清空筛选</Button>
                                </Stack>
                                <Button
                                    variant="contained"
                                    color="secondary"
                                    onClick={handleExport}
                                    disabled={exporting}
                                    startIcon={exporting ? <CircularProgress size={20} /> : null}
                                >
                                    导出筛选结果
                                </Button>
                            </Stack>
                        </Stack>
                    </AccordionDetails>
                </Accordion>

                <Box sx={{ mt: 3 }}>
                    {loading ? (
                        <Box sx={{ display: 'flex', justifyContent: 'center', p: 5 }}>
                            <CircularProgress />
                        </Box>
                    ) : (
                        <>
                            <Grid container spacing={3}>
                                {posts.length > 0 ? posts.map(post => (
                                    <Grid size={{ xs: 12, sm: 6, md: 4 }} key={post.id}>
                                        <Card>
                                            <CardContent>
                                                <Typography variant="subtitle2" color="text.secondary">
                                                    {post.user?.screen_name || '未知用户'}
                                                </Typography>
                                                <Typography variant="body2" sx={{ mt: 1 }}>
                                                    {post.text}
                                                </Typography>
                                            </CardContent>
                                        </Card>
                                    </Grid>
                                )) : (
                                    <Grid size={{ xs: 12 }}>
                                        <Typography sx={{ textAlign: 'center', p: 5 }}>没有找到符合条件的帖子。</Typography>
                                    </Grid>
                                )}
                            </Grid>
                            {totalPages > 0 && (
                                <Box sx={{ display: 'flex', justifyContent: 'center', mt: 3 }}>
                                    <Pagination
                                        count={totalPages}
                                        page={page}
                                        onChange={handlePageChange}
                                        color="primary"
                                    />
                                </Box>
                            )}
                        </>
                    )}
                </Box>
            </Box>
        </LocalizationProvider>
    );
};

export default LocalExportPage;
