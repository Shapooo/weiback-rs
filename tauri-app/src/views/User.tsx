import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSnackbar } from 'notistack';
import {
  Card, CardContent, Typography, TextField, Button, Box, Stack, Grid, Table, TableBody, TableRow, TableCell, Paper, TableContainer
} from '@mui/material';

enum LoginStatus {
  Init,
  WaitingCode,
  CodeSent,
  LoggedIn
}

interface UserInfo {
  id: number;
  screen_name: string;
}

const UserPage: React.FC = () => {
  const { enqueueSnackbar } = useSnackbar();
  const [loginStatus, setLoginStatus] = useState<LoginStatus>(LoginStatus.Init);
  const [userInfo, setUserInfo] = useState<UserInfo | null>(null);
  const [phone, setPhone] = useState('');
  const [verificationCode, setVerificationCode] = useState(Array(6).fill(''));
  const inputRefs = useRef<(HTMLInputElement | null)[]>([]);

  useEffect(() => {
    const checkLoginStatus = async () => {
      try {
        const user: UserInfo | null = await invoke('login_state');
        if (user) {
          setUserInfo(user);
          setLoginStatus(LoginStatus.LoggedIn);
        } else {
          setLoginStatus(LoginStatus.Init);
        }
      } catch (e) {
        enqueueSnackbar(`检查登录状态失败: ${e}`, { variant: 'error' });
        setLoginStatus(LoginStatus.Init);
      }
    };
    checkLoginStatus();
  }, [enqueueSnackbar]);

  const handleGetCode = async () => {
    if (!/^1\d{10}$/.test(phone)) {
      enqueueSnackbar('请输入有效的手机号码', { variant: 'error' });
      return;
    }
    try {
      await invoke('get_sms_code', { phone_number: phone });
      setLoginStatus(LoginStatus.CodeSent);
      enqueueSnackbar(`验证码已发送至 ${phone}`, { variant: 'success' });
      inputRefs.current[0]?.focus();
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
      const res: UserInfo | null = await invoke('login', { sms_code: code });
      if (res) {
        setUserInfo(res);
        setLoginStatus(LoginStatus.LoggedIn);
        enqueueSnackbar('登录成功！', { variant: 'success' });
      }
      setPhone('');
      setVerificationCode(Array(6).fill(''));
    } catch (e) {
      enqueueSnackbar(`登录失败: ${e}`, { variant: 'error' });
    }
  };

  const handleLogout = async () => {
    try {
      // await invoke('logout');
      setLoginStatus(LoginStatus.Init);
      setUserInfo(null);
      enqueueSnackbar('已退出登录', { variant: 'info' });
    } catch (e) {
      enqueueSnackbar(`退出登录失败: ${e}`, { variant: 'error' });
    }
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

  return (
    <Card sx={{ maxWidth: 400, mx: 'auto', mt: 5 }}>
      <CardContent>
        <Typography variant="h5" component="div" sx={{ mb: 2 }}>
          {loginStatus === LoginStatus.LoggedIn ? '用户信息' : '用户登录'}
        </Typography>

        {loginStatus === LoginStatus.LoggedIn && userInfo ? (
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
        ) : (
          <Box component="form" noValidate autoComplete="off">
            {loginStatus !== LoginStatus.CodeSent ? (
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
            ) : (
              <Stack spacing={2}>
                <Typography align="center">验证码已发送至 {phone}</Typography>
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
            )}
          </Box>
        )}
      </CardContent>
    </Card>
  );
};

export default UserPage;