import React, { useState, useEffect, useCallback } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useSnackbar } from 'notistack';
import {
    Box,
    Typography,
    Grid,
    Pagination,
    Accordion,
    AccordionSummary,
    AccordionDetails,
    Button,
    Stack,
    CircularProgress,
    FormControlLabel,
    Checkbox,
    Modal,
    TextField,
    InputAdornment,
    Select,
    MenuItem,
    Divider,
} from '@mui/material';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import { LocalizationProvider, DatePicker } from '@mui/x-date-pickers';
import { AdapterDateFns } from '@mui/x-date-pickers/AdapterDateFns';
import FullSizeImage from '../components/FullSizeImage';
import PostDisplay from '../components/PostDisplay';
import { PostInfo, User, PostQuery, ExportJobOptions, TaskStatus } from '../types';
import PostPreviewModal from '../components/PostPreviewModal';
import { useTaskStore } from '../stores/taskStore';
import UserSelector from '../components/UserSelector';
import { queryLocalPosts, exportPosts, rebackupPosts } from '../lib/api';

const POSTS_PER_PAGE = 12;

// --- Main Component ---

const ContentExplorerPage: React.FC = () => {
    const { enqueueSnackbar } = useSnackbar();
    const isTaskRunning = useTaskStore(state => state.currentTask?.status === TaskStatus.InProgress);
    const fetchCurrentTask = useTaskStore(state => state.fetchCurrentTask);

    // State for UI controls
    const [userInput, setUserInput] = useState<User | string | null>(null);
    const [filters, setFilters] = useState({
        startDate: null as Date | null,
        endDate: null as Date | null,
        isFavorited: false,
        reverseOrder: false,
        searchTerm: '',
        searchMode: 'fuzzy' as 'fuzzy' | 'strict',
    });
    // State to hold the filters that are actually applied
    const [appliedFilters, setAppliedFilters] = useState({ ...filters, userInput });

    // State for data and pagination
    const [postInfos, setPostInfos] = useState<PostInfo[]>([]);
    const [page, setPage] = useState(1);
    const [totalPages, setTotalPages] = useState(0);
    const [jumpPage, setJumpPage] = useState('');
    const [lightboxImageId, setLightboxImageId] = useState<string | null>(null);
    const [hoveredPostInfo, setHoveredPostInfo] = useState<PostInfo | null>(null);
    const [showPreviewModal, setShowPreviewModal] = useState(false);


    // State for loading indicators
    const [loading, setLoading] = useState(false);

    const handleJump = () => {
        const pageNum = parseInt(jumpPage, 10);
        if (!isNaN(pageNum) && pageNum >= 1 && pageNum <= totalPages) {
            setPage(pageNum);
            setJumpPage('');
        } else {
            enqueueSnackbar(`请输入 1 到 ${totalPages} 之间的有效页码`, { variant: 'warning' });
        }
    };

    const handleOpenLightbox = (imageId: string) => {
        setLightboxImageId(imageId);
    };

    const handleCloseLightbox = () => {
        setLightboxImageId(null);
    };

    const handlePostClick = useCallback((postInfo: PostInfo) => {
        setHoveredPostInfo(postInfo);
        setShowPreviewModal(true);
    }, []);

    const handleClosePreviewModal = useCallback(() => {
        setShowPreviewModal(false);
        setHoveredPostInfo(null);
    }, []);

    const getUserId = (input: User | string | null): number | undefined => {
        if (!input) return undefined;
        if (typeof input === 'object' && input.id) {
            return input.id;
        }
        if (typeof input === 'string' && /^\d+$/.test(input)) {
            return parseInt(input, 10);
        }
        return undefined;
    }

    const buildQueryFromFilters = (
        currentFilters: typeof appliedFilters,
        currentPage: number,
        isBatchOperation: boolean,
    ): PostQuery => {
        const startDate = currentFilters.startDate ? new Date(currentFilters.startDate) : null;
        if (startDate) {
            startDate.setHours(0, 0, 0, 0);
        }

        const endDate = currentFilters.endDate ? new Date(currentFilters.endDate) : null;
        if (endDate) {
            endDate.setHours(23, 59, 59, 999);
        }

        const userId = getUserId(currentFilters.userInput);

        return {
            page: isBatchOperation ? 1 : currentPage,
            posts_per_page: isBatchOperation ? 1_000_000 : POSTS_PER_PAGE,
            is_favorited: currentFilters.isFavorited,
            reverse_order: currentFilters.reverseOrder,
            user_id: userId,
            start_date: startDate ? Math.floor(startDate.getTime() / 1000) : undefined,
            end_date: endDate ? Math.floor(endDate.getTime() / 1000) : undefined,
            search_term: currentFilters.searchTerm
                ? (currentFilters.searchMode === 'fuzzy'
                    ? { Fuzzy: currentFilters.searchTerm }
                    : { Strict: currentFilters.searchTerm })
                : undefined,
        };
    };

    const fetchPosts = useCallback(async (currentPage: number, currentFilters: typeof appliedFilters) => {
        setLoading(true);
        try {
            const query = buildQueryFromFilters(currentFilters, currentPage, false);

            const result = await queryLocalPosts(query);
            setPostInfos(result.posts);
            setTotalPages(Math.ceil(result.total_items / POSTS_PER_PAGE));
        } catch (e) {
            enqueueSnackbar(`查询帖子失败: ${e}`, { variant: 'error' });
            setPostInfos([]);
            setTotalPages(0);
        } finally {
            setLoading(false);
        }
    }, [enqueueSnackbar]);

    const handlePostDeleted = useCallback(() => {
        fetchPosts(page, appliedFilters);
    }, [fetchPosts, page, appliedFilters]);

    // Fetch posts when page or applied filters change
    useEffect(() => {
        fetchPosts(page, appliedFilters);
    }, [page, appliedFilters, fetchPosts]);

    const handleSearch = () => {
        setPage(1); // Reset to first page on new search
        setAppliedFilters({ ...filters, userInput });
    };

    const handleClearFilters = () => {
        const clearedFilters = {
            startDate: null,
            endDate: null,
            isFavorited: false,
            reverseOrder: false,
            searchTerm: '',
            searchMode: 'fuzzy' as 'fuzzy' | 'strict',
        };
        const clearedUserInput = null;

        setUserInput(clearedUserInput);
        setFilters(clearedFilters);

        if (JSON.stringify(appliedFilters) !== JSON.stringify({ ...clearedFilters, userInput: clearedUserInput })) {
            setPage(1);
            setAppliedFilters({ ...clearedFilters, userInput: clearedUserInput });
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

        try {
            const query = buildQueryFromFilters(appliedFilters, page, true);
            const options: ExportJobOptions = {
                query,
                output: {
                    task_name: `weiback_export_${Date.now()}`,
                    export_dir: selectedPath,
                }
            };

            await exportPosts(options);
            enqueueSnackbar('导出任务已成功启动', { variant: 'success' });
            fetchCurrentTask();
        } catch (e) {
            enqueueSnackbar(`启动导出任务失败: ${e}`, { variant: 'error' });
        }
    };

    const handleRebackup = async () => {
        try {
            const query = buildQueryFromFilters(appliedFilters, page, true);
            await rebackupPosts(query);
            enqueueSnackbar('批量重新备份任务已成功启动', { variant: 'success' });
            fetchCurrentTask();
        } catch (e) {
            enqueueSnackbar(`启动批量重新备份任务失败: ${e}`, { variant: 'error' });
        }
    };

    return (
        <LocalizationProvider dateAdapter={AdapterDateFns}>
            <Box sx={{ width: '100%', p: 3 }}>
                <Typography variant="h4" gutterBottom>
                    内容浏览与处理
                </Typography>

                <Accordion defaultExpanded>
                    <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                        <Typography variant="h6">查询与批量处理</Typography>
                    </AccordionSummary>
                    <AccordionDetails>
                        <Stack spacing={3}>
                            {/* --- Filter Section --- */}
                            <Box>
                                <Typography variant="subtitle1" gutterBottom sx={{ fontWeight: 'bold' }}>
                                    筛选
                                </Typography>
                                <Grid container spacing={2} alignItems="center">
                                    <Grid size={{ xs: 12, md: 6 }}>
                                        <UserSelector
                                            value={userInput}
                                            onChange={setUserInput}
                                        />
                                    </Grid>
                                    <Grid size={{ xs: 12, md: 6 }}>
                                        <TextField
                                            fullWidth
                                            label="搜索正文"
                                            value={filters.searchTerm}
                                            onChange={(e) => setFilters(f => ({ ...f, searchTerm: e.target.value }))}
                                            onKeyDown={(e) => {
                                                if (e.key === 'Enter') {
                                                    handleSearch();
                                                }
                                            }}
                                            InputProps={{
                                                startAdornment: (
                                                    <InputAdornment position="start">
                                                        <Select
                                                            variant="standard"
                                                            value={filters.searchMode}
                                                            onChange={(e) => setFilters(f => ({ ...f, searchMode: e.target.value as 'fuzzy' | 'strict' }))}
                                                            disableUnderline
                                                            sx={{ fontSize: '0.875rem' }}
                                                        >
                                                            <MenuItem value="fuzzy">模糊</MenuItem>
                                                            <MenuItem value="strict">严格</MenuItem>
                                                        </Select>
                                                    </InputAdornment>
                                                ),
                                            }}
                                        />
                                    </Grid>
                                    <Grid size={{ xs: 12, md: 6 }}>
                                        <DatePicker
                                            label="起始日期"
                                            value={filters.startDate}
                                            onChange={(date) => setFilters(f => ({ ...f, startDate: date }))}
                                            sx={{ width: '100%' }}
                                        />
                                    </Grid>
                                    <Grid size={{ xs: 12, md: 6 }}>
                                        <DatePicker
                                            label="结束日期"
                                            value={filters.endDate}
                                            onChange={(date) => setFilters(f => ({ ...f, endDate: date }))}
                                            sx={{ width: '100%' }}
                                        />
                                    </Grid>
                                    <Grid size={{ xs: 12 }} sx={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-start' }}>
                                        <FormControlLabel
                                            control={
                                                <Checkbox
                                                    checked={filters.isFavorited}
                                                    onChange={(e) => setFilters(f => ({ ...f, isFavorited: e.target.checked }))}
                                                />
                                            }
                                            label="仅看收藏"
                                        />
                                        <FormControlLabel
                                            control={
                                                <Checkbox
                                                    checked={filters.reverseOrder}
                                                    onChange={(e) => setFilters(f => ({ ...f, reverseOrder: e.target.checked }))}
                                                />
                                            }
                                            label="结果逆序"
                                        />
                                    </Grid>
                                </Grid>
                                <Stack direction="row" spacing={2} sx={{ mt: 2 }}>
                                    <Button variant="contained" onClick={handleSearch}>查询</Button>
                                    <Button variant="outlined" onClick={handleClearFilters}>清空筛选</Button>
                                </Stack>
                            </Box>

                            <Divider />

                            {/* --- Batch Actions Section --- */}
                            <Box>
                                <Typography variant="subtitle1" gutterBottom sx={{ fontWeight: 'bold' }}>
                                    对筛选结果进行操作
                                </Typography>
                                <Stack direction="row" spacing={2}>
                                    <Button
                                        variant="contained"
                                        color="secondary"
                                        onClick={handleExport}
                                        disabled={isTaskRunning}
                                    >
                                        {isTaskRunning ? '任务进行中...' : '导出为 HTML'}
                                    </Button>
                                    <Button
                                        variant="contained"
                                        color="primary"
                                        onClick={handleRebackup}
                                        disabled={isTaskRunning}
                                    >
                                        {isTaskRunning ? '任务进行中...' : '重新备份筛选结果'}
                                    </Button>
                                </Stack>
                            </Box>
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
                            {postInfos.length > 0 ? (
                                <Box sx={{
                                    columnCount: { xs: 1, sm: 2, md: 3 },
                                    columnGap: '24px', // From spacing={3}
                                }}>
                                    {postInfos.map(postInfo => (
                                        <Box key={postInfo.post.id} sx={{ breakInside: 'avoid-column', mb: 3 }}>
                                            <PostDisplay
                                                postInfo={postInfo}
                                                onImageClick={handleOpenLightbox}
                                                maxAttachedImages={3}
                                                onClick={handlePostClick}
                                                maxLines={3}
                                                onPostDeleted={handlePostDeleted}
                                            />
                                        </Box>
                                    ))}
                                </Box>
                            ) : (
                                <Typography sx={{ textAlign: 'center', p: 5 }}>没有找到符合条件的帖子。</Typography>
                            )}

                            {totalPages > 0 && (
                                <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', mt: 3, flexWrap: 'wrap', gap: 2 }}>
                                    <Pagination
                                        count={totalPages}
                                        page={page}
                                        onChange={handlePageChange}
                                        color="primary"
                                    />
                                    <Stack direction="row" spacing={1} alignItems="center">
                                        <TextField
                                            size="small"
                                            label="跳至"
                                            value={jumpPage}
                                            onChange={(e) => setJumpPage(e.target.value)}
                                            onKeyDown={(e) => {
                                                if (e.key === 'Enter') {
                                                    handleJump();
                                                }
                                            }}
                                            sx={{ width: '80px' }}
                                        />
                                        <Button variant="outlined" size="small" onClick={handleJump} sx={{ height: '40px' }}>
                                            跳转
                                        </Button>
                                    </Stack>
                                </Box>
                            )}
                        </>
                    )}
                </Box>

                <Modal
                    open={!!lightboxImageId}
                    onClose={handleCloseLightbox}
                    aria-labelledby="lightbox-image"
                    sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                >
                    <Box sx={{ outline: 'none' }}>
                        {lightboxImageId && <FullSizeImage imageId={lightboxImageId} onClose={handleCloseLightbox} />}
                    </Box>
                </Modal>

                <PostPreviewModal
                    postInfo={hoveredPostInfo}
                    open={showPreviewModal}
                    onClose={handleClosePreviewModal}
                    onImageClick={handleOpenLightbox}
                />

            </Box>
        </LocalizationProvider>
    );
};

export default ContentExplorerPage;
