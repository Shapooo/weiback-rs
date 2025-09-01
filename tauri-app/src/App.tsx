import React from 'react';
// import { Outlet } from 'react-router-dom';
import { Box, Drawer, CssBaseline } from '@mui/material';
import { MainListItems } from './listItems';
import AppRouter from './router';

const drawerWidth = 200;

const App: React.FC = () => {
  return (
    <Box sx={{ display: 'flex' }}>
      <CssBaseline />
      <Drawer
        variant="permanent"
        sx={{
          width: drawerWidth,
          flexShrink: 0,
          [`& .MuiDrawer-paper`]: { width: drawerWidth, boxSizing: 'border-box' },
        }}
      >
        <Box sx={{ overflow: 'auto' }}>
          <MainListItems />
        </Box>
      </Drawer>
      <Box
        component="main"
        sx={{ flexGrow: 1, p: 3, width: `calc(100% - ${drawerWidth}px)` }}
      >
        <AppRouter />
      </Box>
    </Box>
  );
};

export default App;
