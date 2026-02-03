import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
    Avatar,
    Box,
    Typography,
    Card,
    CardContent,
    CardHeader,
    IconButton,
    Stack,
} from '@mui/material';
import OpenInNewIcon from '@mui/icons-material/OpenInNew';
import { openUrl } from '@tauri-apps/plugin-opener';
import { avatarCache, attachmentCache } from '../cache';

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
    emoji_ids: Record<string, string>;
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
                if (!isCancelled && blob.byteLength > 0) {
                    const imageBlob = new Blob([blob]);
                    const objectUrl = URL.createObjectURL(imageBlob);
                    avatarCache.set(avatarId, objectUrl);
                    setImageUrl(objectUrl);
                } else {
                    setImageUrl(''); // Handle case where blob is empty
                }
            } catch (error) {
                console.error('Failed to fetch avatar:', error);
                setImageUrl(''); // Handle fetch error
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
    const [imageUrl, setImageUrl] = useState<string>('');

    useEffect(() => {
        let isCancelled = false;

        const fetchAndCacheImage = async () => {
            const cachedUrl = attachmentCache.get(imageId); // Use attachmentCache
            if (cachedUrl) {
                setImageUrl(cachedUrl);
                return;
            }

            try {
                const blob: ArrayBuffer = await invoke('get_picture_blob', { id: imageId });
                if (!isCancelled && blob.byteLength > 0) {
                    const imageBlob = new Blob([blob]);
                    const objectUrl = URL.createObjectURL(imageBlob);
                    attachmentCache.set(imageId, objectUrl); // Use attachmentCache
                    setImageUrl(objectUrl);
                } else {
                    setImageUrl('');
                }
            } catch (error) {
                console.error('Failed to fetch attachment image:', error);
                setImageUrl('');
            }
        };

        fetchAndCacheImage();

        return () => {
            isCancelled = true;
        };
    }, [imageId]);

    return (
        <Box
            onClick={(e) => {
                e.stopPropagation(); // Stop the click from bubbling up to the Card
                onClick(imageId);
            }}
            sx={{
                width: size,
                height: size,
                backgroundImage: `url(${imageUrl})`,
                backgroundSize: 'cover',
                backgroundPosition: 'center',
                borderRadius: 1,
                flexShrink: 0,
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

interface PostDisplayProps {
    postInfo: PostInfo;
    onImageClick: (id: string) => void;
    maxAttachmentImages?: number; // Prop to limit displayed attachments
    textLimit?: number; // Prop to limit the text length
    onClick?: (postInfo: PostInfo) => void;
}

const PostDisplay: React.FC<PostDisplayProps> = ({ postInfo, onImageClick, maxAttachmentImages, textLimit, onClick }) => {
    const postText = textLimit !== undefined && postInfo.post.text.length > textLimit
        ? postInfo.post.text.substring(0, textLimit) + '...'
        : postInfo.post.text;

    const retweetedText = postInfo.post.retweeted_status && textLimit !== undefined && postInfo.post.retweeted_status.text.length > textLimit
        ? postInfo.post.retweeted_status.text.substring(0, textLimit) + '...'
        : postInfo.post.retweeted_status?.text;

    return (
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
                    postInfo.post.user?.id && postInfo.post.id ? (
                        <IconButton
                            aria-label="open original post"
                            onClick={() => {
                                const url = `https://weibo.com/${postInfo.post.user!.id}/${postInfo.post.id}`;
                                openUrl(url).catch(e => console.error('Failed to open URL:', e));
                            }}
                        >
                            <OpenInNewIcon />
                        </IconButton>
                    ) : null
                }
            />
            <CardContent>
                <Typography variant="body2">
                    {postText}
                </Typography>
                {postInfo.post.retweeted_status && (
                    <Box sx={{ mt: 2, p: 2, backgroundColor: 'grey.100', borderRadius: 1 }}>
                        <Typography variant="subtitle2" color="text.secondary">
                            @{postInfo.post.retweeted_status.user?.screen_name || '未知用户'}
                        </Typography>
                        <Typography variant="body2" sx={{ mt: 1 }}>
                            {retweetedText}
                        </Typography>
                    </Box>
                )}
                <AttachmentImages attachmentIds={postInfo.attachment_ids} onImageClick={onImageClick} maxImages={maxAttachmentImages} />
            </CardContent>
        </Card>
    );
};

export default PostDisplay;
