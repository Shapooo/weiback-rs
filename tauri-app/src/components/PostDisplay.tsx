import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
} from '@mui/material';
import OpenInNewIcon from '@mui/icons-material/OpenInNew';
import DeleteIcon from '@mui/icons-material/Delete';
import BrokenImageIcon from '@mui/icons-material/BrokenImage';
import { openUrl } from '@tauri-apps/plugin-opener';
import { avatarCache, attachmentCache } from '../cache';
import Emoji from './Emoji';

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
    retweeted_status?: Post | null;
}

export interface PostInfo {
    post: Post;
    avatar_id: string | null;
    emoji_map: Record<string, string>;
    attachment_ids: string[];
}

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
                const blob: ArrayBuffer = await invoke('get_picture_blob', { id: avatarId });
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

interface AttachmentImageProps {
    imageId: string;
    size: number;
    onClick: (id: string) => void;
}

const AttachmentImage: React.FC<AttachmentImageProps> = ({ imageId, size, onClick }) => {
    type Status = 'loading' | 'loaded' | 'not-found' | 'error';
    const [status, setStatus] = useState<Status>('loading');
    const [imageUrl, setImageUrl] = useState<string>('');

    useEffect(() => {
        let isCancelled = false;

        const fetchAndCacheImage = async () => {
            setStatus('loading');
            const cachedUrl = attachmentCache.get(imageId);
            if (cachedUrl) {
                setImageUrl(cachedUrl);
                setStatus('loaded');
                return;
            }

            try {
                const blob: ArrayBuffer = await invoke('get_picture_blob', { id: imageId });
                if (isCancelled) return;

                const imageBlob = new Blob([blob]);
                const objectUrl = URL.createObjectURL(imageBlob);
                attachmentCache.set(imageId, objectUrl);
                setImageUrl(objectUrl);
                setStatus('loaded');
            } catch (error) {
                if (isCancelled) return;

                if (error && typeof error === 'object' && 'kind' in error && (error as any).kind === 'NotFound') {
                    setStatus('not-found');
                } else {
                    console.error('Failed to fetch attachment image:', error);
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

interface AttachmentImagesProps {
    attachmentIds: string[];
    onImageClick: (id: string) => void;
    maxImages?: number; // New prop to control the number of displayed images
}

const AttachmentImages: React.FC<AttachmentImagesProps> = ({ attachmentIds, onImageClick, maxImages }) => {
    if (!attachmentIds || attachmentIds.length === 0) {
        return null;
    }

    const displayedImages = maxImages !== undefined ? attachmentIds.slice(0, maxImages) : attachmentIds;
    const remainingCount = attachmentIds.length - displayedImages.length;

    return (
        <Stack direction="row" spacing={1} sx={{ mt: 1, flexWrap: 'wrap' }}>
            {displayedImages.map(id => (
                <AttachmentImage key={id} imageId={id} size={THUMBNAIL_SIZE} onClick={onImageClick} />
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

interface TextWithEmojisProps {
    text: string;
    emoji_map: Record<string, string>;
    maxLines?: number;
}

const TextWithEmojis: React.FC<TextWithEmojisProps> = ({ text, emoji_map, maxLines }) => {
    // If no emoji map is provided, just return the plain text
    if (!emoji_map) {
        return <>{text}</>;
    }

    const inPreviewMode = maxLines !== undefined;
    const processedText = inPreviewMode ? text.replace(/\n/g, ' ') : text;

    // Regex to find emoji text like [like] or [哈哈]
    const emojiRegex = /\[.*?\]/g;
    const parts = processedText.split(emojiRegex);
    const matches = processedText.match(emojiRegex) || [];

    const content = parts.reduce<React.ReactNode[]>((acc, part, i) => {
        if (part) {
            acc.push(part);
        }
        if (matches[i]) {
            const emojiKey = matches[i];
            const imageId = emoji_map[emojiKey];
            if (imageId) {
                acc.push(<Emoji key={`${imageId}-${i}`} imageId={imageId} emojiText={emojiKey} />);
            }
            // If imageId is not found, the emojiKey is ignored and not added to acc
        }
        return acc;
    }, []);

    const previewStyles = inPreviewMode ? {
        display: '-webkit-box',
        WebkitLineClamp: maxLines,
        WebkitBoxOrient: 'vertical',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
    } : {};

    return (
        <Typography variant="body2" component="div" sx={{ ...previewStyles, wordBreak: 'break-word', lineHeight: '1.8' }}>
            {content}
        </Typography>
    );
};

interface PostDisplayProps {
    postInfo: PostInfo;
    onImageClick: (id: string) => void;
    maxAttachmentImages?: number; // Prop to limit displayed attachments
    onClick?: (postInfo: PostInfo) => void;
    maxLines?: number; // New prop for text truncation
    onPostDeleted?: () => void;
}

const PostDisplay: React.FC<PostDisplayProps> = ({ postInfo, onImageClick, maxAttachmentImages, onClick, maxLines, onPostDeleted }) => {
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
            await invoke('delete_post', { id: postInfo.post.id.toString() });
            enqueueSnackbar('帖子已删除', { variant: 'success' });
            onPostDeleted?.();
        } catch (error) {
            console.error('Failed to delete post:', error);
            enqueueSnackbar(`删除失败: ${error}`, { variant: 'error' });
        } finally {
            setDialogOpen(false);
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
                    <TextWithEmojis text={postInfo.post.text} emoji_map={postInfo.emoji_map} maxLines={maxLines} />
                    {postInfo.post.retweeted_status && (
                        <Box sx={{ mt: 2, p: 2, backgroundColor: 'grey.100', borderRadius: 1 }}>
                            <Typography variant="subtitle2" color="text.secondary">
                                @{postInfo.post.retweeted_status.user?.screen_name || '未知用户'}
                            </Typography>
                            <TextWithEmojis text={postInfo.post.retweeted_status.text} emoji_map={postInfo.emoji_map} maxLines={maxLines} />
                        </Box>
                    )}
                    <AttachmentImages attachmentIds={postInfo.attachment_ids} onImageClick={onImageClick} maxImages={maxAttachmentImages} />
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
