import { Link as RouterLink, useLocation } from 'react-router-dom';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import List from '@mui/material/List';
import CloudDownloadOutlined from '@mui/icons-material/CloudDownloadOutlined';
import ArchiveOutlined from '@mui/icons-material/ArchiveOutlined';
import SettingsOutlined from '@mui/icons-material/SettingsOutlined';
import PersonOutlined from '@mui/icons-material/PersonOutlined';

const menuItems = [
    { key: '/online-backup', label: '备 份', icon: <CloudDownloadOutlined />, path: '/online-backup' },
    { key: '/export', label: '导 出', icon: <ArchiveOutlined />, path: '/export' },
    { key: '/user', label: '用 户', icon: <PersonOutlined />, path: '/user' },
    { key: '/settings', label: '设 置', icon: <SettingsOutlined />, path: '/settings' },
];

export const MainListItems = () => {
    const { pathname } = useLocation();

    return (
        <List component="nav">
            {menuItems.map((item) => (
                <ListItemButton
                    key={item.key}
                    component={RouterLink}
                    to={item.path!}
                    selected={pathname.startsWith(item.path!)}
                >
                    <ListItemIcon>{item.icon}</ListItemIcon>
                    <ListItemText primary={item.label} />
                </ListItemButton>
            ))}
        </List>
    );
};
