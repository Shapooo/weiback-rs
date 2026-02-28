import React from 'react';
import { Box, Typography } from '@mui/material';

const DataManage: React.FC = () => {
    return (
        <Box sx={{ p: 3 }}>
            <Typography variant="h4" gutterBottom>
                数据管理
            </Typography>
            <Typography variant="body1">
                此处将实现本地数据的批量管理功能，如图片清晰度清理等。
            </Typography>
        </Box>
    );
};

export default DataManage;
