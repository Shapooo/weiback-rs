import React, { useState, useEffect } from 'react';
import { useSnackbar } from 'notistack';
import {
    Avatar,
    Box,
    Typography,
    Card,
    CardContent,
    CardHeader,
    IconButton,
    Stack,
    CircularProgress,
    Tooltip,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
    Button,
    Link,
} from '@mui/material';
import OpenInNewIcon from '@mui/icons-material/OpenInNew';
import DeleteIcon from '@mui/icons-material/Delete';
import SyncIcon from '@mui/icons-material/Sync';
import BrokenImageIcon from '@mui/icons-material/BrokenImage';
import LinkIcon from '@mui/icons-material/Link';
import { openUrl } from '@tauri-apps/plugin-opener';
import { avatarCache, attachedCache } from '../cache';
import Emoji from './Emoji';
import { PostInfo, UrlStructItem } from '../types';
import { getPictureBlob, deletePost, rebackupPost } from '../lib/api';

// --- Type Definitions are now in ../types.ts ---

interface AvatarImageProps {
    avatarId: string | null;
}

const AvatarImage: React.FC<AvatarImageProps> = ({ avatarId }) => {
    const [imageUrl, setImageUrl] = useState<string>('');

    useEffect(() => {
        let isCancelled = false;

        const fetchAndCacheAvatar = async () => {
            if (!avatarId) {
                setImageUrl('');
                return;
            }

            const cachedUrl = avatarCache.get(avatarId);
            if (cachedUrl) {
                setImageUrl(cachedUrl);
                return;
            }

            try {
                const blob = await getPictureBlob(avatarId);
                if (!isCancelled) {
                    const imageBlob = new Blob([blob]);
                    const objectUrl = URL.createObjectURL(imageBlob);
                    avatarCache.set(avatarId, objectUrl);
                    setImageUrl(objectUrl);
                }
            } catch (error) {
                if (!isCancelled) {
                    // Any error (NotFound, Internal) results in showing the default Avatar placeholder
                    setImageUrl('');
                }
            }
        };

        fetchAndCacheAvatar();

        return () => {
            isCancelled = true;
        };
    }, [avatarId]);

    return <Avatar src={imageUrl} />;
};

const THUMBNAIL_SIZE = 70; // Define a consistent size for thumbnails

interface AttachedImageProps {
    imageId: string;
    size: number;
    onClick: (id: string) => void;
}

const AttachedImage: React.FC<AttachedImageProps> = ({ imageId, size, onClick }) => {
    type Status = 'loading' | 'loaded' | 'not-found' | 'error';
    const [status, setStatus] = useState<Status>('loading');
    const [imageUrl, setImageUrl] = useState<string>('');

    useEffect(() => {
        let isCancelled = false;

        const fetchAndCacheImage = async () => {
            setStatus('loading');
            const cachedUrl = attachedCache.get(imageId);
            if (cachedUrl) {
                setImageUrl(cachedUrl);
                setStatus('loaded');
                return;
            }

            try {
                const blob = await getPictureBlob(imageId);
                if (isCancelled) return;

                const imageBlob = new Blob([blob]);
                const objectUrl = URL.createObjectURL(imageBlob);
                attachedCache.set(imageId, objectUrl);
                setImageUrl(objectUrl);
                setStatus('loaded');
            } catch (error) {
                if (isCancelled) return;

                if (error && typeof error === 'object' && 'kind' in error && (error as any).kind === 'NotFound') {
                    setStatus('not-found');
                } else {
                    console.error(`Failed to fetch attached image ${imageId}: ${error}`);
                    setStatus('error');
                }
            }
        };

        fetchAndCacheImage();

        return () => {
            isCancelled = true;
        };
    }, [imageId]);

    const commonSx = {
        width: size,
        height: size,
        borderRadius: 1,
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
    };

    if (status === 'loading') {
        return (
            <Box sx={commonSx}>
                <CircularProgress size={20} />
            </Box>
        );
    }

    if (status === 'not-found' || status === 'error') {
        return (
            <Box sx={{ ...commonSx, bgcolor: 'grey.200' }}>
                <BrokenImageIcon color="action" />
            </Box>
        );
    }

    // status === 'loaded'
    return (
        <Box
            onClick={(e) => {
                e.stopPropagation();
                onClick(imageId);
            }}
            sx={{
                ...commonSx,
                backgroundImage: `url(${imageUrl})`,
                backgroundSize: 'cover',
                backgroundPosition: 'center',
                cursor: 'pointer',
            }}
        />
    );
};

interface AttachedImagesProps {
    attachedIds: string[];
    onImageClick: (id: string) => void;
    maxImages?: number; // New prop to control the number of displayed images
}

