import React, { useState, useEffect, useRef } from 'react';
import { useSnackbar } from 'notistack';
import {
  Card, CardContent, Typography, TextField, Button, Box, Stack, Grid, Table, TableBody, TableRow, TableCell, Paper, TableContainer, CircularProgress, IconButton
} from '@mui/material';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import { useAuthStore } from '../stores/authStore';
import { getSmsCode, login as apiLogin } from '../lib/api';

enum UserPageState {
  Phone,
  Code,
  LoggedIn,
}

const UserPage: React.FC = () => {
  const { enqueueSnackbar } = useSnackbar();
  const { userInfo, isLoggedIn, isAuthLoading, login, logout } = useAuthStore();
  const [phone, setPhone] = useState('');
  const [verificationCode, setVerificationCode] = useState(Array(6).fill(''));
  const inputRefs = useRef<(HTMLInputElement | null)[]>([]);

  const [pageState, setPageState] = useState<UserPageState>(
    isLoggedIn ? UserPageState.LoggedIn : UserPageState.Phone
  );

  useEffect(() => {
    // Synchronize local page state with global auth state
    if (isLoggedIn) {
      setPageState(UserPageState.LoggedIn);
    } else if (pageState === UserPageState.LoggedIn) {
      // Only reset to Phone if we were previously in LoggedIn state
      setPageState(UserPageState.Phone);
      setPhone('');
      setVerificationCode(Array(6).fill(''));
    }
  }, [isLoggedIn]);

  const handleGetCode = async () => {
    if (!/^1\d{10}$/.test(phone)) {
      enqueueSnackbar('请输入有效的手机号码', { variant: 'error' });
      return;
    }
    try {
      await getSmsCode(phone);
      setPageState(UserPageState.Code);
      enqueueSnackbar(`验证码已发送至 ${phone}`, { variant: 'success' });
      setTimeout(() => inputRefs.current[0]?.focus(), 0);
    } catch (e) {
      enqueueSnackbar(`验证码请求失败: ${e}`, { variant: 'error' });
    }
  };

  const handleLogin = async () => {
    const code = verificationCode.join('');
    if (code.length !== 6 || !/^\d{6}$/.test(code)) {
      enqueueSnackbar('请输入完整的6位验证码', { variant: 'error' });
      return;
    }
    try {
      const res = await apiLogin(code);
      if (res) {
        login(res);
        enqueueSnackbar('登录成功！', { variant: 'success' });
      } else {
        enqueueSnackbar('登录失败，未获取到用户信息', { variant: 'error' });
      }
    } catch (e) {
      enqueueSnackbar(`登录失败: ${e}`, { variant: 'error' });
    }
  };

  const handleLogout = () => {
    logout();
    enqueueSnackbar('已退出登录', { variant: 'info' });
  };

  const handleCodeInputChange = (index: number, value: string) => {
    const newCode = [...verificationCode];
    newCode[index] = value;
    setVerificationCode(newCode);
    if (value && index < 5) {
      inputRefs.current[index + 1]?.focus();
    }
  };

  const handleKeyDown = (index: number, e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Backspace' && !verificationCode[index] && index > 0) {
      inputRefs.current[index - 1]?.focus();
    }
  };

  const renderContent = () => {
    if (isAuthLoading) {
      return (
        <Box sx={{ display: 'flex', justifyContent: 'center', p: 4 }}>
          <CircularProgress />
        </Box>
      );
    }

    switch (pageState) {
      case UserPageState.LoggedIn:
        return userInfo ? (
          <Box>
            <TableContainer component={Paper}>
              <Table>
                <TableBody>
                  <TableRow><TableCell>UID</TableCell><TableCell>{userInfo.id}</TableCell></TableRow>
                  <TableRow><TableCell>用户名</TableCell><TableCell>{userInfo.screen_name}</TableCell></TableRow>
                </TableBody>
              </Table>
            </TableContainer>
            <Button variant="outlined" color="error" onClick={handleLogout} sx={{ width: '100%', mt: 2 }}>
              退出登录
            </Button>
          </Box>
        ) : null;

      case UserPageState.Phone:
        return (
          <Stack spacing={2}>
            <TextField
              fullWidth
              label="手机号"
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
            />
            <Button variant="contained" onClick={handleGetCode} fullWidth>
              获取验证码
            </Button>
          </Stack>
        );

      case UserPageState.Code:
        return (
          <Stack spacing={2}>
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
              <IconButton
                size="small"
                onClick={() => setPageState(UserPageState.Phone)}
                sx={{ mr: 1 }}
                title="返回修改手机号"
              >
                <ArrowBackIcon fontSize="small" />
              </IconButton>
              <Typography variant="body2" color="text.secondary">
                验证码已发送至 {phone}
              </Typography>
            </Box>
            <Grid container spacing={1} justifyContent="center">
              {verificationCode.map((digit, index) => (
                <Grid size={{ xs: 2 }} key={index}>
                  <TextField
                    inputRef={(el) => (inputRefs.current[index] = el)}
                    value={digit}
                    onChange={(e) => handleCodeInputChange(index, e.target.value)}
                    onKeyDown={(e) => handleKeyDown(index, e)}
                    slotProps={{ htmlInput: { maxLength: 1, style: { textAlign: 'center' } } }}
                  />
                </Grid>
              ))}
            </Grid>
            <Button variant="contained" color="success" onClick={handleLogin} fullWidth>
              登 录
            </Button>
          </Stack>
        );
    }
  };

  return (
    <Card sx={{ maxWidth: 400, mx: 'auto', mt: 5 }}>
      <CardContent>
        <Typography variant="h5" component="div" sx={{ mb: 2 }}>
          {pageState === UserPageState.LoggedIn ? '用户信息' : '用户登录'}
        </Typography>
        <Box component="form" noValidate autoComplete="off">
          {renderContent()}
        </Box>
      </CardContent>
    </Card>
  );
};

export default UserPage;