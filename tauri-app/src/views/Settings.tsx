import React from 'react';
import { useThemeContext } from '../ThemeContext';
import { Card, CardContent, Typography, FormControlLabel, Switch, Box } from '@mui/material';
import { useTheme } from '@mui/material/styles';

const SettingsPage: React.FC = () => {
  const { toggleColorMode } = useThemeContext();
  const theme = useTheme();

  return (
    <Card sx={{ maxWidth: 500, mx: 'auto', mt: 3 }}>
      <CardContent>
        <Typography variant="h5" component="div" sx={{ mb: 2 }}>
          设置
        </Typography>
        <Box>
          <FormControlLabel
            control={<Switch checked={theme.palette.mode === 'dark'} onChange={toggleColorMode} />}
            label="暗色模式"
          />
        </Box>
      </CardContent>
    </Card>
  );
};

export default SettingsPage;