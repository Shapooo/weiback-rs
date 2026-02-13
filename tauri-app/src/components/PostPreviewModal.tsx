import React, { useState, useEffect, useCallback } from 'react';
import {
    Modal,
    Box,
} from '@mui/material';
import PostDisplay from './PostDisplay';
import FullSizeImage from './FullSizeImage';
import { PostInfo } from '../types';

interface PostPreviewModalProps {
    postInfo: PostInfo | null;
    open: boolean;
    onClose: () => void;
    onImageClick: (id: string) => void;
}

const PostPreviewModal: React.FC<PostPreviewModalProps> = ({ postInfo, open, onClose, onImageClick }) => {
    const [lightboxImageId, setLightboxImageId] = useState<string | null>(null);

    const handleCloseInternal = useCallback(() => {
        setLightboxImageId(null);
        onClose();
    }, [onClose]);

    const handleLightboxOpen = useCallback((id: string) => {
        setLightboxImageId(id);
    }, []);

    const handleLightboxClose = useCallback(() => {
        setLightboxImageId(null);
    }, []);

    useEffect(() => {
        if (!open) {
            setLightboxImageId(null);
        }
    }, [open]);

    // Handle ESC key to close
    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === 'Escape') {
                if (lightboxImageId) {
                    handleLightboxClose();
                } else {
                    handleCloseInternal();
                }
            }
        };
        window.addEventListener('keydown', handleKeyDown);
        return () => {
            window.removeEventListener('keydown', handleKeyDown);
        };
    }, [handleCloseInternal, handleLightboxClose, lightboxImageId]);

    if (!postInfo) {
        return null;
    }

    return (
        <Modal
            open={open}
            onClose={handleCloseInternal}
            sx={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                backdropFilter: 'blur(2px)', // Gray overlay effect
                backgroundColor: 'rgba(0, 0, 0, 0.5)',
            }}
        >
            <Box
                sx={{
                    width: '90vw',
                    maxWidth: '600px', // Final max width
                    maxHeight: '90vh', // Final max height
                    overflowY: 'auto',
                    outline: 'none',
                    bgcolor: 'background.paper',
                    borderRadius: 2,
                    boxShadow: 24,
                    p: 2,
                }}
            >
                <PostDisplay
                    postInfo={postInfo}
                    onImageClick={handleLightboxOpen} // Internal lightbox for full images
                />

                <Modal
                    open={!!lightboxImageId}
                    onClose={handleLightboxClose}
                    aria-labelledby="lightbox-image"
                    sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                >
                    <Box onClick={handleLightboxClose} sx={{ outline: 'none' }}>
                        {lightboxImageId && <FullSizeImage imageId={lightboxImageId} />}
                    </Box>
                </Modal>
            </Box>
        </Modal>
    );
};

export default PostPreviewModal;
