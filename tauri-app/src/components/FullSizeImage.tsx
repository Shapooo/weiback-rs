import React, { useState, useEffect, useRef } from 'react';
import { CircularProgress } from '@mui/material';
import { attachedCache } from '../cache';
import { getPictureBlob } from '../lib/api';

interface FullSizeImageProps {
    imageId: string;
    onClose: () => void;
}

const FullSizeImage: React.FC<FullSizeImageProps> = ({ imageId, onClose }) => {
    const [imageUrl, setImageUrl] = useState<string>('');
    const [loading, setLoading] = useState(true);
    const [transform, setTransform] = useState({
        scale: 1,
        originX: '50%',
        originY: '50%',
    });
    const [position, setPosition] = useState({ x: 0, y: 0 });
    const [isDragging, setIsDragging] = useState(false);
    const dragStartRef = useRef({ startX: 0, startY: 0, initialX: 0, initialY: 0 });
    const didDragRef = useRef(false);
    const imgRef = useRef<HTMLImageElement>(null);

    const handleWheel = (e: React.WheelEvent<HTMLImageElement>) => {
        e.preventDefault();

        const zoomFactor = 1.1;
        const newScale = e.deltaY < 0 ? transform.scale * zoomFactor : transform.scale / zoomFactor;
        const clampedScale = Math.min(Math.max(1, newScale), 10);

        if (clampedScale === 1) {
            // Reset position and origin when zoomed back to original size
            setPosition({ x: 0, y: 0 });
            setTransform({ scale: 1, originX: '50%', originY: '50%' });
        } else if (imgRef.current) {
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

    const handleMouseDown = (e: React.MouseEvent<HTMLImageElement>) => {
        didDragRef.current = false;
        if (transform.scale <= 1) return;
        e.preventDefault();
        setIsDragging(true);
        dragStartRef.current = {
            startX: e.clientX,
            startY: e.clientY,
            initialX: position.x,
            initialY: position.y,
        };
    };

    const handleMouseMove = (e: React.MouseEvent<HTMLImageElement>) => {
        if (!isDragging) return;
        didDragRef.current = true;
        e.preventDefault();
        const dx = e.clientX - dragStartRef.current.startX;
        const dy = e.clientY - dragStartRef.current.startY;
        setPosition({
            x: dragStartRef.current.initialX + dx,
            y: dragStartRef.current.initialY + dy,
        });
    };

    const handleMouseUpOrLeave = () => {
        setIsDragging(false);
    };

    const handleClick = (e: React.MouseEvent<HTMLImageElement>) => {
        if (didDragRef.current) {
            e.preventDefault();
            return;
        }
        onClose();
    };

    // Reset transform when image changes
    useEffect(() => {
        setTransform({ scale: 1, originX: '50%', originY: '50%' });
        setPosition({ x: 0, y: 0 });
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
                const blob = await getPictureBlob(imageId);
                if (!isCancelled && blob.byteLength > 0) {
                    const imageBlob = new Blob([blob]);
                    const objectUrl = URL.createObjectURL(imageBlob);
                    attachedCache.set(imageId, objectUrl);
                    setImageUrl(objectUrl);
                }
            } catch (error) {
                console.error(`Failed to fetch full-size image ${imageId}: ${error}`);
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
            onClick={handleClick}
            onWheel={handleWheel}
            onMouseDown={handleMouseDown}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUpOrLeave}
            onMouseLeave={handleMouseUpOrLeave}
            style={{
                maxHeight: '90vh',
                maxWidth: '90vw',
                borderRadius: '4px',
                transform: `translate(${position.x}px, ${position.y}px) scale(${transform.scale})`,
                transformOrigin: `${transform.originX} ${transform.originY}`,
                transition: isDragging ? 'none' : 'transform 0.1s ease-out',
                cursor: transform.scale > 1 ? (isDragging ? 'grabbing' : 'grab') : 'zoom-in',
            }}
        />
    );
};

export default FullSizeImage;
