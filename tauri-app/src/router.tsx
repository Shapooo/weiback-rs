import React, { Suspense } from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';
import { Box, CircularProgress } from '@mui/material';

// Layout
const OnlineBackup = React.lazy(() => import('./views/OnlineBackup'));

// Pages
const LocalExport = React.lazy(() => import('./views/LocalExport'));
const Settings = React.lazy(() => import('./views/Settings'));
const User = React.lazy(() => import('./views/User'));

const AppRouter: React.FC = () => (
  <Suspense fallback={
    <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh' }}>
      <CircularProgress />
    </Box>
  }>
    <Routes>
      <Route path="/" element={<Navigate to="/online-backup" replace />} />
      <Route path="/online-backup" element={<OnlineBackup />} />
      <Route path="/export" element={<LocalExport />} />
      <Route path="/settings" element={<Settings />} />
      <Route path="/user" element={<User />} />
    </Routes>
  </Suspense>
);

export default AppRouter;
