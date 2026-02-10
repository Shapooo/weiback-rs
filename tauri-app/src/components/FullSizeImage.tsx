import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CircularProgress } from '@mui/material';
import { attachedCache } from '../cache';

const FullSizeImage: React.FC<{ imageId: string }> = ({ imageId }) => {
    const [imageUrl, setImageUrl] = useState<string>('');
    const [loading, setLoading] = useState(true);
    const [transform, setTransform] = useState({
        scale: 1,
        originX: '50%',
        originY: '50%',
    });
    const imgRef = useRef<HTMLImageElement>(null);

    const handleWheel = (e: React.WheelEvent<HTMLImageElement>) => {
        e.preventDefault();

        // Determine zoom direction
        const zoomFactor = 1.1;
        const newScale = e.deltaY < 0 ? transform.scale * zoomFactor : transform.scale / zoomFactor;

        // Clamp scale value
        const clampedScale = Math.min(Math.max(1, newScale), 10); // Min scale 1x, max 10x

        if (imgRef.current) {
            const rect = imgRef.current.getBoundingClientRect();
            const newOriginX = `${((e.clientX - rect.left) / rect.width) * 100}%`;
            const newOriginY = `${((e.clientY - rect.top) / rect.height) * 100}%`;

            setTransform({
                scale: clampedScale,
                originX: newOriginX,
                originY: newOriginY,
            });
        }
    };

    // Reset transform when image changes
    useEffect(() => {
        setTransform({ scale: 1, originX: '50%', originY: '50%' });
    }, [imageId]);


    useEffect(() => {
        let isCancelled = false;
        setLoading(true);

        const fetchAndCacheImage = async () => {
            const cachedUrl = attachedCache.get(imageId);
            if (cachedUrl) {
                setImageUrl(cachedUrl);
                setLoading(false);
                return;
            }
            try {
                const blob: ArrayBuffer = await invoke('get_picture_blob', { id: imageId });
                if (!isCancelled && blob.byteLength > 0) {
                    const imageBlob = new Blob([blob]);
                    const objectUrl = URL.createObjectURL(imageBlob);
                    attachedCache.set(imageId, objectUrl);
                    setImageUrl(objectUrl);
                }
            } catch (error) {
                console.error('Failed to fetch full-size image:', error);
            } finally {
                if (!isCancelled) setLoading(false);
            }
        };

        fetchAndCacheImage();
        return () => { isCancelled = true; };
    }, [imageId]);

    if (loading) {
        return <CircularProgress sx={{ color: 'white' }} />;
    }

    return (
        <img
            ref={imgRef}
            src={imageUrl}
            alt="Lightbox"
            onWheel={handleWheel}
            style={{
                maxHeight: '90vh',
                maxWidth: '90vw',
                borderRadius: '4px',
                transform: `scale(${transform.scale})`,
                transformOrigin: `${transform.originX} ${transform.originY}`,
                transition: 'transform 0.1s ease-out',
                cursor: 'zoom-in',
            }}
        />
    );
};

export default FullSizeImage;
