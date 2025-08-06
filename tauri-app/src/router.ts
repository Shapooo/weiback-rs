import { createRouter, createWebHistory } from 'vue-router';
import OnlineBackup from './views/OnlineBackup.vue';
import LocalExport from './views/LocalExport.vue';
import Settings from './views/Settings.vue';
import User from './views/User.vue';

const routes = [
  { path: '/', redirect: '/online-backup' },
  {
    path: '/online-backup',
    component: OnlineBackup,
    redirect: '/online-backup/user',
    children: [
      {
        path: 'user',
        name: 'UserBackup',
        component: () => import('./views/UserBackup.vue'),
      },
      {
        path: 'favorites',
        name: 'FavoritesBackup',
        component: () => import('./views/FavoritesBackup.vue'),
      },
    ],
  },
  { path: '/export', component: LocalExport },
  { path: '/settings', component: Settings },
  { path: '/user', component: User },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
