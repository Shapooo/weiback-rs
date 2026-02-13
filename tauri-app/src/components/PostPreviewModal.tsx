import React, { useCallback, useEffect } from 'react';
import {
    Modal,
    Box,
} from '@mui/material';
import PostDisplay from './PostDisplay';
import { PostInfo } from '../types';

interface PostPreviewModalProps {
    postInfo: PostInfo | null;
    open: boolean;
    onClose: () => void;
    onImageClick: (id: string) => void;
}

const PostPreviewModal: React.FC<PostPreviewModalProps> = ({ postInfo, open, onClose, onImageClick }) => {

    const handleCloseInternal = useCallback(() => {
        onClose();
    }, [onClose]);

    // Handle ESC key to close
    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === 'Escape') {
                handleCloseInternal();
            }
        };
        window.addEventListener('keydown', handleKeyDown);
        return () => {
            window.removeEventListener('keydown', handleKeyDown);
        };
    }, [handleCloseInternal]);

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
                    onImageClick={onImageClick} // Use the onImageClick prop passed from the parent
                />
            </Box>
        </Modal>
    );
};

export default PostPreviewModal;
