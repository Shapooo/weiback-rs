import React from 'react';
import { Link as RouterLink, useLocation } from 'react-router-dom';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import List from '@mui/material/List';
import Collapse from '@mui/material/Collapse';
import CloudDownloadOutlined from '@mui/icons-material/CloudDownloadOutlined';
import ArchiveOutlined from '@mui/icons-material/ArchiveOutlined';
import SettingsOutlined from '@mui/icons-material/SettingsOutlined';
import PersonOutlined from '@mui/icons-material/PersonOutlined';
import PeopleOutlined from '@mui/icons-material/PeopleOutlined';
import StarBorderOutlined from '@mui/icons-material/StarBorderOutlined';
import ExpandLess from '@mui/icons-material/ExpandLess';
import ExpandMore from '@mui/icons-material/ExpandMore';

const menuItems = [
  {
    key: 'online-backup',
    label: '备 份',
    icon: <CloudDownloadOutlined />,
    children: [
      { key: '/online-backup/user', label: '用 户', icon: <PeopleOutlined />, path: '/online-backup/user' },
      { key: '/online-backup/favorites', label: '收 藏', icon: <StarBorderOutlined />, path: '/online-backup/favorites' },
    ],
  },
  { key: '/export', label: '导 出', icon: <ArchiveOutlined />, path: '/export' },
  { key: '/user', label: '用 户', icon: <PersonOutlined />, path: '/user' },
  { key: '/settings', label: '设 置', icon: <SettingsOutlined />, path: '/settings' },
];

export const MainListItems = () => {
  const { pathname } = useLocation();
  const [open, setOpen] = React.useState(true);

  const handleClick = () => {
    setOpen(!open);
  };

  return (
    <List component="nav">
      {menuItems.map((item) => {
        if (item.children) {
          return (
            <React.Fragment key={item.key}>
              <ListItemButton onClick={handleClick}>
                <ListItemIcon>{item.icon}</ListItemIcon>
                <ListItemText primary={item.label} />
                {open ? <ExpandLess /> : <ExpandMore />}
              </ListItemButton>
              <Collapse in={open} timeout="auto" unmountOnExit>
                <List component="div" disablePadding>
                  {item.children.map((child) => (
                    <ListItemButton
                      key={child.key}
                      component={RouterLink}
                      to={child.path}
                      selected={pathname === child.path}
                      sx={{ pl: 4 }}
                    >
                      <ListItemIcon>{child.icon}</ListItemIcon>
                      <ListItemText primary={child.label} />
                    </ListItemButton>
                  ))}
                </List>
              </Collapse>
            </React.Fragment>
          );
        }
        return (
          <ListItemButton
            key={item.key}
            component={RouterLink}
            to={item.path!}
            selected={pathname === item.path}
          >
            <ListItemIcon>{item.icon}</ListItemIcon>
            <ListItemText primary={item.label} />
          </ListItemButton>
        );
      })}
    </List>
  );
};
