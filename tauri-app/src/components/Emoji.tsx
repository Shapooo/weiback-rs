import React, { useState, useEffect } from 'react';
import { emojiCache } from '../cache';
import { Box } from '@mui/material';
import { getPictureBlob } from '../lib/api';

interface EmojiProps {
    imageId: string;
    emojiText: string; // Add emojiText prop for fallback
}

const Emoji: React.FC<EmojiProps> = ({ imageId, emojiText }) => {
    const [imageUrl, setImageUrl] = useState<string>('');
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        let isCancelled = false;
        setLoading(true); // Start loading state for each new imageId

        const fetchAndCacheEmoji = async () => {
            const cachedUrl = emojiCache.get(imageId);
            if (cachedUrl) {
                setImageUrl(cachedUrl);
                setLoading(false);
                return;
            }

            try {
                const blob = await getPictureBlob(imageId);
                if (!isCancelled && blob.byteLength > 0) {
                    const imageBlob = new Blob([blob]);
                    const objectUrl = URL.createObjectURL(imageBlob);
                    emojiCache.set(imageId, objectUrl);
                    setImageUrl(objectUrl);
                } else {
                    setImageUrl(''); // Ensure imageUrl is cleared if blob is empty/invalid
                }
            } catch (error) {
                console.error(`Failed to fetch emoji image ${imageId}: ${error}`);
                setImageUrl(''); // Clear imageUrl on error to trigger fallback
            } finally {
                if (!isCancelled) setLoading(false);
            }
        };

        fetchAndCacheEmoji();

        return () => {
            isCancelled = true;
        };
    }, [imageId]);

    if (loading) {
        return <Box component="span">{emojiText}</Box>; // Show text while loading
    }

    if (!imageUrl) {
        return <Box component="span">{emojiText}</Box>; // Fallback to text if image fails
    }

    return (
        <Box
            component="img"
            src={imageUrl}
            alt={emojiText} // Use emojiText as alt text
            sx={{
                width: '1.2em',
                height: '1.2em',
                verticalAlign: 'middle',
                display: 'inline-block',
            }}
        />
    );
};

export default Emoji;
