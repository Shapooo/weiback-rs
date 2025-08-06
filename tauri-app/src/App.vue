<script setup lang="ts">
import { h, ref, Component } from 'vue';
import { RouterLink, useRoute } from 'vue-router';
import { NIcon, MenuOption } from 'naive-ui';
import {
  CloudDownloadOutline,
  ArchiveOutline,
  SettingsOutline,
  PersonOutline,
  PeopleOutline,
  StarOutline,
} from '@vicons/ionicons5';
import { theme, themeOverrides } from './store/theme';

// Helper function to render icons
const renderIcon = (icon: Component) => {
  return () => h(NIcon, null, { default: () => h(icon) });
};

// Menu options definition
const menuOptions: MenuOption[] = [
  {
    label: '备 份',
    key: 'online-backup',
    icon: renderIcon(CloudDownloadOutline),
    children: [
      {
        label: () =>
          h(
            RouterLink,
            {
              to: {
                name: 'UserBackup',
              },
            },
            { default: () => '用 户' }
          ),
        key: '/online-backup/user',
        icon: renderIcon(PeopleOutline),
      },
      {
        label: () =>
          h(
            RouterLink,
            {
              to: {
                name: 'FavoritesBackup',
              },
            },
            { default: () => '收 藏' }
          ),
        key: '/online-backup/favorites',
        icon: renderIcon(StarOutline),
      },
    ],
  },
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            path: '/export',
          },
        },
        { default: () => '导 出' }
      ),
    key: '/export',
    icon: renderIcon(ArchiveOutline),
  },
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            path: '/settings',
          },
        },
        { default: () => '设 置' }
      ),
    key: '/settings',
    icon: renderIcon(SettingsOutline),
  },
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            path: '/user',
          },
        },
        { default: () => '用 户' }
      ),
    key: '/user',
    icon: renderIcon(PersonOutline),
  },
];

const route = useRoute();
const activeKey = ref(route.path);

</script>

<template>
  <n-config-provider :theme="theme" :theme-overrides="themeOverrides">
    <n-message-provider>
      <n-layout has-sider class="main-container">
        <n-layout-sider
          bordered
          collapse-mode="width"
          :collapsed-width="64"
          :width="200"
          :native-scrollbar="false"
        >
          <n-menu
            v-model:value="activeKey"
            :options="menuOptions"
            :collapsed-width="64"
            :collapsed-icon-size="22"
          />
        </n-layout-sider>
        <n-layout-content class="main-content">
          <router-view></router-view>
        </n-layout-content>
      </n-layout>
    </n-message-provider>
  </n-config-provider>
</template>

<style>
.main-container {
  height: 100vh;
}
.main-content {
  padding: 20px;
}
.n-menu .n-menu-item-content {
  border-radius: 6px;
  margin: 2px 4px;
}
</style>
