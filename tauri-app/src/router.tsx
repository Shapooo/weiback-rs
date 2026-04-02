import React, { Suspense } from 'react'
import { Routes, Route, Navigate } from 'react-router-dom'
import { Box, CircularProgress } from '@mui/material'

// Layout
const OnlineBackup = React.lazy(() => import('./views/OnlineBackup'))

// Pages
const ContentExplorer = React.lazy(() => import('./views/ContentExplorer'))
const Settings = React.lazy(() => import('./views/Settings'))
const User = React.lazy(() => import('./views/User'))
const DataManage = React.lazy(() => import('./views/DataManage'))
const About = React.lazy(() => import('./views/About'))

const AppRouter: React.FC = () => (
  <Suspense
    fallback={
      <Box
        sx={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          height: '100vh',
        }}
      >
        <CircularProgress />
      </Box>
    }
  >
    <Routes>
      <Route path="/" element={<Navigate to="/online-backup" replace />} />
      <Route path="/online-backup" element={<OnlineBackup />} />
      <Route path="/explorer" element={<ContentExplorer />} />
      <Route path="/manage" element={<DataManage />} />
      <Route path="/settings" element={<Settings />} />
      <Route path="/user" element={<User />} />
      <Route path="/about" element={<About />} />
    </Routes>
  </Suspense>
)

export default AppRouter