const AttachedImages: React.FC<AttachedImagesProps> = ({ attachedIds, onImageClick, maxImages }) => {
    if (!attachedIds || attachedIds.length === 0) {
        return null;
    }

    const displayedImages = maxImages !== undefined ? attachedIds.slice(0, maxImages) : attachedIds;
    const remainingCount = attachedIds.length - displayedImages.length;

    return (
        <Stack direction="row" spacing={1} sx={{ mt: 1, flexWrap: 'wrap' }}>
            {displayedImages.map(id => (
                <AttachedImage key={id} imageId={id} size={THUMBNAIL_SIZE} onClick={onImageClick} />
            ))}
            {remainingCount > 0 && (
                <Box
                    sx={{
                        width: THUMBNAIL_SIZE,
                        height: THUMBNAIL_SIZE,
                        backgroundColor: 'rgba(0, 0, 0, 0.5)',
                        borderRadius: 1,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        flexShrink: 0,
                    }}
                >
                    <Typography variant="body2" color="white">
                        +{remainingCount}
                    </Typography>
                </Box>
            )}
        </Stack>
    );
};

interface ProcessedTextProps {
    text: string;
    emoji_map: Record<string, string>;
    url_struct: UrlStructItem[] | null;
    maxLines?: number;
}

const URL_REGEX = /https?:\/\/[a-zA-Z0-9$%&~_#./\-:=,?]{5,280}/g;
const AT_REGEX = /@[\u4e00-\u9fa5\uE7C7-\uE7F3\w_\-·]+/gu;
const TOPIC_REGEX = /#[^#]+#/g;
const EMAIL_REGEX = /[A-Za-z0-9]+([_.][A-Za-z0-9]+)*@([A-Za-z0-9-]+\.)+[A-Za-z]{2,6}/g;
const EMOJI_REGEX = /\[.*?\]/g;

const COMBINED_REGEX = new RegExp(`(${URL_REGEX.source})|(${AT_REGEX.source})|(${TOPIC_REGEX.source})|(${EMOJI_REGEX.source})`, 'gu');


const ProcessedText: React.FC<ProcessedTextProps> = ({ text, emoji_map, url_struct, maxLines }) => {
    const inPreviewMode = maxLines !== undefined;
    const processedText = inPreviewMode ? text.replace(/\n/g, ' ') : text;

    const urlMap = new Map<string, UrlStructItem>();
    if (url_struct) {
        for (const item of url_struct) {
            urlMap.set(item.short_url, item);
        }
    }

    const emailSuffixes = new Set<string>();
    for (const match of processedText.matchAll(EMAIL_REGEX)) {
        const email = match[0];
        const atMatch = email.match(AT_REGEX);
        if (atMatch) {
            emailSuffixes.add(atMatch[0]);
        }
    }

    const nodes: React.ReactNode[] = [];
    let lastIndex = 0;

    for (const match of processedText.matchAll(COMBINED_REGEX)) {
        const fullMatch = match[0];
        const url = match[1];
        const at = match[2];
        const topic = match[3];
        const emoji = match[4];
        const matchIndex = match.index!;

        if (matchIndex > lastIndex) {
            nodes.push(processedText.substring(lastIndex, matchIndex));
        }

        if (url) {
            const urlData = urlMap.get(url);
            if (urlData) {
                const longUrl = urlData.long_url || url;
                const title = urlData.url_title || '网页链接';
                nodes.push(
                    <Link key={lastIndex} href={longUrl} target="_blank" rel="noopener noreferrer">
                        <LinkIcon sx={{ marginRight: '0.2em', verticalAlign: 'middle', width: '16px', height: '16px' }} />
                        {title}
                    </Link>
                );
            } else {
                nodes.push(<Link key={lastIndex} href={url} target="_blank" rel="noopener noreferrer">{url}</Link>);
            }
        } else if (at) {
            if (emailSuffixes.has(at)) {
                nodes.push(at);
            } else {
                const username = at.substring(1);
                nodes.push(<Link key={lastIndex} href={`https://weibo.com/n/${username}`} target="_blank" rel="noopener noreferrer">{at}</Link>);
            }
        } else if (topic) {
            nodes.push(<Link key={lastIndex} href={`https://s.weibo.com/weibo?q=${encodeURIComponent(topic)}`} target="_blank" rel="noopener noreferrer">{topic}</Link>);
        } else if (emoji) {
            const imageId = emoji_map[emoji];
            if (imageId) {
                nodes.push(<Emoji key={lastIndex} imageId={imageId} emojiText={emoji} />);
            } else {
                nodes.push(emoji);
            }
        }

        lastIndex = matchIndex + fullMatch.length;
    }

    if (lastIndex < processedText.length) {
        nodes.push(processedText.substring(lastIndex));
    }

    const content = nodes.map((node, index) => <React.Fragment key={index}>{node}</React.Fragment>);


    const previewStyles = inPreviewMode ? {
        display: '-webkit-box',
        WebkitLineClamp: maxLines,
        WebkitBoxOrient: 'vertical',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
    } : {};

    return (
        <Typography
            variant="body2"
            component="div"
            sx={{
                ...previewStyles,
                wordBreak: 'break-word',
                lineHeight: '1.8',
                ...(inPreviewMode ? {} : { whiteSpace: 'pre-wrap' }),
            }}
        >
            {content}
        </Typography>
    );
};

interface PostDisplayProps {
    postInfo: PostInfo;
    onImageClick: (id: string) => void;
    maxAttachedImages?: number; // Prop to limit displayed attached images
    onClick?: (postInfo: PostInfo) => void;
    maxLines?: number; // New prop for text truncation
    onPostDeleted?: () => void;
}

const PostDisplay: React.FC<PostDisplayProps> = ({ postInfo, onImageClick, maxAttachedImages, onClick, maxLines, onPostDeleted }) => {
    const { enqueueSnackbar } = useSnackbar();
    const [dialogOpen, setDialogOpen] = useState(false);

    const handleDeleteClick = (e: React.MouseEvent) => {
        e.stopPropagation();
        setDialogOpen(true);
    };

    const handleDialogClose = () => {
        setDialogOpen(false);
    };

    const handleDeleteConfirm = async (e: React.MouseEvent) => {
        e.stopPropagation();
        try {
            await deletePost(postInfo.post.id.toString());
            enqueueSnackbar('帖子已删除', { variant: 'success' });
            onPostDeleted?.();
        } catch (error) {
            console.error('Failed to delete post:', error);
            enqueueSnackbar(`删除失败: ${error}`, { variant: 'error' });
        } finally {
            setDialogOpen(false);
        }
    };

    const handleRebackupClick = async (e: React.MouseEvent) => {
        e.stopPropagation();
        try {
            await rebackupPost(postInfo.post.id.toString());
            enqueueSnackbar('已加入重新备份队列', { variant: 'info' });
        } catch (error) {
            console.error('Failed to re-backup post:', error);
            enqueueSnackbar(`重新备份失败: ${error}`, { variant: 'error' });
        }
    };

    return (
        <>
            <Card
                onClick={() => onClick?.(postInfo)}
                sx={{ cursor: onClick ? 'pointer' : 'default' }}
            >
                <CardHeader
                    avatar={
                        <AvatarImage avatarId={postInfo.avatar_id} />
                    }
                    title={postInfo.post.user?.screen_name || '未知用户'}
                    subheader={new Date(postInfo.post.created_at).toLocaleString()}
                    action={
                        <Stack direction="row" alignItems="center">
                            {postInfo.post.id ? (
                                <Tooltip title="重新备份" enterDelay={500} arrow>
                                    <IconButton
                                        aria-label="re-backup post"
                                        onClick={handleRebackupClick}
                                    >
                                        <SyncIcon />
                                    </IconButton>
                                </Tooltip>
                            ) : null}
                            {postInfo.post.user?.id && postInfo.post.id ? (
                                <Tooltip
                                    title={`https://weibo.com/${postInfo.post.user.id}/${postInfo.post.id}`}
                                    enterDelay={500}
                                    arrow
                                >
                                    <IconButton
                                        aria-label="open original post"
                                        onClick={(e) => {
                                            e.stopPropagation();
                                            const url = `https://weibo.com/${postInfo.post.user!.id}/${postInfo.post.id}`;
                                            openUrl(url).catch(e => console.error('Failed to open URL:', e));
                                        }}
                                    >
                                        <OpenInNewIcon />
                                    </IconButton>
                                </Tooltip>
                            ) : null}
                            {postInfo.post.id ? (
                                <Tooltip title="删除" enterDelay={500} arrow>
                                    <IconButton
                                        aria-label="delete post"
                                        onClick={handleDeleteClick}
                                    >
                                        <DeleteIcon />
                                    </IconButton>
                                </Tooltip>
                            ) : null}
                        </Stack>
                    } />
                <CardContent>
                    <ProcessedText text={postInfo.post.text} emoji_map={postInfo.emoji_map} url_struct={postInfo.post.url_struct} maxLines={maxLines} />
                    {postInfo.post.retweeted_status && (
                        <Box sx={{ mt: 2, p: 2, backgroundColor: 'grey.100', borderRadius: 1 }}>
                            <Typography variant="subtitle2" color="text.secondary">
                                @{postInfo.post.retweeted_status.user?.screen_name || '未知用户'}
                            </Typography>
                            <ProcessedText text={postInfo.post.retweeted_status.text} emoji_map={postInfo.emoji_map} url_struct={postInfo.post.retweeted_status.url_struct} maxLines={maxLines} />
                        </Box>
                    )}
                    <AttachedImages attachedIds={postInfo.standalone_ids} onImageClick={onImageClick} maxImages={maxAttachedImages} />
                </CardContent>
            </Card>
            <Dialog
                open={dialogOpen}
                onClose={handleDialogClose}
                onClick={(e) => e.stopPropagation()}
                aria-labelledby="alert-dialog-title"
                aria-describedby="alert-dialog-description"
            >
                <DialogTitle id="alert-dialog-title">
                    {"确认删除"}
                </DialogTitle>
                <DialogContent>
                    <DialogContentText id="alert-dialog-description">
                        删除会删除该post及该post的所有转发以及图片等资源，且无法撤回。你确定要删除吗？
                    </DialogContentText>
                </DialogContent>
                <DialogActions>
                    <Button onClick={handleDialogClose}>取消</Button>
                    <Button onClick={handleDeleteConfirm} color="error" autoFocus>
                        确认
                    </Button>
                </DialogActions>
            </Dialog>
        </>
    );
};

export default PostDisplay;
